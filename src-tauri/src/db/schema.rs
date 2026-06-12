// Cybermanju Drive — Database Schema Definitions
// All data models serialized as JSON strings stored in redb key-value tables
// Field names MUST match exactly what all command files expect.

use serde::{Deserialize, Serialize};

/// Global serde configuration: all schema structs serialize to camelCase for the frontend.
/// This ensures Rust `file_type` becomes JSON `"fileType"`, matching TypeScript interfaces.

// ---------------------------------------------------------------------------
// Core file node — the central entity
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileNode {
    pub id: String,
    pub name: String,
    pub file_type: String, // "file" | "folder"
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
    pub modified_at: String, // used by commands for mutation timestamps
}

// ---------------------------------------------------------------------------
// Account — storage origin (local / cloud / network)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub name: String,
    pub account_type: String, // "local" | "cloud" | "network"
    pub path: Option<String>,
    pub color: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

// ---------------------------------------------------------------------------
// Collection — curated group of files (highlights, best moments, custom)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub collection_type: String, // "highlights" | "best_moments" | "custom"
    pub color: String,
    pub description: Option<String>,
    pub item_ids: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ---------------------------------------------------------------------------
// CollectionItem — junction table linking a file to a collection
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionItem {
    pub id: String,
    pub collection_id: String,
    pub file_id: String,
    pub note: Option<String>,
    pub added_at: String,
}

// ---------------------------------------------------------------------------
// FaceGroup — cluster of similar faces (a "person")
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FaceGroup {
    pub id: String,
    pub name: String,
    pub file_ids: Vec<String>,
    pub centroid_embedding: Option<Vec<f32>>,
    /// 64-bit SimHash binary code for fast Hamming-distance pre-filtering.
    pub binary_hash: Option<u64>,
    /// Average intra-cluster cosine distance (lower = tighter cluster).
    pub cohesion: Option<f32>,
    /// Number of face embeddings stored for this group.
    pub embedding_count: u32,
    /// Clustering algorithm that produced this group.
    pub algorithm: Option<String>,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// EncryptionKey — PQC or classical encryption keypair
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncryptionKey {
    pub id: String,
    pub algorithm: String,
    pub public_key: String,
    pub private_key: String,
    pub label: Option<String>,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// LooseGroup — user-defined ad-hoc file groupings
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LooseGroup {
    pub id: String,
    pub name: String,
    pub color: String,
    pub file_ids: Vec<String>,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// User — application-level user with role-based access
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub display_name: Option<String>,
    pub role: String, // "admin" | "user" | "viewer"
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

// ---------------------------------------------------------------------------
// UserFilePermission — per-file access control
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserFilePermission {
    pub id: String,
    pub user_id: String,
    pub file_id: String,
    pub access: String, // "read" | "write" | "admin"
    pub granted_by: String,
    pub granted_at: String,
}
