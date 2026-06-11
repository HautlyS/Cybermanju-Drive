use tauri::State;

use crate::AppState;
use crate::sync::backends::create_backend;
use crate::sync::models::*;

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// List all saved sync configurations.
#[tauri::command]
pub fn list_sync_configs(state: State<'_, AppState>) -> Result<Vec<SyncConfig>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let tx = db.begin_read().map_err(|e| e.to_string())?;
    let table = tx
        .open_table(crate::db::Database::get_sync_configs_table())
        .map_err(|e| e.to_string())?;

    let mut configs = Vec::new();
    for entry in table.iter().map_err(|e| e.to_string())? {
        let (_, value) = entry.map_err(|e| e.to_string())?;
        let config: SyncConfig = serde_json::from_str(&value.value())
            .map_err(|e| e.to_string())?;
        configs.push(config);
    }

    Ok(configs)
}

/// Create (or save) a sync configuration.
#[tauri::command]
pub fn create_sync_config(
    config: SyncConfig,
    state: State<'_, AppState>,
) -> Result<SyncConfig, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Generate an ID if none provided
    let config_id = if config.id.is_empty() {
        uuid::Uuid::new_v4().to_string()
    } else {
        config.id.clone()
    };

    let mut config = config;
    config.id = config_id.clone();

    let serialized = serde_json::to_string(&config).map_err(|e| e.to_string())?;
    let tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = tx
            .open_table(crate::db::Database::get_sync_configs_table())
            .map_err(|e| e.to_string())?;
        table
            .insert(config_id.as_str(), serialized.as_str())
            .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;

    Ok(config)
}

/// Delete a sync configuration by ID.
#[tauri::command]
pub fn delete_sync_config(
    config_id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = tx
            .open_table(crate::db::Database::get_sync_configs_table())
            .map_err(|e| e.to_string())?;
        let removed = table
            .remove(config_id.as_str())
            .map_err(|e| e.to_string())?
            .is_some();
        if !removed {
            return Err(format!("Sync config not found: {}", config_id));
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(true)
}

/// Start a sync operation for the given config and file IDs.
#[tauri::command]
pub fn start_sync(
    config_id: String,
    file_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<SyncResult, String> {
    // 1. Load the sync config from DB
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let tx_read = db.begin_read().map_err(|e| e.to_string())?;
    let table = tx_read
        .open_table(crate::db::Database::get_sync_configs_table())
        .map_err(|e| e.to_string())?;
    let value = table
        .get(config_id.as_str())
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Sync config not found: {}", config_id))?;
    let config: SyncConfig = serde_json::from_str(&value.value())
        .map_err(|e| e.to_string())?;
    drop(tx_read);

    if !config.enabled {
        return Err(format!("Sync config '{}' is not enabled", config_id));
    }

    // 2. Create pipeline and run sync
    let pipeline = crate::sync::SyncPipeline::new(config, "cybermanju.db".to_string());
    let result = pipeline.sync_all(file_ids, &state)?;

    Ok(result)
}

/// Get the current sync progress.
#[tauri::command]
pub fn get_sync_progress(_state: State<'_, AppState>) -> Result<SyncProgress, String> {
    // Since we run syncs synchronously, if no active pipeline exists,
    // return an idle progress.
    Ok(SyncProgress {
        total_files: 0,
        processed_files: 0,
        current_file: None,
        status: SyncStatus::Idle,
        bytes_uploaded: 0,
        errors: Vec::new(),
        started_at: None,
        estimated_remaining_seconds: None,
    })
}

/// Test the connection for a sync configuration.
#[tauri::command]
pub fn test_sync_connection(config: SyncConfig) -> Result<bool, String> {
    let backend = create_backend(&config)?;
    backend.test_connection()
}

/// Cancel the current sync operation.
#[tauri::command]
pub fn cancel_sync(_state: State<'_, AppState>) -> Result<bool, String> {
    // Since we run syncs synchronously on the main thread, cancellation
    // is a no-op in the current architecture. If async is added later,
    // this would signal the active pipeline to stop.
    Ok(true)
}

/// List files on the remote backend.
#[tauri::command]
pub fn list_remote_files(
    config: SyncConfig,
    prefix: String,
) -> Result<Vec<RemoteFile>, String> {
    let backend = create_backend(&config)?;
    backend.list_files(&prefix)
}