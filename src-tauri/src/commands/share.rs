use redb::ReadableTable;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::db::schema::ShareLink;
use crate::AppState;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareLinkResult {
    pub id: String,
    pub file_id: String,
    pub token: String,
    pub expires_at: String,
    pub url: String,
}

#[tauri::command]
pub fn generate_share_link(
    file_id: String,
    expires_in_hours: Option<u64>,
    state: State<'_, AppState>,
) -> Result<ShareLinkResult, String> {
    let db = state.db.write().map_err(|e| e.to_string())?;
    let link = db
        .create_share_link(&file_id, expires_in_hours.unwrap_or(0))
        .map_err(|e| e.to_string())?;
    Ok(ShareLinkResult {
        id: link.id,
        file_id: link.file_id,
        token: link.token.clone(),
        expires_at: link.expires_at,
        url: format!("/api/share/{}", link.token),
    })
}

#[tauri::command]
pub fn get_shared_file(
    token: String,
    state: State<'_, AppState>,
) -> Result<Option<crate::db::schema::FileNode>, String> {
    let db = state.db.read().map_err(|e| e.to_string())?;
    let link = db
        .get_share_link_by_token(&token)
        .map_err(|e| e.to_string())?;

    match link {
        Some(link) => {
            // Check expiry
            if let Ok(expires) = chrono::DateTime::parse_from_rfc3339(&link.expires_at) {
                if chrono::Utc::now() > expires {
                    return Err("Share link has expired".to_string());
                }
            }
            db.get_file_node(&link.file_id).map_err(|e| e.to_string())
        }
        None => Ok(None),
    }
}

#[tauri::command]
pub fn list_share_links(state: State<'_, AppState>) -> Result<Vec<ShareLink>, String> {
    let db = state.db.read().map_err(|e| e.to_string())?;
    let tx = db.begin_read().map_err(|e| e.to_string())?;
    let table = tx
        .open_table(crate::db::Database::get_share_links_table())
        .map_err(|e| e.to_string())?;
    let mut links = Vec::new();
    for entry in table.iter().map_err(|e| e.to_string())? {
        let (_, value) = entry.map_err(|e| e.to_string())?;
        if let Ok(link) = serde_json::from_str::<ShareLink>(value.value()) {
            links.push(link.with_url());
        }
    }
    Ok(links)
}
