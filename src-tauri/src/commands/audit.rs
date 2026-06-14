use crate::db::schema::AuditEntry;
use crate::AppState;
use tauri::State;

/// Fetch audit log entries with optional limit and entity filter.
#[tauri::command]
pub fn get_audit_log(
    limit: Option<u32>,
    entity_type: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<AuditEntry>, String> {
    let db = state.db.read().map_err(|e| e.to_string())?;
    let tx = db.begin_read().map_err(|e| e.to_string())?;
    let table = tx
        .open_table(crate::db::Database::get_audit_log_table())
        .map_err(|e| e.to_string())?;

    let mut entries: Vec<AuditEntry> = table
        .iter()
        .map_err(|e| e.to_string())?
        .filter_map(|entry| {
            let (_, value) = entry.ok()?;
            serde_json::from_str::<AuditEntry>(value.value()).ok()
        })
        .filter(|e| {
            if let Some(ref et) = entity_type {
                e.entity_type == *et
            } else {
                true
            }
        })
        .collect();

    // Sort by timestamp descending (most recent first)
    entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    let limit = limit.unwrap_or(100) as usize;
    entries.truncate(limit);
    Ok(entries)
}
