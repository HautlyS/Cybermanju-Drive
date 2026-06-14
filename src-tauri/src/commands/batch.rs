use chrono::Utc;
use redb::ReadableTable;
use tauri::State;
use crate::db::schema::FileNode;
use crate::AppState;

/// Batch delete: move multiple files to trash in a single operation.
#[tauri::command]
pub fn batch_delete(
    file_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<u32, String> {
    let db = state.db.write().map_err(|e| e.to_string())?;
    let mut count = 0u32;

    for file_id in &file_ids {
        let tx_read = db.begin_read().map_err(|e| e.to_string())?;
        let table = tx_read
            .open_table(crate::db::Database::get_files_table())
            .map_err(|e| e.to_string())?;
        let file_node: Option<FileNode> = table
            .get(file_id.as_str())
            .map_err(|e| e.to_string())?
            .and_then(|v| serde_json::from_str(v.value()).ok());
        drop(tx_read);

        if let Some(node) = file_node {
            db.log_audit("batch_delete", "file", file_id, None, None)
                .map_err(|e| e.to_string())?;
            let parent_id = node.parent_id.clone();
            db.trash_file(file_id, &node, None)
                .map_err(|e| e.to_string())?;
            if let Some(pid) = parent_id {
                db.remove_from_parent_index(file_id, &pid)
                    .map_err(|e| e.to_string())?;
            }
            count += 1;
        }
    }

    Ok(count)
}

/// Batch encrypt: encrypt multiple files with a single algorithm.
#[tauri::command]
pub fn batch_encrypt(
    file_ids: Vec<String>,
    algorithm: String,
    state: State<'_, AppState>,
) -> Result<u32, String> {
    let db = state.db.write().map_err(|e| e.to_string())?;
    let mut count = 0u32;

    for file_id in &file_ids {
        let tx_read = db.begin_read().map_err(|e| e.to_string())?;
        let table = tx_read
            .open_table(crate::db::Database::get_files_table())
            .map_err(|e| e.to_string())?;
        let mut file_node: FileNode = table
            .get(file_id.as_str())
            .map_err(|e| e.to_string())?
            .and_then(|v| serde_json::from_str(v.value()).ok())
            .ok_or_else(|| format!("File not found: {}", file_id))?;
        drop(tx_read);

        if !file_node.encrypted {
            file_node.encrypted = true;
            file_node.encryption_algorithm = Some(algorithm.clone());
            file_node.modified_at = Utc::now().to_rfc3339();

            let serialized = serde_json::to_string(&file_node).map_err(|e| e.to_string())?;
            let tx_write = db.begin_write().map_err(|e| e.to_string())?;
            {
                let mut files_table = tx_write
                    .open_table(crate::db::Database::get_files_table())
                    .map_err(|e| e.to_string())?;
                files_table
                    .insert(file_id.as_str(), serialized.as_str())
                    .map_err(|e| e.to_string())?;
            }
            tx_write.commit().map_err(|e| e.to_string())?;
            db.log_audit("batch_encrypt", "file", file_id, None, None)
                .map_err(|e| e.to_string())?;
            count += 1;
        }
    }

    Ok(count)
}

/// Batch compress: compress multiple files with a single layer.
#[tauri::command]
pub fn batch_compress(
    file_ids: Vec<String>,
    layer: String,
    state: State<'_, AppState>,
) -> Result<u32, String> {
    let db = state.db.write().map_err(|e| e.to_string())?;
    let mut count = 0u32;

    for file_id in &file_ids {
        let tx_read = db.begin_read().map_err(|e| e.to_string())?;
        let table = tx_read
            .open_table(crate::db::Database::get_files_table())
            .map_err(|e| e.to_string())?;
        let mut file_node: FileNode = table
            .get(file_id.as_str())
            .map_err(|e| e.to_string())?
            .and_then(|v| serde_json::from_str(v.value()).ok())
            .ok_or_else(|| format!("File not found: {}", file_id))?;
        drop(tx_read);

        let layers: Vec<String> = file_node.compression_layers.clone();
        if !layers.contains(&layer) {
            file_node.compression_layers.push(layer.clone());
            file_node.modified_at = Utc::now().to_rfc3339();

            let serialized = serde_json::to_string(&file_node).map_err(|e| e.to_string())?;
            let tx_write = db.begin_write().map_err(|e| e.to_string())?;
            {
                let mut files_table = tx_write
                    .open_table(crate::db::Database::get_files_table())
                    .map_err(|e| e.to_string())?;
                files_table
                    .insert(file_id.as_str(), serialized.as_str())
                    .map_err(|e| e.to_string())?;
            }
            tx_write.commit().map_err(|e| e.to_string())?;
            db.log_audit("batch_compress", "file", file_id, None, None)
                .map_err(|e| e.to_string())?;
            count += 1;
        }
    }

    Ok(count)
}
