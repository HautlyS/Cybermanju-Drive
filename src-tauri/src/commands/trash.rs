use crate::db::schema::TrashItem;
use crate::AppState;
use tauri::State;

/// List all items currently in the trash.
#[tauri::command]
pub fn list_trash(state: State<'_, AppState>) -> Result<Vec<TrashItem>, String> {
    let db = state.db.read().map_err(|e| e.to_string())?;
    db.list_trash().map_err(|e| e.to_string())
}

/// Restore a file or folder from trash to its original location.
#[tauri::command]
pub fn restore_from_trash(
    file_id: String,
    state: State<'_, AppState>,
) -> Result<TrashItem, String> {
    let db = state.db.write().map_err(|e| e.to_string())?;
    db.restore_from_trash(&file_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Item not found in trash: {}", file_id))
}

/// Permanently delete all items from the trash.
#[tauri::command]
pub fn empty_trash(state: State<'_, AppState>) -> Result<u32, String> {
    let db = state.db.write().map_err(|e| e.to_string())?;
    db.empty_trash().map_err(|e| e.to_string())
}

/// Permanently delete a single item from the trash (no restore).
#[tauri::command]
pub fn delete_from_trash(file_id: String, state: State<'_, AppState>) -> Result<bool, String> {
    let db = state.db.write().map_err(|e| e.to_string())?;
    let tx = db.begin_write().map_err(|e| e.to_string())?;
    let removed = {
        let mut table = tx
            .open_table(crate::db::Database::get_trash_table())
            .map_err(|e| e.to_string())?;
        let result = table.remove(file_id.as_str()).map_err(|e| e.to_string())?;
        result.is_some()
    };
    tx.commit().map_err(|e| e.to_string())?;
    Ok(removed)
}
