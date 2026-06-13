use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FileNode {
    pub id: String,
    pub name: String,
    pub file_type: String,
    pub parent_id: Option<String>,
    pub size_bytes: u64,
    pub mime_type: Option<String>,
    pub hash_blake3: Option<String>,
    pub encrypted: bool,
    pub encryption_algorithm: Option<String>,
    pub compression_layers: Vec<String>,
    pub thumbnail_path: Option<String>,
    pub context_data: Option<serde_json::Value>,
    pub tags: Vec<String>,
    pub collection_ids: Vec<String>,
    pub face_group_ids: Vec<String>,
    pub loose_group_ids: Vec<String>,
    pub gps_lat: Option<f64>,
    pub gps_lon: Option<f64>,
    pub created_at: String,
    pub modified_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub name: String,
    pub account_type: String,
    pub path: Option<String>,
    pub color: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub collection_type: String,
    pub color: String,
    pub description: Option<String>,
    pub item_ids: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CollectionItem {
    pub id: String,
    pub collection_id: String,
    pub file_id: String,
    pub note: Option<String>,
    pub added_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FaceGroup {
    pub id: String,
    pub name: String,
    pub file_ids: Vec<String>,
    pub centroid_embedding: Option<Vec<f32>>,
    pub binary_hash: Option<u64>,
    pub cohesion: Option<f32>,
    pub embedding_count: u32,
    pub algorithm: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EncryptionKey {
    pub id: String,
    pub algorithm: String,
    pub public_key: String,
    pub private_key: String,
    pub label: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LooseGroup {
    pub id: String,
    pub name: String,
    pub color: String,
    pub file_ids: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub display_name: Option<String>,
    pub role: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UserFilePermission {
    pub id: String,
    pub user_id: String,
    pub file_id: String,
    pub access: String,
    pub granted_by: String,
    pub granted_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub id: String,
    pub file_id: String,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<f64>,
    pub place_name: Option<String>,
    pub created_at: String,
}
