// Cybermanju Drive — Share links (shared by Tauri IPC and REST)

use cybermanju_db::Database;
use cybermanju_types::schema::{FileNode, ShareLink};
use redb::ReadableTable;
use serde::{Deserialize, Serialize};

/// Share link payload returned to the frontend.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareLinkResult {
    pub id: String,
    pub file_id: String,
    pub token: String,
    pub expires_at: String,
    pub url: String,
}

/// Create a share link for a file.
pub fn generate(
    db: &Database,
    file_id: &str,
    expires_in_hours: Option<u64>,
) -> Result<ShareLinkResult, String> {
    let link = db
        .create_share_link(file_id, expires_in_hours.unwrap_or(0))
        .map_err(|e| e.to_string())?;
    Ok(ShareLinkResult {
        id: link.id,
        file_id: link.file_id,
        token: link.token.clone(),
        expires_at: link.expires_at,
        url: format!("/api/shared/{}", link.token),
    })
}

/// Resolve a share token to its file, enforcing expiry.
pub fn resolve(db: &Database, token: &str) -> Result<Option<FileNode>, String> {
    let link = db
        .get_share_link_by_token(token)
        .map_err(|e| e.to_string())?;

    match link {
        Some(link) => {
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

/// List every share link, with its public URL filled in.
pub fn list(db: &Database) -> Result<Vec<ShareLink>, String> {
    let tx = db.begin_read().map_err(|e| e.to_string())?;
    let table = tx
        .open_table(Database::get_share_links_table())
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
