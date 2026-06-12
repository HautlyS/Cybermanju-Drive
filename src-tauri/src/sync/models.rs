// Cybermanju Drive — Storage Sync Models
// Shared types for the sync backend system

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Type of storage backend
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum SyncBackendType {
    Local,
    GitHub,
    GitLab,
    GoogleDrive,
    GooglePhotos,
}

/// Current status of a sync operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum SyncStatus {
    Idle,
    Scanning,
    Compressing,
    Uploading,
    Linking,
    Cleaning,
    Error,
    Done,
    Syncing,
    Completed,
    Cancelled,
}

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// Metadata for a single file being synced
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncFile {
    pub id: String,
    pub original_path: String,
    pub compressed_path: Option<String>,
    pub preview_path: Option<String>,
    pub remote_url: Option<String>,
    pub size_bytes: u64,
    pub compressed_size_bytes: Option<u64>,
    pub hash_blake3: Option<String>,
    pub backend_type: SyncBackendType,
    pub synced_at: Option<String>,
    pub status: SyncStatus,
    pub error_message: Option<String>,
}

/// Configuration for a sync backend instance
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncConfig {
    pub id: String,
    pub backend_type: SyncBackendType,
    pub enabled: bool,
    /// Local: directory path, GitLab: instance URL (e.g., https://gitlab.com)
    pub base_path: Option<String>,
    /// GitHub: owner/repo, GitLab: project ID or namespace/project path
    pub repo_name: Option<String>,
    pub branch: Option<String>,
    /// GitHub: PAT, GitLab: PAT/OAuth, Google: OAuth2 Bearer token
    pub token: Option<String>,
    /// Google Drive: folder ID
    pub folder_id: Option<String>,
    /// Google Photos: album ID
    pub album_id: Option<String>,
    pub auto_sync: bool,
    pub compress_before_upload: bool,
    pub create_previews: bool,
    pub delete_raw_after_sync: bool,
    pub max_concurrent_uploads: u32,
}

/// Live progress of a sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncProgress {
    pub total_files: u32,
    pub processed_files: u32,
    pub current_file: Option<String>,
    pub status: SyncStatus,
    pub bytes_uploaded: u64,
    pub errors: Vec<String>,
    pub started_at: Option<String>,
    pub estimated_remaining_seconds: Option<f64>,
}

/// Final result of a completed sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    pub files_synced: u32,
    pub bytes_uploaded: u64,
    pub bytes_saved_by_compression: u64,
    pub errors: Vec<String>,
    pub duration_ms: u64,
}

/// A file on a remote storage backend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteFile {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified_at: String,
    pub url: String,
}

// ---------------------------------------------------------------------------
// StorageBackend trait
// ---------------------------------------------------------------------------

/// Trait that all storage backends must implement.
pub trait StorageBackend: Send + Sync {
    /// Human-readable backend name
    fn name(&self) -> &str;

    /// Which backend type this is
    fn backend_type(&self) -> SyncBackendType;

    /// Upload a file from `local_path` to `remote_path` on the backend.
    /// Returns the URL of the uploaded file.
    fn upload_file(&self, local_path: &str, remote_path: &str) -> Result<String, String>;

    /// Download a file from `remote_path` on the backend to `local_path`.
    fn download_file(&self, remote_path: &str, local_path: &str) -> Result<(), String>;

    /// Delete a file at `remote_path` on the backend.
    fn delete_file(&self, remote_path: &str) -> Result<(), String>;

    /// List files under `prefix` on the backend.
    fn list_files(&self, prefix: &str) -> Result<Vec<RemoteFile>, String>;

    /// Get a public/shared URL for the file at `remote_path`.
    fn get_file_url(&self, remote_path: &str) -> Result<String, String>;

    /// Test that the backend connection is valid (auth works, etc.)
    fn test_connection(&self) -> Result<bool, String>;
}