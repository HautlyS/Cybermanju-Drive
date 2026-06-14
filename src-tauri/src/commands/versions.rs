use chrono::Utc;
use redb::ReadableTable;
use tauri::State;
use crate::db::schema::{FileNode, FileVersion};
use crate::AppState;

/// List all versions of a specific file.
#[tauri::command]
pub fn list_file_versions(
    file_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<FileVersion>, String> {
    let db = state.db.read().map_err(|e| e.to_string())?;
    db.list_file_versions(&file_id).map_err(|e| e.to_string())
}

/// Create a new version snapshot for a file.
#[tauri::command]
pub fn create_file_version(
    file_id: String,
    state: State<'_, AppState>,
) -> Result<FileVersion, String> {
    let db = state.db.write().map_err(|e| e.to_string())?;

    let tx_read = db.begin_read().map_err(|e| e.to_string())?;
    let table = tx_read
        .open_table(crate::db::Database::get_files_table())
        .map_err(|e| e.to_string())?;
    let file_node: FileNode = table
        .get(file_id.as_str())
        .map_err(|e| e.to_string())?
        .and_then(|v| serde_json::from_str(v.value()).ok())
        .ok_or_else(|| format!("File not found: {}", file_id))?;
    drop(tx_read);

    let version = db
        .create_file_version(&file_node, None)
        .map_err(|e| e.to_string())?;

    db.log_audit(
        "create_version",
        "file_version",
        &version.id,
        None,
        None,
    )
    .map_err(|e| e.to_string())?;

    Ok(version)
}

/// Revert a file to a previous version.
#[tauri::command]
pub fn revert_file_version(
    file_id: String,
    version_id: String,
    state: State<'_, AppState>,
) -> Result<FileVersion, String> {
    let db = state.db.write().map_err(|e| e.to_string())?;

    let version = db
        .revert_file_version(&file_id, &version_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Version not found: {}", version_id))?;

    db.log_audit(
        "revert_version",
        "file_version",
        &version_id,
        None,
        None,
    )
    .map_err(|e| e.to_string())?;

    Ok(version)
}

/// Create a version snapshot for every file in the database.
#[tauri::command]
pub fn snapshot_all_versions(state: State<'_, AppState>) -> Result<u32, String> {
    let db = state.db.write().map_err(|e| e.to_string())?;
    let mut count = 0u32;

    let tx_read = db.begin_read().map_err(|e| e.to_string())?;
    let table = tx_read
        .open_table(crate::db::Database::get_files_table())
        .map_err(|e| e.to_string())?;
    let file_nodes: Vec<FileNode> = table
        .iter()
        .map_err(|e| e.to_string())?
        .filter_map(|entry| {
            let (_, value) = entry.ok()?;
            serde_json::from_str::<FileNode>(value.value()).ok()
        })
        .collect();
    drop(tx_read);

    for node in &file_nodes {
        if node.file_type == "file" {
            db.create_file_version(node, None)
                .map_err(|e| e.to_string())?;
            count += 1;
        }
    }

    Ok(count)
}
