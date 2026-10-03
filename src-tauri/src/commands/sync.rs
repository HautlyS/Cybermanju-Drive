use std::sync::Arc;
use tauri::State;

pub use cybermanju_sync::SyncState;

use crate::sync::models::*;
use crate::AppState;

// ---------------------------------------------------------------------------
// Tauri commands (thin wrappers over the shared sync API)
// ---------------------------------------------------------------------------

/// List all saved sync configurations.
#[tauri::command]
pub fn list_sync_configs(state: State<'_, AppState>) -> Result<Vec<SyncConfig>, String> {
    let db = state.db.read().map_err(|e| e.to_string())?;
    cybermanju_web::api::sync_api::list_configs(&db)
}

/// Create (or save) a sync configuration.
#[tauri::command]
pub fn create_sync_config(
    config: SyncConfig,
    state: State<'_, AppState>,
) -> Result<SyncConfig, String> {
    let db = state.db.write().map_err(|e| e.to_string())?;
    cybermanju_web::api::sync_api::save_config(&db, config)
}

/// Delete a sync configuration by ID.
#[tauri::command]
pub fn delete_sync_config(config_id: String, state: State<'_, AppState>) -> Result<bool, String> {
    let db = state.db.write().map_err(|e| e.to_string())?;
    cybermanju_web::api::sync_api::delete_config(&db, &config_id)
}

/// Start a sync operation for the given config and file IDs.
///
/// The shared implementation releases the database lock before the pipeline
/// runs: the pipeline acquires its own locks per file, so holding one here
/// would deadlock.
#[tauri::command]
pub fn start_sync(
    config_id: String,
    file_ids: Vec<String>,
    state: State<'_, AppState>,
    sync_state: State<'_, Arc<SyncState>>,
) -> Result<SyncResult, String> {
    cybermanju_web::api::sync_api::start(
        &state.db,
        &state.compression,
        &sync_state,
        &config_id,
        file_ids,
    )
}

/// Get the current sync progress.
#[tauri::command]
pub fn get_sync_progress(sync_state: State<'_, Arc<SyncState>>) -> Result<SyncProgress, String> {
    Ok(cybermanju_web::api::sync_api::progress(&sync_state))
}

/// Test the connection for a sync configuration.
#[tauri::command]
pub fn test_sync_connection(config: SyncConfig) -> Result<bool, String> {
    cybermanju_web::api::sync_api::test_connection(&config)
}

/// Cancel the current sync operation.
#[tauri::command]
pub fn cancel_sync(sync_state: State<'_, Arc<SyncState>>) -> Result<bool, String> {
    Ok(cybermanju_web::api::sync_api::cancel(&sync_state))
}

/// List files on the remote backend.
#[tauri::command]
pub fn list_remote_files(config: SyncConfig, prefix: String) -> Result<Vec<RemoteFile>, String> {
    cybermanju_web::api::sync_api::list_remote_files(&config, &prefix)
}
