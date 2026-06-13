use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum SyncBackendType {
    Local,
    GitHub,
    GitLab,
    GoogleDrive,
    GooglePhotos,
    Telegram,
}

impl std::fmt::Display for SyncBackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local => write!(f, "local"),
            Self::GitHub => write!(f, "github"),
            Self::GitLab => write!(f, "gitlab"),
            Self::GoogleDrive => write!(f, "googleDrive"),
            Self::GooglePhotos => write!(f, "googlePhotos"),
            Self::Telegram => write!(f, "telegram"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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

impl std::fmt::Display for SyncStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "idle"),
            Self::Scanning => write!(f, "scanning"),
            Self::Compressing => write!(f, "compressing"),
            Self::Uploading => write!(f, "uploading"),
            Self::Linking => write!(f, "linking"),
            Self::Cleaning => write!(f, "cleaning"),
            Self::Error => write!(f, "error"),
            Self::Done => write!(f, "done"),
            Self::Syncing => write!(f, "syncing"),
            Self::Completed => write!(f, "completed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudAccount {
    pub id: String,
    pub name: String,
    pub backend_type: SyncBackendType,
    pub token: Option<String>,
    pub oauth_credentials: Option<OAuthCredentials>,
    pub config: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthCredentials {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<u64>,
    pub client_id: String,
    pub client_secret: Option<String>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncConfig {
    pub id: String,
    pub backend_type: SyncBackendType,
    pub enabled: bool,
    pub account_id: Option<String>,
    pub name: Option<String>,
    pub base_path: Option<String>,
    pub repo_name: Option<String>,
    pub branch: Option<String>,
    pub token: Option<String>,
    pub folder_id: Option<String>,
    pub album_id: Option<String>,
    pub chat_id: Option<String>,
    pub auto_sync: bool,
    pub compress_before_upload: bool,
    pub create_previews: bool,
    pub delete_raw_after_sync: bool,
    pub max_concurrent_uploads: u32,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    pub files_synced: u32,
    pub bytes_uploaded: u64,
    pub bytes_saved_by_compression: u64,
    pub errors: Vec<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteFile {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified_at: String,
    pub url: String,
}

pub trait StorageBackend: Send + Sync {
    fn name(&self) -> &str;
    fn backend_type(&self) -> SyncBackendType;
    fn upload_file(&self, local_path: &str, remote_path: &str) -> Result<String, String>;
    fn download_file(&self, remote_path: &str, local_path: &str) -> Result<(), String>;
    fn delete_file(&self, remote_path: &str) -> Result<(), String>;
    fn list_files(&self, prefix: &str) -> Result<Vec<RemoteFile>, String>;
    fn get_file_url(&self, remote_path: &str) -> Result<String, String>;
    fn test_connection(&self) -> Result<bool, String>;
}
