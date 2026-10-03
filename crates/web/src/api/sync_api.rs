// Cybermanju Drive — Sync configuration + run lifecycle (shared by Tauri IPC and REST)

use std::sync::{Arc, RwLock};

use cybermanju_compression::TripleCompressor;
use cybermanju_db::Database;
use cybermanju_sync::{create_backend, SyncPipeline, SyncState, SyncStatus};
use cybermanju_types::sync::{RemoteFile, SyncConfig, SyncProgress, SyncResult};
use redb::ReadableTable;
use serde::Deserialize;

// ─── Request wire types ──────────────────────────────────────────────

/// `POST /api/sync/start` body.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartRequest {
    pub config_id: String,
    pub file_ids: Vec<String>,
}

/// Body carrying a full `SyncConfig` (create / test / remote listing).
#[derive(Debug, Deserialize)]
pub struct ConfigRequest {
    pub config: SyncConfig,
}

/// `POST /api/sync/remote-files` body.
#[derive(Debug, Deserialize)]
pub struct RemoteFilesRequest {
    pub config: SyncConfig,
    #[serde(default)]
    pub prefix: String,
}

/// List all saved sync configurations.
pub fn list_configs(db: &Database) -> Result<Vec<SyncConfig>, String> {
    let tx = db.begin_read().map_err(|e| e.to_string())?;
    let table = tx
        .open_table(Database::get_sync_configs_table())
        .map_err(|e| e.to_string())?;

    let mut configs = Vec::new();
    for entry in table.iter().map_err(|e| e.to_string())? {
        let (_, value) = entry.map_err(|e| e.to_string())?;
        let config: SyncConfig = serde_json::from_str(value.value()).map_err(|e| e.to_string())?;
        configs.push(config);
    }

    Ok(configs)
}

/// Create (or overwrite) a sync configuration. Generates an ID when absent.
pub fn save_config(db: &Database, config: SyncConfig) -> Result<SyncConfig, String> {
    let config_id = if config.id.is_empty() {
        uuid::Uuid::new_v4().to_string()
    } else {
        config.id.clone()
    };

    let now = chrono::Utc::now().to_rfc3339();
    let mut config = config;
    config.id = config_id.clone();
    if config.created_at.is_none() {
        config.created_at = Some(now.clone());
    }
    config.updated_at = Some(now);

    let serialized = serde_json::to_string(&config).map_err(|e| e.to_string())?;
    let tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = tx
            .open_table(Database::get_sync_configs_table())
            .map_err(|e| e.to_string())?;
        table
            .insert(config_id.as_str(), serialized.as_str())
            .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;

    Ok(config)
}

/// Delete a sync configuration by ID.
pub fn delete_config(db: &Database, config_id: &str) -> Result<bool, String> {
    let tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = tx
            .open_table(Database::get_sync_configs_table())
            .map_err(|e| e.to_string())?;
        let removed = table
            .remove(config_id)
            .map_err(|e| e.to_string())?
            .is_some();
        if !removed {
            return Err(format!("Sync config not found: {}", config_id));
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(true)
}

/// Load a single sync configuration by ID.
pub fn get_config(db: &Database, config_id: &str) -> Result<SyncConfig, String> {
    let tx = db.begin_read().map_err(|e| e.to_string())?;
    let table = tx
        .open_table(Database::get_sync_configs_table())
        .map_err(|e| e.to_string())?;
    let value = table
        .get(config_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Sync config not found: {}", config_id))?;
    serde_json::from_str(value.value()).map_err(|e| e.to_string())
}

/// Test a configuration's remote connectivity.
pub fn test_connection(config: &SyncConfig) -> Result<bool, String> {
    let backend = create_backend(config)?;
    backend.test_connection()
}

/// List files stored on the remote backend for a configuration.
pub fn list_remote_files(config: &SyncConfig, prefix: &str) -> Result<Vec<RemoteFile>, String> {
    let backend = create_backend(config)?;
    backend.list_files(prefix)
}

/// Current sync progress snapshot (lockless).
pub fn progress(sync_state: &Arc<SyncState>) -> SyncProgress {
    sync_state.snapshot()
}

/// Request cancellation of the current sync (lockless).
pub fn cancel(sync_state: &Arc<SyncState>) -> bool {
    // Sets the flag the pipeline polls on every file, and flips the reported
    // status to `cancelled`.
    sync_state.cancel();
    true
}

/// Start a sync run for `config_id`.
///
/// The database lock is released before the pipeline runs: the pipeline
/// acquires its own lock per file, so holding one here would deadlock.
pub fn start(
    db: &RwLock<Database>,
    compression: &TripleCompressor,
    sync_state: &Arc<SyncState>,
    config_id: &str,
    file_ids: Vec<String>,
) -> Result<SyncResult, String> {
    // 1. Load the sync config from DB (lock released at end of this block)
    let config = {
        let db = db.read().map_err(|e| e.to_string())?;
        get_config(&db, config_id)?
    };

    if !config.enabled {
        return Err(format!("Sync config '{}' is not enabled", config_id));
    }

    // 2. Reset shared progress so pollers see a live run
    sync_state.reset(file_ids.len() as u32);
    sync_state.set_status(SyncStatus::Syncing);
    sync_state.set_current(Some("Starting sync...".to_string()));
    sync_state.set_started_at(Some(chrono::Utc::now().to_rfc3339()));

    // 3. Create pipeline sharing the same progress/cancellation state
    let pipeline = SyncPipeline::new(config, Arc::clone(sync_state));
    let result = match pipeline.sync_all(file_ids, db, compression) {
        Ok(r) => r,
        Err(e) => {
            // A failed run must not leave pollers stuck in `syncing`
            sync_state.set_status(SyncStatus::Error);
            sync_state.add_error(e.clone());
            return Err(e);
        }
    };

    // 4. Only report completion when the run was not cancelled
    if !sync_state.is_cancelled() {
        sync_state.set_status(SyncStatus::Completed);
        sync_state.set_current(None);
        let total = sync_state.snapshot().total_files;
        sync_state.set_processed(total);
    }

    Ok(result)
}
