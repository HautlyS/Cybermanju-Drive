// Cybermanju Drive — Storage Sync Backends
// Five backend implementations: Local, GitHub, GitLab, Google Drive, Google Photos
// All HTTP backends use reqwest::blocking (no curl subprocess).

use crate::sync::models::*;
use crate::sync::oauth::{self, OAuthCredentials};
use log::info;
use std::fs;
use std::path::Path;

// ===========================================================================
// Helper: safe path join (prevents path traversal)
// ===========================================================================

/// Safely join a base path with a remote path, ensuring the result stays
/// within the base directory. Rejects symlinks that resolve outside base.
fn safe_join(base: &str, remote: &str) -> Result<String, String> {
    let base = std::path::Path::new(base)
        .canonicalize()
        .map_err(|e| format!("Cannot canonicalize base path '{}': {}", base, e))?;
    let joined = base.join(remote);
    // canonicalize will fail if the target doesn't exist yet (e.g. during upload dest).
    // For existing files it validates the real location; for new files we validate
    // the parent directory.
    match joined.canonicalize() {
        Ok(canonical) => {
            if !canonical.starts_with(&base) {
                return Err("Path traversal detected".to_string());
            }
            // Also reject if the path itself is a symlink pointing outside
            if joined.is_symlink() {
                let link_target = std::fs::read_link(&joined)
                    .map_err(|e| format!("Cannot read symlink: {}", e))?;
                if !base.join(&link_target).starts_with(&base) {
                    return Err("Symlink target outside base path".to_string());
                }
            }
            Ok(canonical.to_string_lossy().to_string())
        }
        Err(_) => {
            // File doesn't exist yet — validate the parent
            if let Some(parent) = joined.parent() {
                if parent.as_os_str().is_empty() {
                    // parent is "/" or empty, just use base
                    return Ok(base.to_string_lossy().to_string());
                }
                let parent_canonical = parent
                    .canonicalize()
                    .map_err(|e| format!("Cannot canonicalize parent directory: {}", e))?;
                if !parent_canonical.starts_with(&base) {
                    return Err("Path traversal detected in parent directory".to_string());
                }
            }
            // Canonicalize the base and re-join to get a clean absolute path
            let clean = base.join(remote);
            let clean_str = clean.to_string_lossy().to_string();
            // Final safety: ensure no ".." component in the resolved result
            let resolved = std::path::Path::new(&clean_str);
            for component in resolved.components() {
                if let std::path::Component::ParentDir = component {
                    return Err("Path traversal detected: parent directory component".to_string());
                }
            }
            Ok(clean_str)
        }
    }
}

// ===========================================================================
// Helper: parse a GitHub repo name into (owner, repo)
// ===========================================================================

fn parse_repo(repo_name: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = repo_name.trim_start_matches('/').splitn(2, '/').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return Err(format!(
            "Invalid repo name '{}'. Expected 'owner/repo'.",
            repo_name
        ));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

// ===========================================================================
// Helper: build a shared reqwest::blocking::Client
// ===========================================================================

fn http_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .user_agent("CybermanjuDrive/0.1")
        .connect_timeout(std::time::Duration::from_secs(15))
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))
}

// ===========================================================================
// 1. LocalBackend
// ===========================================================================

pub struct LocalBackend {
    base_path: String,
}

impl LocalBackend {
    pub fn new(base_path: &str) -> Self {
        Self {
            base_path: base_path.to_string(),
        }
    }
}

impl StorageBackend for LocalBackend {
    fn name(&self) -> &str {
        "Local Storage"
    }

    fn backend_type(&self) -> SyncBackendType {
        SyncBackendType::Local
    }

    fn upload_file(&self, local_path: &str, remote_path: &str) -> Result<String, String> {
        let dest = safe_join(&self.base_path, remote_path)?;
        if let Some(parent) = Path::new(&dest).parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
        }
        fs::copy(local_path, &dest).map_err(|e| format!("Failed to copy file: {}", e))?;
        Ok(dest)
    }

    fn download_file(&self, remote_path: &str, local_path: &str) -> Result<(), String> {
        let src = safe_join(&self.base_path, remote_path)?;
        // Verify the source is not a symlink pointing outside
        if Path::new(&src).is_symlink() {
            let link_target =
                fs::read_link(&src).map_err(|e| format!("Cannot read symlink: {}", e))?;
            let base = std::path::Path::new(&self.base_path)
                .canonicalize()
                .map_err(|e| e.to_string())?;
            if !base.join(&link_target).starts_with(&base) {
                return Err("Symlink target outside base path".to_string());
            }
        }
        if let Some(parent) = Path::new(local_path).parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
        }
        fs::copy(&src, local_path).map_err(|e| format!("Failed to copy file: {}", e))?;
        Ok(())
    }

    fn delete_file(&self, remote_path: &str) -> Result<(), String> {
        let path = safe_join(&self.base_path, remote_path)?;
        if Path::new(&path).exists() {
            fs::remove_file(&path).map_err(|e| format!("Failed to delete file: {}", e))?;
        }
        Ok(())
    }

    fn list_files(&self, prefix: &str) -> Result<Vec<RemoteFile>, String> {
        let dir = safe_join(&self.base_path, prefix)?;
        if !Path::new(&dir).exists() || !Path::new(&dir).is_dir() {
            return Ok(Vec::new());
        }

        let base = std::path::Path::new(&self.base_path)
            .canonicalize()
            .map_err(|e| e.to_string())?;
        let mut files = Vec::new();
        for entry in fs::read_dir(&dir).map_err(|e| format!("Failed to read directory: {}", e))? {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();

            // Skip symlinks that point outside base
            if path.is_symlink() {
                let target = fs::read_link(&path).unwrap_or_default();
                if !base.join(&target).starts_with(&base) {
                    continue;
                }
            }

            if path.is_file() {
                let metadata = entry.metadata().ok();
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let relative = path
                    .strip_prefix(&self.base_path)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                let modified = metadata
                    .and_then(|m| m.modified().ok())
                    .map(|t| {
                        chrono::DateTime::<chrono::Utc>::from(t)
                            .format("%Y-%m-%dT%H:%M:%SZ")
                            .to_string()
                    })
                    .unwrap_or_default();
                let size = metadata.map(|m| m.len()).unwrap_or(0);
                files.push(RemoteFile {
                    name,
                    path: relative,
                    size_bytes: size,
                    modified_at: modified,
                    url: path.to_string_lossy().to_string(),
                });
            }
        }
        Ok(files)
    }

    fn get_file_url(&self, remote_path: &str) -> Result<String, String> {
        let full = safe_join(&self.base_path, remote_path)?;
        Ok(full)
    }

    fn test_connection(&self) -> Result<bool, String> {
        let path = std::path::Path::new(&self.base_path);
        if !path.exists() {
            fs::create_dir_all(path)
                .map_err(|e| format!("Cannot create base path '{}': {}", self.base_path, e))?;
        }
        Ok(true)
    }
}

// ===========================================================================
// 2. GitHubBackend
// ===========================================================================

pub struct GitHubBackend {
    token: String,
    repo_name: String,
    branch: String,
}

impl GitHubBackend {
    pub fn new(token: &str, repo_name: &str, branch: &str) -> Self {
        Self {
            token: token.to_string(),
            repo_name: repo_name.to_string(),
            branch: branch.to_string(),
        }
    }

    /// Upload via GitHub Releases API for files > 100 MB.
    fn upload_via_release(&self, local_path: &str, remote_path: &str) -> Result<String, String> {
        let (owner, repo) = parse_repo(&self.repo_name)?;
        let file_name = Path::new(remote_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "upload".to_string());

        let client = http_client()?;

        // 1. Create a release
        let tag = format!("sync-{}", chrono::Utc::now().format("%Y%m%d%H%M%S"));
        let release_body = serde_json::json!({
            "tag_name": tag,
            "name": format!("Sync upload: {}", file_name),
            "body": format!("Uploaded via Cybermanju Drive sync: {}", remote_path),
            "draft": true,
            "prerelease": false
        });

        let resp = client
            .post(&format!(
                "https://api.github.com/repos/{}/{}/releases",
                owner, repo
            ))
            .header("Authorization", format!("token {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .json(&release_body)
            .send()
            .map_err(|e| format!("GitHub release request failed: {}", e))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .map_err(|e| format!("Failed to read release response: {}", e))?;

        if status < 200 || status >= 300 {
            return Err(format!(
                "GitHub release creation failed ({}): {}",
                status, body
            ));
        }

        let release: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse release response: {}", e))?;
        let _release_id = release["id"].as_u64().ok_or("No release ID in response")?;
        let upload_url = release["upload_url"]
            .as_str()
            .unwrap_or("")
            .replace("{?name,label}", "");

        // 2. Upload the asset
        let file_bytes = fs::read(local_path)
            .map_err(|e| format!("Failed to read file for release upload: {}", e))?;

        let upload_endpoint = format!("{}?name={}", upload_url, file_name);
        let up_resp = client
            .post(&upload_endpoint)
            .header("Authorization", format!("token {}", self.token))
            .header("Content-Type", "application/octet-stream")
            .body(file_bytes)
            .send()
            .map_err(|e| format!("GitHub release upload request failed: {}", e))?;

        let up_status = up_resp.status().as_u16();
        let up_body = up_resp.text().unwrap_or_default();

        if up_status < 200 || up_status >= 300 {
            return Err(format!(
                "GitHub release upload failed ({}): {}",
                up_status, up_body
            ));
        }

        let release_url = format!("https://github.com/{}/{}/releases/tag/{}", owner, repo, tag);
        Ok(release_url)
    }
}

impl StorageBackend for GitHubBackend {
    fn name(&self) -> &str {
        "GitHub"
    }

    fn backend_type(&self) -> SyncBackendType {
        SyncBackendType::GitHub
    }

    fn upload_file(&self, local_path: &str, remote_path: &str) -> Result<String, String> {
        let file_size = fs::metadata(local_path).map(|m| m.len()).unwrap_or(0);
        const LARGE_FILE_THRESHOLD: u64 = 100 * 1024 * 1024; // 100 MB

        if file_size > LARGE_FILE_THRESHOLD {
            info!("File > 100 MB, using GitHub Releases API");
            return self.upload_via_release(local_path, remote_path);
        }

        let (owner, repo) = parse_repo(&self.repo_name)?;
        let data = fs::read(local_path)
            .map_err(|e| format!("Failed to read file '{}': {}", local_path, e))?;
        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);

        let put_body = serde_json::json!({
            "message": format!("Sync upload: {}", remote_path),
            "content": b64,
            "branch": self.branch,
        });

        let url = format!(
            "https://api.github.com/repos/{}/{}/contents/{}",
            owner, repo, remote_path
        );

        let client = http_client()?;
        let resp = client
            .put(&url)
            .header("Authorization", format!("token {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .json(&put_body)
            .send()
            .map_err(|e| format!("GitHub upload request failed: {}", e))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .map_err(|e| format!("Failed to read upload response: {}", e))?;

        if status < 200 || status >= 300 {
            return Err(format!("GitHub upload failed ({}): {}", status, body));
        }

        let resp_json: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse upload response: {}", e))?;
        let download_url = resp_json["content"]["download_url"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(download_url)
    }

    fn download_file(&self, remote_path: &str, local_path: &str) -> Result<(), String> {
        let (owner, repo) = parse_repo(&self.repo_name)?;
        let url = format!(
            "https://api.github.com/repos/{}/{}/contents/{}",
            owner, repo, remote_path
        );

        let client = http_client()?;
        let resp = client
            .get(&url)
            .header("Authorization", format!("token {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .send()
            .map_err(|e| format!("GitHub download request failed: {}", e))?;

        let status = resp.status().as_u16();
        if status < 200 || status >= 300 {
            let body = resp.text().unwrap_or_default();
            return Err(format!("GitHub download failed ({}): {}", status, body));
        }

        let body = resp
            .text()
            .map_err(|e| format!("Failed to read download response: {}", e))?;
        let resp_json: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| format!("Failed to parse response: {}", e))?;
        let content_b64 = resp_json["content"]
            .as_str()
            .ok_or("No 'content' field in response")?;

        // GitHub API base64 may contain newlines; strip them
        let content_b64_clean: String = content_b64.chars().filter(|c| *c != '\n').collect();
        let data = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            content_b64_clean,
        )
        .map_err(|e| format!("Failed to decode base64: {}", e))?;

        if let Some(parent) = Path::new(local_path).parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
        }
        fs::write(local_path, &data).map_err(|e| format!("Failed to write file: {}", e))?;

        Ok(())
    }

    fn delete_file(&self, remote_path: &str) -> Result<(), String> {
        let (owner, repo) = parse_repo(&self.repo_name)?;
        let url = format!(
            "https://api.github.com/repos/{}/{}/contents/{}",
            owner, repo, remote_path
        );

        let client = http_client()?;

        // First get the file's SHA
        let get_resp = client
            .get(&url)
            .header("Authorization", format!("token {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .send()
            .map_err(|e| format!("GitHub get SHA request failed: {}", e))?;

        let get_status = get_resp.status().as_u16();
        if get_status == 404 {
            return Ok(());
        }
        if get_status < 200 || get_status >= 300 {
            let body = get_resp.text().unwrap_or_default();
            return Err(format!("GitHub get SHA failed ({}): {}", get_status, body));
        }

        let get_body = get_resp
            .text()
            .map_err(|e| format!("Failed to read get response: {}", e))?;
        let get_json: serde_json::Value = serde_json::from_str(&get_body)
            .map_err(|e| format!("Failed to parse get response: {}", e))?;
        let sha = get_json["sha"]
            .as_str()
            .ok_or("No 'sha' field in response")?;

        let delete_body = serde_json::json!({
            "message": format!("Sync delete: {}", remote_path),
            "sha": sha,
            "branch": self.branch,
        });

        let del_resp = client
            .delete(&url)
            .header("Authorization", format!("token {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .json(&delete_body)
            .send()
            .map_err(|e| format!("GitHub delete request failed: {}", e))?;

        let del_status = del_resp.status().as_u16();
        if del_status < 200 || del_status >= 300 {
            let del_body = del_resp.text().unwrap_or_default();
            return Err(format!(
                "GitHub delete failed ({}): {}",
                del_status, del_body
            ));
        }

        Ok(())
    }

    fn list_files(&self, prefix: &str) -> Result<Vec<RemoteFile>, String> {
        let (owner, repo) = parse_repo(&self.repo_name)?;
        let url = format!(
            "https://api.github.com/repos/{}/{}/contents/{}",
            owner, repo, prefix
        );

        let client = http_client()?;
        let resp = client
            .get(&url)
            .header("Authorization", format!("token {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .send()
            .map_err(|e| format!("GitHub list request failed: {}", e))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .map_err(|e| format!("Failed to read list response: {}", e))?;

        if status < 200 || status >= 300 {
            return Err(format!("GitHub list failed ({}): {}", status, body));
        }

        let items: Vec<serde_json::Value> =
            serde_json::from_str(&body).map_err(|e| format!("Failed to parse listing: {}", e))?;

        let mut files = Vec::new();
        for item in &items {
            // Skip directories
            if item["type"].as_str() == Some("dir") {
                continue;
            }
            let name = item["name"].as_str().unwrap_or("unknown").to_string();
            let path = item["path"].as_str().unwrap_or(&name).to_string();
            let size = item["size"].as_u64().unwrap_or(0);
            let modified = item
                .get("created_at")
                .or_else(|| item.get("updated_at"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let url_str = item["download_url"].as_str().unwrap_or("").to_string();

            files.push(RemoteFile {
                name,
                path,
                size_bytes: size,
                modified_at: modified,
                url: url_str,
            });
        }

        Ok(files)
    }

    fn get_file_url(&self, remote_path: &str) -> Result<String, String> {
        let (owner, repo) = parse_repo(&self.repo_name)?;
        Ok(format!(
            "https://raw.githubusercontent.com/{}/{}/{}/{}",
            owner, repo, self.branch, remote_path
        ))
    }

    fn test_connection(&self) -> Result<bool, String> {
        let client = http_client()?;
        let resp = client
            .get("https://api.github.com/user")
            .header("Authorization", format!("token {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .send()
            .map_err(|e| format!("GitHub connection test request failed: {}", e))?;

        let status = resp.status().as_u16();
        // Consume response body to free connection
        let _ = resp.text();

        if status == 200 {
            Ok(true)
        } else {
            Err(format!("GitHub connection test failed (HTTP {})", status))
        }
    }
}

// ===========================================================================
// 3. GitLabBackend
// ===========================================================================

pub struct GitLabBackend {
    token: String,
    project_id: String,
    branch: String,
    base_url: String,
}

impl GitLabBackend {
    pub fn new(token: &str, project_id: &str, branch: &str, base_url: Option<&str>) -> Self {
        Self {
            token: token.to_string(),
            project_id: project_id.to_string(),
            branch: branch.to_string(),
            base_url: base_url
                .unwrap_or("https://gitlab.com")
                .trim_end_matches('/')
                .to_string(),
        }
    }

    fn api_url(&self, endpoint: &str) -> String {
        format!(
            "{}/api/v4/projects/{}/{}",
            self.base_url, self.project_id, endpoint
        )
    }
}

impl StorageBackend for GitLabBackend {
    fn name(&self) -> &str {
        "GitLab"
    }

    fn backend_type(&self) -> SyncBackendType {
        SyncBackendType::GitLab
    }

    fn upload_file(&self, local_path: &str, remote_path: &str) -> Result<String, String> {
        let file_name = Path::new(local_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "upload".to_string());

        let data = fs::read(local_path)
            .map_err(|e| format!("Failed to read file '{}': {}", local_path, e))?;
        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);

        let encoded_path = urlencoding(remote_path);
        let url = format!(
            "{}/repository/files/{}",
            self.api_url("repository"),
            encoded_path
        );

        let client = http_client()?;

        // First check if file exists to decide POST (create) vs PUT (update)
        let check_resp = client
            .head(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .send();

        let file_exists = match check_resp {
            Ok(r) => r.status().as_u16() == 200,
            Err(_) => false,
        };

        let resp = if file_exists {
            // Update existing file
            let put_body = serde_json::json!({
                "branch": self.branch,
                "content": b64,
                "encoding": "base64",
                "commit_message": format!("Sync update: {}", remote_path),
            });
            client
                .put(&url)
                .header("PRIVATE-TOKEN", &self.token)
                .header("Content-Type", "application/json")
                .json(&put_body)
                .send()
                .map_err(|e| format!("GitLab update request failed: {}", e))?
        } else {
            // Create new file
            let post_body = serde_json::json!({
                "branch": self.branch,
                "content": b64,
                "encoding": "base64",
                "commit_message": format!("Sync upload: {}", remote_path),
            });
            client
                .post(&url)
                .header("PRIVATE-TOKEN", &self.token)
                .header("Content-Type", "application/json")
                .json(&post_body)
                .send()
                .map_err(|e| format!("GitLab upload request failed: {}", e))?
        };

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .map_err(|e| format!("Failed to read upload response: {}", e))?;

        if status < 200 || status >= 300 {
            return Err(format!("GitLab upload failed ({}): {}", status, body));
        }

        let resp_json: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse upload response: {}", e))?;
        let file_path = resp_json["file_path"]
            .as_str()
            .unwrap_or(&file_name)
            .to_string();

        Ok(format!(
            "{}/-/blob/{}/{}",
            self.base_url, self.branch, file_path
        ))
    }

    fn download_file(&self, remote_path: &str, local_path: &str) -> Result<(), String> {
        let encoded_path = urlencoding(remote_path);
        let url = format!(
            "{}/repository/files/{}/raw?ref={}",
            self.api_url("repository"),
            encoded_path,
            urlencoding(&self.branch)
        );

        let client = http_client()?;
        let resp = client
            .get(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .send()
            .map_err(|e| format!("GitLab download request failed: {}", e))?;

        let status = resp.status().as_u16();
        if status < 200 || status >= 300 {
            let body = resp.text().unwrap_or_default();
            return Err(format!("GitLab download failed ({}): {}", status, body));
        }

        let bytes = resp
            .bytes()
            .map_err(|e| format!("Failed to read download body: {}", e))?;

        if let Some(parent) = Path::new(local_path).parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
        }
        fs::write(local_path, &bytes).map_err(|e| format!("Failed to write file: {}", e))?;

        Ok(())
    }

    fn delete_file(&self, remote_path: &str) -> Result<(), String> {
        let encoded_path = urlencoding(remote_path);
        let url = format!(
            "{}/repository/files/{}",
            self.api_url("repository"),
            encoded_path
        );

        let delete_body = serde_json::json!({
            "branch": self.branch,
            "commit_message": format!("Sync delete: {}", remote_path),
        });

        let client = http_client()?;
        let resp = client
            .delete(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .header("Content-Type", "application/json")
            .json(&delete_body)
            .send()
            .map_err(|e| format!("GitLab delete request failed: {}", e))?;

        let status = resp.status().as_u16();
        if status == 204 || status == 200 {
            Ok(())
        } else {
            let body = resp.text().unwrap_or_default();
            Err(format!("GitLab delete failed ({}): {}", status, body))
        }
    }

    fn list_files(&self, prefix: &str) -> Result<Vec<RemoteFile>, String> {
        let url = if prefix.is_empty() {
            format!(
                "{}/repository/tree?ref={}&per_page=100",
                self.api_url("repository"),
                urlencoding(&self.branch)
            )
        } else {
            format!(
                "{}/repository/tree?ref={}&path={}&per_page=100",
                self.api_url("repository"),
                urlencoding(&self.branch),
                urlencoding(prefix)
            )
        };

        let client = http_client()?;
        let resp = client
            .get(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .send()
            .map_err(|e| format!("GitLab list request failed: {}", e))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .map_err(|e| format!("Failed to read list response: {}", e))?;

        if status < 200 || status >= 300 {
            return Err(format!("GitLab list failed ({}): {}", status, body));
        }

        let items: Vec<serde_json::Value> =
            serde_json::from_str(&body).map_err(|e| format!("Failed to parse listing: {}", e))?;

        let mut files = Vec::new();
        for item in &items {
            // Skip directories (trees)
            if item["type"].as_str() == Some("tree") {
                continue;
            }
            let name = item["name"].as_str().unwrap_or("unknown").to_string();
            let path = item["path"].as_str().unwrap_or(&name).to_string();

            files.push(RemoteFile {
                name,
                path: path.clone(),
                size_bytes: 0, // GitLab tree API doesn't return size
                modified_at: String::new(),
                url: format!("{}/-/raw/{}/{}", self.base_url, self.branch, path),
            });
        }

        Ok(files)
    }

    fn get_file_url(&self, remote_path: &str) -> Result<String, String> {
        Ok(format!(
            "{}/-/raw/{}/{}",
            self.base_url, self.branch, remote_path
        ))
    }

    fn test_connection(&self) -> Result<bool, String> {
        let client = http_client()?;
        let url = format!("{}/api/v4/projects/{}", self.base_url, self.project_id);
        let resp = client
            .get(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .send()
            .map_err(|e| format!("GitLab connection test request failed: {}", e))?;

        let status = resp.status().as_u16();
        // Consume response body to free connection
        let _ = resp.text();

        if status == 200 {
            Ok(true)
        } else {
            Err(format!("GitLab connection test failed (HTTP {})", status))
        }
    }
}

// ===========================================================================
// 4. GoogleDriveBackend
// ===========================================================================

pub struct GoogleDriveBackend {
    token: String,
    folder_id: Option<String>,
    credentials: Option<OAuthCredentials>,
}

impl GoogleDriveBackend {
    pub fn new(token: &str, folder_id: Option<&str>) -> Self {
        Self {
            token: token.to_string(),
            folder_id: folder_id.map(|s| s.to_string()),
            credentials: None,
        }
    }

    pub fn with_credentials(
        token: &str,
        folder_id: Option<&str>,
        credentials: OAuthCredentials,
    ) -> Self {
        Self {
            token: token.to_string(),
            folder_id: folder_id.map(|s| s.to_string()),
            credentials: Some(credentials),
        }
    }

    fn get_valid_token(&mut self) -> Result<String, String> {
        if let Some(ref mut creds) = self.credentials {
            // Try to refresh if expired (with 5 minute buffer)
            if creds.is_expired(300) {
                info!("Google Drive token expired, refreshing...");
                oauth::refresh_google_token(creds)?;
                self.token = creds.access_token.clone();
            }
        }
        Ok(self.token.clone())
    }
}

impl StorageBackend for GoogleDriveBackend {
    fn name(&self) -> &str {
        "Google Drive"
    }

    fn backend_type(&self) -> SyncBackendType {
        SyncBackendType::GoogleDrive
    }

    fn upload_file(&self, local_path: &str, _remote_path: &str) -> Result<String, String> {
        let file_name = Path::new(local_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "upload".to_string());

        let file_data = fs::read(local_path).map_err(|e| format!("Failed to read file: {}", e))?;

        // Build the JSON metadata part
        let mut metadata = serde_json::json!({
            "name": file_name,
        });
        if let Some(ref folder_id) = self.folder_id {
            metadata["parents"] = serde_json::json!([folder_id]);
        }

        let client = http_client()?;

        // Use reqwest multipart
        let form = reqwest::blocking::multipart::Form::new()
            .part(
                "metadata",
                reqwest::blocking::multipart::Part::text(
                    serde_json::to_string(&metadata)
                        .map_err(|e| format!("Failed to serialize metadata: {}", e))?,
                )
                .mime_str("application/json; charset=UTF-8")
                .map_err(|e| format!("Invalid MIME: {}", e))?,
            )
            .part(
                "file",
                reqwest::blocking::multipart::Part::bytes(file_data)
                    .file_name(file_name)
                    .mime_str("application/octet-stream")
                    .map_err(|e| format!("Invalid MIME: {}", e))?,
            );

        let resp = client
            .post("https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart")
            .header("Authorization", format!("Bearer {}", self.token))
            .multipart(form)
            .send()
            .map_err(|e| format!("Google Drive upload request failed: {}", e))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .map_err(|e| format!("Failed to read upload response: {}", e))?;

        if status < 200 || status >= 300 {
            return Err(format!("Google Drive upload failed ({}): {}", status, body));
        }

        let resp_json: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| format!("Failed to parse response: {}", e))?;
        let file_id = resp_json["id"]
            .as_str()
            .ok_or("No 'id' in upload response")?;

        Ok(format!("https://drive.google.com/file/d/{}/view", file_id))
    }

    fn download_file(&self, remote_path: &str, local_path: &str) -> Result<(), String> {
        // remote_path for Google Drive should be the file ID
        let file_id = remote_path;
        let url = format!(
            "https://www.googleapis.com/drive/v3/files/{}?alt=media",
            file_id
        );

        let client = http_client()?;
        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .map_err(|e| format!("Google Drive download request failed: {}", e))?;

        let status = resp.status().as_u16();
        if status < 200 || status >= 300 {
            let body = resp.text().unwrap_or_default();
            return Err(format!(
                "Google Drive download failed ({}): {}",
                status, body
            ));
        }

        let bytes = resp
            .bytes()
            .map_err(|e| format!("Failed to read download body: {}", e))?;

        if let Some(parent) = Path::new(local_path).parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
        }
        fs::write(local_path, &bytes).map_err(|e| format!("Failed to write file: {}", e))?;

        Ok(())
    }

    fn delete_file(&self, remote_path: &str) -> Result<(), String> {
        let file_id = remote_path;
        let url = format!("https://www.googleapis.com/drive/v3/files/{}", file_id);

        let client = http_client()?;
        let resp = client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .map_err(|e| format!("Google Drive delete request failed: {}", e))?;

        let status = resp.status().as_u16();
        // 204 No Content is success
        if status == 204 || status == 200 {
            Ok(())
        } else {
            let body = resp.text().unwrap_or_default();
            Err(format!("Google Drive delete failed ({}): {}", status, body))
        }
    }

    fn list_files(&self, _prefix: &str) -> Result<Vec<RemoteFile>, String> {
        let query = match &self.folder_id {
            Some(folder_id) => format!("'{}' in parents and trashed=false", folder_id),
            None => "trashed=false".to_string(),
        };

        let url = format!(
            "https://www.googleapis.com/drive/v3/files?q={}&fields=files(id,name,size,modifiedTime,webContentLink),nextPageToken",
            urlencoding(&query)
        );

        let client = http_client()?;
        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .map_err(|e| format!("Google Drive list request failed: {}", e))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .map_err(|e| format!("Failed to read list response: {}", e))?;

        if status < 200 || status >= 300 {
            return Err(format!("Google Drive list failed ({}): {}", status, body));
        }

        let resp_json: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| format!("Failed to parse response: {}", e))?;
        let files_array = resp_json["files"].as_array().cloned().unwrap_or_default();

        let mut files = Vec::new();
        for item in &files_array {
            let name = item["name"].as_str().unwrap_or("unknown").to_string();
            let file_id = item["id"].as_str().unwrap_or("").to_string();
            // size can be a number or string depending on the API response
            let size = item["size"]
                .as_u64()
                .or_else(|| item["size"].as_str().and_then(|s| s.parse().ok()))
                .unwrap_or(0);
            let modified = item["modifiedTime"].as_str().unwrap_or("").to_string();
            let url_str = format!("https://drive.google.com/file/d/{}/view", file_id);

            files.push(RemoteFile {
                name,
                path: file_id.clone(),
                size_bytes: size,
                modified_at: modified,
                url: url_str,
            });
        }

        Ok(files)
    }

    fn get_file_url(&self, remote_path: &str) -> Result<String, String> {
        Ok(format!(
            "https://drive.google.com/file/d/{}/view",
            remote_path
        ))
    }

    fn test_connection(&self) -> Result<bool, String> {
        let client = http_client()?;
        let resp = client
            .get("https://www.googleapis.com/drive/v3/about?fields=user")
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .map_err(|e| format!("Google Drive connection test request failed: {}", e))?;

        let status = resp.status().as_u16();
        // Consume response body to free connection
        let _ = resp.text();

        if status == 200 {
            Ok(true)
        } else {
            Err(format!(
                "Google Drive connection test failed (HTTP {})",
                status
            ))
        }
    }
}

// ===========================================================================
// 5. GooglePhotosBackend
// ===========================================================================

pub struct GooglePhotosBackend {
    token: String,
    album_id: Option<String>,
    credentials: Option<OAuthCredentials>,
}

impl GooglePhotosBackend {
    pub fn new(token: &str, album_id: Option<&str>) -> Self {
        Self {
            token: token.to_string(),
            album_id: album_id.map(|s| s.to_string()),
            credentials: None,
        }
    }

    pub fn with_credentials(
        token: &str,
        album_id: Option<&str>,
        credentials: OAuthCredentials,
    ) -> Self {
        Self {
            token: token.to_string(),
            album_id: album_id.map(|s| s.to_string()),
            credentials: Some(credentials),
        }
    }
}

impl StorageBackend for GooglePhotosBackend {
    fn name(&self) -> &str {
        "Google Photos"
    }

    fn backend_type(&self) -> SyncBackendType {
        SyncBackendType::GooglePhotos
    }

    fn upload_file(&self, local_path: &str, _remote_path: &str) -> Result<String, String> {
        let file_data = fs::read(local_path).map_err(|e| format!("Failed to read file: {}", e))?;

        let client = http_client()?;

        // Step 1: Upload the raw bytes to get an upload token
        let upload_resp = client
            .post("https://photoslibrary.googleapis.com/v1/uploads")
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/octet-stream")
            .header("X-Goog-Upload-Protocol", "raw")
            .body(file_data)
            .send()
            .map_err(|e| format!("Google Photos upload token request failed: {}", e))?;

        let up_status = upload_resp.status().as_u16();
        let up_body = upload_resp
            .text()
            .map_err(|e| format!("Failed to read upload token response: {}", e))?;

        if up_status < 200 || up_status >= 300 {
            return Err(format!(
                "Google Photos upload token failed ({}): {}",
                up_status, up_body
            ));
        }

        let upload_token = up_body.trim().to_string();

        // Step 2: Create a media item with the upload token
        let mut create_body = serde_json::json!({
            "newMediaItems": [{
                "simpleMediaItem": {
                    "uploadToken": upload_token
                }
            }]
        });
        if let Some(ref album_id) = self.album_id {
            create_body["albumId"] = serde_json::json!(album_id);
        }

        let create_resp = client
            .post("https://photoslibrary.googleapis.com/v1/mediaItems:batchCreate")
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .json(&create_body)
            .send()
            .map_err(|e| format!("Google Photos batchCreate request failed: {}", e))?;

        let create_status = create_resp.status().as_u16();
        let create_body = create_resp
            .text()
            .map_err(|e| format!("Failed to read batchCreate response: {}", e))?;

        if create_status < 200 || create_status >= 300 {
            return Err(format!(
                "Google Photos media item creation failed ({}): {}",
                create_status, create_body
            ));
        }

        let resp_json: serde_json::Value = serde_json::from_str(&create_body)
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        let media_item = &resp_json["newMediaItemResults"][0]["mediaItem"];
        let base_url = media_item["baseUrl"].as_str().unwrap_or("").to_string();

        Ok(base_url)
    }

    fn download_file(&self, remote_path: &str, local_path: &str) -> Result<(), String> {
        let url = format!(
            "https://photoslibrary.googleapis.com/v1/mediaItems/{}:download",
            remote_path
        );

        let client = http_client()?;
        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .map_err(|e| format!("Google Photos download request failed: {}", e))?;

        let status = resp.status().as_u16();
        if status < 200 || status >= 300 {
            let body = resp.text().unwrap_or_default();
            return Err(format!(
                "Google Photos download failed ({}): {}",
                status, body
            ));
        }

        let bytes = resp
            .bytes()
            .map_err(|e| format!("Failed to read download body: {}", e))?;

        if let Some(parent) = Path::new(local_path).parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
        }
        fs::write(local_path, &bytes).map_err(|e| format!("Failed to write file: {}", e))?;

        Ok(())
    }

    fn delete_file(&self, remote_path: &str) -> Result<(), String> {
        let url = format!(
            "https://photoslibrary.googleapis.com/v1/mediaItems/{}",
            remote_path
        );

        let client = http_client()?;
        let resp = client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .map_err(|e| format!("Google Photos delete request failed: {}", e))?;

        let status = resp.status().as_u16();
        if status == 200 || status == 204 {
            Ok(())
        } else {
            let body = resp.text().unwrap_or_default();
            Err(format!(
                "Google Photos delete failed ({}): {}",
                status, body
            ))
        }
    }

    fn list_files(&self, _prefix: &str) -> Result<Vec<RemoteFile>, String> {
        let url = "https://photoslibrary.googleapis.com/v1/mediaItems?pageSize=100";

        let client = http_client()?;
        let resp = client
            .get(url)
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .map_err(|e| format!("Google Photos list request failed: {}", e))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .map_err(|e| format!("Failed to read list response: {}", e))?;

        if status < 200 || status >= 300 {
            return Err(format!("Google Photos list failed ({}): {}", status, body));
        }

        let resp_json: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| format!("Failed to parse response: {}", e))?;
        let items = resp_json["mediaItems"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        let mut files = Vec::new();
        for item in &items {
            let item_id = item["id"].as_str().unwrap_or("").to_string();
            let filename = item["filename"].as_str().unwrap_or("unknown").to_string();
            let base_url = item["baseUrl"].as_str().unwrap_or("").to_string();
            let modified = item["mediaMetadata"]["creationTime"]
                .as_str()
                .or_else(|| item["creationTime"].as_str())
                .unwrap_or("")
                .to_string();

            files.push(RemoteFile {
                name: filename,
                path: item_id.clone(),
                size_bytes: 0, // Google Photos doesn't return size in list
                modified_at: modified,
                url: base_url,
            });
        }

        Ok(files)
    }

    fn get_file_url(&self, remote_path: &str) -> Result<String, String> {
        let url = format!(
            "https://photoslibrary.googleapis.com/v1/mediaItems/{}",
            remote_path
        );

        let client = http_client()?;
        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .map_err(|e| format!("Google Photos get URL request failed: {}", e))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .map_err(|e| format!("Failed to read response: {}", e))?;

        if status < 200 || status >= 300 {
            return Err(format!(
                "Google Photos get URL failed ({}): {}",
                status, body
            ));
        }

        let resp_json: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| format!("Failed to parse response: {}", e))?;
        let base_url = resp_json["baseUrl"].as_str().unwrap_or("").to_string();

        Ok(base_url)
    }

    fn test_connection(&self) -> Result<bool, String> {
        let client = http_client()?;
        let resp = client
            .get("https://photoslibrary.googleapis.com/v1/albums?pageSize=1")
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .map_err(|e| format!("Google Photos connection test request failed: {}", e))?;

        let status = resp.status().as_u16();
        // Consume response body to free connection
        let _ = resp.text();

        if status == 200 {
            Ok(true)
        } else {
            Err(format!(
                "Google Photos connection test failed (HTTP {})",
                status
            ))
        }
    }
}

// ===========================================================================
// 6. TelegramBackend
// ===========================================================================

pub struct TelegramBackend {
    bot_token: String,
    chat_id: String,
}

impl TelegramBackend {
    pub fn new(bot_token: &str, chat_id: &str) -> Self {
        Self {
            bot_token: bot_token.to_string(),
            chat_id: chat_id.to_string(),
        }
    }

    fn api_url(&self, method: &str) -> String {
        format!("https://api.telegram.org/bot{}/{}", self.bot_token, method)
    }
}

impl StorageBackend for TelegramBackend {
    fn name(&self) -> &str {
        "Telegram"
    }

    fn backend_type(&self) -> SyncBackendType {
        SyncBackendType::Telegram
    }

    fn upload_file(&self, local_path: &str, _remote_path: &str) -> Result<String, String> {
        let file_name = Path::new(local_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "upload".to_string());

        let file_data = fs::read(local_path)
            .map_err(|e| format!("Failed to read file '{}': {}", local_path, e))?;

        let client = http_client()?;

        // Telegram Bot API: sendDocument for files up to 50 MB
        // For larger files, use sendPhoto/sendVideo which support up to 2 GB via local path
        // but since we have bytes, we use multipart form with sendDocument
        let form = reqwest::blocking::multipart::Form::new()
            .text("chat_id", self.chat_id.clone())
            .text("caption", format!("Sync upload: {}", file_name))
            .part(
                "document",
                reqwest::blocking::multipart::Part::bytes(file_data)
                    .file_name(file_name)
                    .mime_str("application/octet-stream")
                    .map_err(|e| format!("Invalid MIME: {}", e))?,
            );

        let resp = client
            .post(&self.api_url("sendDocument"))
            .multipart(form)
            .send()
            .map_err(|e| format!("Telegram upload request failed: {}", e))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .map_err(|e| format!("Failed to read upload response: {}", e))?;

        if status < 200 || status >= 300 {
            return Err(format!("Telegram upload failed ({}): {}", status, body));
        }

        let resp_json: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| format!("Failed to parse response: {}", e))?;

        if !resp_json["ok"].as_bool().unwrap_or(false) {
            let desc = resp_json["description"].as_str().unwrap_or("Unknown error");
            return Err(format!("Telegram API error: {}", desc));
        }

        // Return the message link or a reference
        let message_id = resp_json["result"]["message_id"].as_i64().unwrap_or(0);
        let chat_id = resp_json["result"]["chat"]["id"]
            .as_i64()
            .map(|id| id.to_string())
            .unwrap_or_else(|| self.chat_id.clone());

        Ok(format!(
            "https://t.me/c/{}/{}",
            chat_id.trim_start_matches('-'),
            message_id
        ))
    }

    fn download_file(&self, remote_path: &str, local_path: &str) -> Result<(), String> {
        // remote_path should be a file_id from a previous upload
        let url = format!(
            "https://api.telegram.org/bot{}/getFile?file_id={}",
            self.bot_token, remote_path
        );

        let client = http_client()?;
        let resp = client
            .get(&url)
            .send()
            .map_err(|e| format!("Telegram getFile request failed: {}", e))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .map_err(|e| format!("Failed to read response: {}", e))?;

        if status < 200 || status >= 300 {
            return Err(format!("Telegram getFile failed ({}): {}", status, body));
        }

        let resp_json: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| format!("Failed to parse response: {}", e))?;

        if !resp_json["ok"].as_bool().unwrap_or(false) {
            let desc = resp_json["description"].as_str().unwrap_or("Unknown error");
            return Err(format!("Telegram API error: {}", desc));
        }

        let file_path = resp_json["result"]["file_path"]
            .as_str()
            .ok_or("No file_path in getFile response")?;

        // Download the file
        let download_url = format!(
            "https://api.telegram.org/file/bot{}/{}",
            self.bot_token, file_path
        );

        let dl_resp = client
            .get(&download_url)
            .send()
            .map_err(|e| format!("Telegram file download request failed: {}", e))?;

        let dl_status = dl_resp.status().as_u16();
        if dl_status < 200 || dl_status >= 300 {
            let dl_body = dl_resp.text().unwrap_or_default();
            return Err(format!(
                "Telegram file download failed ({}): {}",
                dl_status, dl_body
            ));
        }

        let bytes = dl_resp
            .bytes()
            .map_err(|e| format!("Failed to read download body: {}", e))?;

        if let Some(parent) = Path::new(local_path).parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
        }
        fs::write(local_path, &bytes).map_err(|e| format!("Failed to write file: {}", e))?;

        Ok(())
    }

    fn delete_file(&self, remote_path: &str) -> Result<(), String> {
        // Telegram Bot API doesn't support deleting messages sent by bots in channels
        // We can only delete messages in groups if the bot has admin rights
        // For now, we return Ok as a no-op since Telegram is append-only for most use cases
        let _ = remote_path;
        info!("Telegram: delete_file is a no-op (Telegram is append-only for bot messages)");
        Ok(())
    }

    fn list_files(&self, _prefix: &str) -> Result<Vec<RemoteFile>, String> {
        // Telegram doesn't have a native file listing API for bot-sent messages
        // We use getUpdates or getChatHistory to retrieve recent messages
        // For simplicity, we return an empty list - Telegram is primarily for upload
        // Users can view sent files in the Telegram chat directly
        Ok(Vec::new())
    }

    fn get_file_url(&self, remote_path: &str) -> Result<String, String> {
        // remote_path should be a file_id
        let url = format!(
            "https://api.telegram.org/bot{}/getFile?file_id={}",
            self.bot_token, remote_path
        );

        let client = http_client()?;
        let resp = client
            .get(&url)
            .send()
            .map_err(|e| format!("Telegram getFile request failed: {}", e))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .map_err(|e| format!("Failed to read response: {}", e))?;

        if status < 200 || status >= 300 {
            return Err(format!("Telegram getFile failed ({}): {}", status, body));
        }

        let resp_json: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| format!("Failed to parse response: {}", e))?;

        if !resp_json["ok"].as_bool().unwrap_or(false) {
            let desc = resp_json["description"].as_str().unwrap_or("Unknown error");
            return Err(format!("Telegram API error: {}", desc));
        }

        let file_path = resp_json["result"]["file_path"]
            .as_str()
            .ok_or("No file_path in getFile response")?;

        Ok(format!(
            "https://api.telegram.org/file/bot{}/{}",
            self.bot_token, file_path
        ))
    }

    fn test_connection(&self) -> Result<bool, String> {
        let client = http_client()?;
        let resp = client
            .get(&self.api_url("getMe"))
            .send()
            .map_err(|e| format!("Telegram connection test request failed: {}", e))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .map_err(|e| format!("Failed to read response: {}", e))?;

        if status != 200 {
            return Err(format!(
                "Telegram connection test failed (HTTP {}): {}",
                status, body
            ));
        }

        let resp_json: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| format!("Failed to parse response: {}", e))?;

        if resp_json["ok"].as_bool().unwrap_or(false) {
            let bot_name = resp_json["result"]["username"]
                .as_str()
                .unwrap_or("unknown");
            info!("Telegram bot connected: @{}", bot_name);
            Ok(true)
        } else {
            let desc = resp_json["description"].as_str().unwrap_or("Unknown error");
            Err(format!("Telegram API error: {}", desc))
        }
    }
}

// ===========================================================================
// URL-encoding helper (simple, no external dependency)
// ===========================================================================

fn urlencoding(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for byte in s.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char);
            }
            b => {
                out.push('%');
                out.push_str(&format!("{:02X}", b));
            }
        }
    }
    out
}

// ===========================================================================
// Factory: build a StorageBackend from a SyncConfig
// ===========================================================================

/// Create the appropriate `StorageBackend` from a `SyncConfig`.
pub fn create_backend(config: &SyncConfig) -> Result<Box<dyn StorageBackend>, String> {
    match config.backend_type {
        SyncBackendType::Local => {
            let base = config
                .base_path
                .as_deref()
                .ok_or("Local backend requires base_path")?;
            Ok(Box::new(LocalBackend::new(base)))
        }
        SyncBackendType::GitHub => {
            let token = config
                .token
                .as_deref()
                .ok_or("GitHub backend requires token")?;
            let repo = config
                .repo_name
                .as_deref()
                .ok_or("GitHub backend requires repo_name")?;
            let branch = config.branch.as_deref().unwrap_or("main");
            Ok(Box::new(GitHubBackend::new(token, repo, branch)))
        }
        SyncBackendType::GitLab => {
            let token = config
                .token
                .as_deref()
                .ok_or("GitLab backend requires token")?;
            let project_id = config
                .repo_name
                .as_deref()
                .ok_or("GitLab backend requires project_id (use repo_name field)")?;
            let branch = config.branch.as_deref().unwrap_or("main");
            let base_url = config.base_path.as_deref();
            Ok(Box::new(GitLabBackend::new(
                token, project_id, branch, base_url,
            )))
        }
        SyncBackendType::GoogleDrive => {
            let token = config
                .token
                .as_deref()
                .ok_or("Google Drive backend requires token")?;
            Ok(Box::new(GoogleDriveBackend::new(
                token,
                config.folder_id.as_deref(),
            )))
        }
        SyncBackendType::GooglePhotos => {
            let token = config
                .token
                .as_deref()
                .ok_or("Google Photos backend requires token")?;
            Ok(Box::new(GooglePhotosBackend::new(
                token,
                config.album_id.as_deref(),
            )))
        }
        SyncBackendType::Telegram => {
            let token = config
                .token
                .as_deref()
                .ok_or("Telegram backend requires bot_token (use token field)")?;
            let chat_id = config
                .chat_id
                .as_deref()
                .ok_or("Telegram backend requires chat_id")?;
            Ok(Box::new(TelegramBackend::new(token, chat_id)))
        }
    }
}
