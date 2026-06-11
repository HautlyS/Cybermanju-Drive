// Cybermanju Drive — Storage Sync Backends
// Four backend implementations: Local, GitHub, Google Drive, Google Photos
// All HTTP backends use `curl` subprocess to avoid external HTTP dependencies.

use crate::sync::models::*;
use log::info;
use std::fs;
use std::path::Path;

// ===========================================================================
// Helper: run curl and return (stdout, stderr, exit_code)
// ===========================================================================

fn run_curl(args: &[&str]) -> Result<(String, String, i32), String> {
    let mut cmd = std::process::Command::new("curl");
    for arg in args {
        cmd.arg(arg);
    }
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let output = cmd.output().map_err(|e| format!("Failed to execute curl: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);

    Ok((stdout, stderr, code))
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
        let dest = Path::new(&self.base_path).join(remote_path);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }
        fs::copy(local_path, &dest)
            .map_err(|e| format!("Failed to copy file: {}", e))?;
        Ok(dest.to_string_lossy().to_string())
    }

    fn download_file(&self, remote_path: &str, local_path: &str) -> Result<(), String> {
        let src = Path::new(&self.base_path).join(remote_path);
        if let Some(parent) = Path::new(local_path).parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }
        fs::copy(&src, local_path)
            .map_err(|e| format!("Failed to copy file: {}", e))?;
        Ok(())
    }

    fn delete_file(&self, remote_path: &str) -> Result<(), String> {
        let path = Path::new(&self.base_path).join(remote_path);
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|e| format!("Failed to delete file: {}", e))?;
        }
        Ok(())
    }

    fn list_files(&self, prefix: &str) -> Result<Vec<RemoteFile>, String> {
        let dir = Path::new(&self.base_path).join(prefix);
        if !dir.exists() || !dir.is_dir() {
            return Ok(Vec::new());
        }

        let mut files = Vec::new();
        for entry in fs::read_dir(&dir)
            .map_err(|e| format!("Failed to read directory: {}", e))?
        {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();
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
        let full = Path::new(&self.base_path).join(remote_path);
        Ok(full.to_string_lossy().to_string())
    }

    fn test_connection(&self) -> Result<bool, String> {
        let path = Path::new(&self.base_path);
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
    fn upload_via_release(
        &self,
        local_path: &str,
        remote_path: &str,
    ) -> Result<String, String> {
        let (owner, repo) = parse_repo(&self.repo_name)?;
        let file_name = Path::new(remote_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "upload".to_string());

        // 1. Create a release
        let tag = format!("sync-{}", chrono::Utc::now().format("%Y%m%d%H%M%S"));
        let release_body = serde_json::json!({
            "tag_name": tag,
            "name": format!("Sync upload: {}", file_name),
            "body": format!("Uploaded via Cybermanju Drive sync: {}", remote_path),
            "draft": true,
            "prerelease": false
        });
        let release_json = serde_json::to_string(&release_body)
            .map_err(|e| format!("Failed to serialize release: {}", e))?;

        let (out, _err, code) = run_curl(&[
            "-s",
            "-w",
            "\n%{http_code}",
            "-X",
            "POST",
            "-H",
            &format!("Authorization: token {}", self.token),
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            "Content-Type: application/json",
            "-d",
            &release_json,
            &format!(
                "https://api.github.com/repos/{}/{}/releases",
                owner, repo
            ),
        ])?;

        // Parse HTTP status code from the last line
        let parts: Vec<&str> = out.rsplitn(2, '\n').collect();
        let status_code: i32 = parts
            .first()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(code);
        let body = if parts.len() > 1 { parts[1] } else { "" };

        if status_code < 200 || status_code >= 300 {
            return Err(format!(
                "GitHub release creation failed ({}): {}",
                status_code, body
            ));
        }

        let release: serde_json::Value = serde_json::from_str(body)
            .map_err(|e| format!("Failed to parse release response: {}", e))?;
        let _release_id = release["id"]
            .as_u64()
            .ok_or("No release ID in response")?;
        let upload_url = release["upload_url"]
            .as_str()
            .unwrap_or("")
            .replace("{?name,label}", "");

        // 2. Upload the asset
        let upload_endpoint = format!("{}?name={}", upload_url, file_name);
        let (up_out, up_err, up_code) = run_curl(&[
            "-s",
            "-w",
            "\n%{http_code}",
            "-X",
            "POST",
            "-H",
            &format!("Authorization: token {}", self.token),
            "-H",
            "Content-Type: application/octet-stream",
            "--data-binary",
            &format("@{}", local_path),
            &upload_endpoint,
        ])?;

        let up_parts: Vec<&str> = up_out.rsplitn(2, '\n').collect();
        let up_status: i32 = up_parts
            .first()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(up_code);

        if up_status < 200 || up_status >= 300 {
            return Err(format!(
                "GitHub release upload failed ({}): {}",
                up_status,
                if up_parts.len() > 1 { up_parts[1] } else { &up_err }
            ));
        }

        let release_url = format!(
            "https://github.com/{}/{}/releases/tag/{}",
            owner, repo, tag
        );
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
        let file_size = fs::metadata(local_path)
            .map(|m| m.len())
            .unwrap_or(0);
        const LARGE_FILE_THRESHOLD: u64 = 100 * 1024 * 1024; // 100 MB

        if file_size > LARGE_FILE_THRESHOLD {
            info!("File > 100 MB, using GitHub Releases API");
            return self.upload_via_release(local_path, remote_path);
        }

        let (owner, repo) = parse_repo(&self.repo_name)?;
        let data = fs::read(local_path)
            .map_err(|e| format!("Failed to read file '{}': {}", local_path, e))?;
        let b64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &data,
        );

        let put_body = serde_json::json!({
            "message": format!("Sync upload: {}", remote_path),
            "content": b64,
            "branch": self.branch,
        });
        let body_json = serde_json::to_string(&put_body)
            .map_err(|e| format!("Failed to serialize: {}", e))?;

        let url = format!(
            "https://api.github.com/repos/{}/{}/contents/{}",
            owner, repo, remote_path
        );

        let (out, _err, code) = run_curl(&[
            "-s",
            "-w",
            "\n%{http_code}",
            "-X",
            "PUT",
            "-H",
            &format!("Authorization: token {}", self.token),
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            "Content-Type: application/json",
            "-d",
            &body_json,
            &url,
        ])?;

        let parts: Vec<&str> = out.rsplitn(2, '\n').collect();
        let status_code: i32 = parts
            .first()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(code);
        let body = if parts.len() > 1 { parts[1] } else { "" };

        if status_code < 200 || status_code >= 300 {
            return Err(format!(
                "GitHub upload failed ({}): {}",
                status_code, body
            ));
        }

        let resp: serde_json::Value = serde_json::from_str(body)
            .map_err(|e| format!("Failed to parse upload response: {}", e))?;
        let download_url = resp["content"]["download_url"]
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

        let (out, _err, code) = run_curl(&[
            "-s",
            "-w",
            "\n%{http_code}",
            "-H",
            &format!("Authorization: token {}", self.token),
            "-H",
            "Accept: application/vnd.github+json",
            &url,
        ])?;

        let parts: Vec<&str> = out.rsplitn(2, '\n').collect();
        let status_code: i32 = parts
            .first()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(code);
        let body = if parts.len() > 1 { parts[1] } else { "" };

        if status_code < 200 || status_code >= 300 {
            return Err(format!(
                "GitHub download failed ({}): {}",
                status_code, body
            ));
        }

        let resp: serde_json::Value = serde_json::from_str(body)
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        let content_b64 = resp["content"]
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
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }
        fs::write(local_path, &data)
            .map_err(|e| format!("Failed to write file: {}", e))?;

        Ok(())
    }

    fn delete_file(&self, remote_path: &str) -> Result<(), String> {
        let (owner, repo) = parse_repo(&self.repo_name)?;

        // First get the file's SHA
        let url = format!(
            "https://api.github.com/repos/{}/{}/contents/{}",
            owner, repo, remote_path
        );

        let (out, _err, code) = run_curl(&[
            "-s",
            "-w",
            "\n%{http_code}",
            "-H",
            &format!("Authorization: token {}", self.token),
            "-H",
            "Accept: application/vnd.github+json",
            &url,
        ])?;

        let parts: Vec<&str> = out.rsplitn(2, '\n').collect();
        let status_code: i32 = parts
            .first()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(code);
        let body = if parts.len() > 1 { parts[1] } else { "" };

        if status_code == 404 {
            // File already gone
            return Ok(());
        }

        if status_code < 200 || status_code >= 300 {
            return Err(format!(
                "GitHub get SHA failed ({}): {}",
                status_code, body
            ));
        }

        let resp: serde_json::Value = serde_json::from_str(body)
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        let sha = resp["sha"]
            .as_str()
            .ok_or("No 'sha' field in response")?;

        let delete_body = serde_json::json!({
            "message": format!("Sync delete: {}", remote_path),
            "sha": sha,
            "branch": self.branch,
        });
        let delete_json = serde_json::to_string(&delete_body)
            .map_err(|e| format!("Failed to serialize: {}", e))?;

        let (del_out, del_err, del_code) = run_curl(&[
            "-s",
            "-w",
            "\n%{http_code}",
            "-X",
            "DELETE",
            "-H",
            &format!("Authorization: token {}", self.token),
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            "Content-Type: application/json",
            "-d",
            &delete_json,
            &url,
        ])?;

        let del_parts: Vec<&str> = del_out.rsplitn(2, '\n').collect();
        let del_status: i32 = del_parts
            .first()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(del_code);

        if del_status < 200 || del_status >= 300 {
            return Err(format!(
                "GitHub delete failed ({}): {}",
                del_status,
                if del_parts.len() > 1 { del_parts[1] } else { &del_err }
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

        let (out, _err, code) = run_curl(&[
            "-s",
            "-w",
            "\n%{http_code}",
            "-H",
            &format!("Authorization: token {}", self.token),
            "-H",
            "Accept: application/vnd.github+json",
            &url,
        ])?;

        let parts: Vec<&str> = out.rsplitn(2, '\n').collect();
        let status_code: i32 = parts
            .first()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(code);
        let body = if parts.len() > 1 { parts[1] } else { "" };

        if status_code < 200 || status_code >= 300 {
            return Err(format!(
                "GitHub list failed ({}): {}",
                status_code, body
            ));
        }

        let items: Vec<serde_json::Value> = serde_json::from_str(body)
            .map_err(|e| format!("Failed to parse listing: {}", e))?;

        let mut files = Vec::new();
        for item in &items {
            // Skip directories
            if item["type"].as_str() == Some("dir") {
                continue;
            }
            let name = item["name"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();
            let path = item["path"]
                .as_str()
                .unwrap_or(&name)
                .to_string();
            let size = item["size"].as_u64().unwrap_or(0);
            let modified = item.get("created_at")
                .or_else(|| item.get("updated_at"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let url_str = item["download_url"]
                .as_str()
                .unwrap_or("")
                .to_string();

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
            "https://raw.githubusercontent.com/{}/{}/{}",
            owner, repo, self.branch, remote_path
        ))
    }

    fn test_connection(&self) -> Result<bool, String> {
        let (out, err, code) = run_curl(&[
            "-s",
            "-o",
            "/dev/null",
            "-w",
            "%{http_code}",
            "-H",
            &format!("Authorization: token {}", self.token),
            "-H",
            "Accept: application/vnd.github+json",
            "https://api.github.com/user",
        ])?;

        let status: i32 = out.trim().parse().unwrap_or(code);
        if status == 200 {
            Ok(true)
        } else {
            Err(format!(
                "GitHub connection test failed (HTTP {}). {}",
                status, err
            ))
        }
    }
}

// ===========================================================================
// 3. GoogleDriveBackend
// ===========================================================================

pub struct GoogleDriveBackend {
    token: String,
    folder_id: Option<String>,
}

impl GoogleDriveBackend {
    pub fn new(token: &str, folder_id: Option<&str>) -> Self {
        Self {
            token: token.to_string(),
            folder_id: folder_id.map(|s| s.to_string()),
        }
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

        // Build the JSON metadata part
        let mut metadata = serde_json::json!({
            "name": file_name,
        });
        if let Some(ref folder_id) = self.folder_id {
            metadata["parents"] = serde_json::json!([folder_id]);
        }
        let metadata_json = serde_json::to_string(&metadata)
            .map_err(|e| format!("Failed to serialize metadata: {}", e))?;

        // Build the multipart body manually for curl
        let boundary = "cybermanju_drive_boundary";
        let mut multipart_body = Vec::new();
        multipart_body
            .extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        multipart_body.extend_from_slice(
            b"Content-Type: application/json; charset=UTF-8\r\n\r\n",
        );
        multipart_body.extend_from_slice(metadata_json.as_bytes());
        multipart_body.extend_from_slice(b"\r\n");
        multipart_body
            .extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        multipart_body.extend_from_slice(
            b"Content-Type: application/octet-stream\r\n\r\n",
        );
        let file_data = fs::read(local_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;
        multipart_body.extend_from_slice(&file_data);
        multipart_body
            .extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

        // Write multipart body to a temp file for curl --data-binary @file
        let tmp_path = format!("/tmp/cybermanju_gdrive_upload_{}.tmp", uuid::Uuid::new_v4());
        fs::write(&tmp_path, &multipart_body)
            .map_err(|e| format!("Failed to write temp file: {}", e))?;

        let content_type = format!("multipart/related; boundary={}", boundary);

        let result: Result<String, String> = (|| {
            let (out, _err, code) = run_curl(&[
                "-s",
                "-w",
                "\n%{http_code}",
                "-X",
                "POST",
                "-H",
                &format!("Authorization: Bearer {}", self.token),
                "-H",
                &format!("Content-Type: {}", content_type),
                "--data-binary",
                &format!("@{}", tmp_path),
                "https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart",
            ])?;
            let _ = fs::remove_file(&tmp_path);

            let parts: Vec<&str> = out.rsplitn(2, '\n').collect();
            let status_code: i32 = parts
                .first()
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(code);
            let body = if parts.len() > 1 { parts[1] } else { "" };

            if status_code < 200 || status_code >= 300 {
                return Err(format!(
                    "Google Drive upload failed ({}): {}",
                    status_code, body
                ));
            }

            let resp: serde_json::Value = serde_json::from_str(body)
                .map_err(|e| format!("Failed to parse response: {}", e))?;
            let file_id = resp["id"]
                .as_str()
                .ok_or("No 'id' in upload response")?;

            Ok(format!(
                "https://drive.google.com/file/d/{}/view",
                file_id
            ))
        })();

        // Clean up temp file if it still exists
        let _ = fs::remove_file(&tmp_path);

        result
    }

    fn download_file(&self, remote_path: &str, local_path: &str) -> Result<(), String> {
        // remote_path for Google Drive should be the file ID
        let file_id = remote_path;
        let url = format!(
            "https://www.googleapis.com/drive/v3/files/{}?alt=media",
            file_id
        );

        let (out, err, code) = run_curl(&[
            "-s",
            "-w",
            "\n%{http_code}",
            "-H",
            &format!("Authorization: Bearer {}", self.token),
            "-o",
            local_path,
            &url,
        ])?;

        let parts: Vec<&str> = out.rsplitn(2, '\n').collect();
        let status_code: i32 = parts
            .first()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(code);

        if status_code < 200 || status_code >= 300 {
            return Err(format!(
                "Google Drive download failed ({}): {}",
                status_code, err
            ));
        }

        Ok(())
    }

    fn delete_file(&self, remote_path: &str) -> Result<(), String> {
        let file_id = remote_path;
        let url = format!(
            "https://www.googleapis.com/drive/v3/files/{}",
            file_id
        );

        let (out, err, code) = run_curl(&[
            "-s",
            "-w",
            "\n%{http_code}",
            "-X",
            "DELETE",
            "-H",
            &format!("Authorization: Bearer {}", self.token),
            &url,
        ])?;

        let status_code: i32 = out
            .trim()
            .parse()
            .unwrap_or(code);

        // 204 No Content is success
        if status_code == 204 || status_code == 200 {
            Ok(())
        } else {
            Err(format!(
                "Google Drive delete failed ({}): {}",
                status_code, err
            ))
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

        let (out, _err, code) = run_curl(&[
            "-s",
            "-w",
            "\n%{http_code}",
            "-H",
            &format!("Authorization: Bearer {}", self.token),
            &url,
        ])?;

        let parts: Vec<&str> = out.rsplitn(2, '\n').collect();
        let status_code: i32 = parts
            .first()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(code);
        let body = if parts.len() > 1 { parts[1] } else { "" };

        if status_code < 200 || status_code >= 300 {
            return Err(format!(
                "Google Drive list failed ({}): {}",
                status_code, body
            ));
        }

        let resp: serde_json::Value = serde_json::from_str(body)
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        let files_array = resp["files"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        let mut files = Vec::new();
        for item in &files_array {
            let name = item["name"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();
            let file_id = item["id"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let size = item["size"]
                .as_str()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let modified = item["modifiedTime"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let url_str = format!(
                "https://drive.google.com/file/d/{}/view",
                file_id
            );

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
        let (out, err, code) = run_curl(&[
            "-s",
            "-w",
            "\n%{http_code}",
            "-H",
            &format!("Authorization: Bearer {}", self.token),
            "https://www.googleapis.com/drive/v3/about?fields=user",
        ])?;

        let parts: Vec<&str> = out.rsplitn(2, '\n').collect();
        let status_code: i32 = parts
            .first()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(code);

        if status_code == 200 {
            Ok(true)
        } else {
            Err(format!(
                "Google Drive connection test failed (HTTP {}). {}",
                status_code, err
            ))
        }
    }
}

// ===========================================================================
// 4. GooglePhotosBackend
// ===========================================================================

pub struct GooglePhotosBackend {
    token: String,
    album_id: Option<String>,
}

impl GooglePhotosBackend {
    pub fn new(token: &str, album_id: Option<&str>) -> Self {
        Self {
            token: token.to_string(),
            album_id: album_id.map(|s| s.to_string()),
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
        let file_data = fs::read(local_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        // Step 1: Upload the raw bytes to get an upload token
        let tmp_path = format!("/tmp/cybermanju_gphotos_upload_{}.tmp", uuid::Uuid::new_v4());
        fs::write(&tmp_path, &file_data)
            .map_err(|e| format!("Failed to write temp file: {}", e))?;

        let upload_result: Result<String, String> = (|| {
            let (out, _err, code) = run_curl(&[
                "-s",
                "-w",
                "\n%{http_code}",
                "-X",
                "POST",
                "-H",
                &format!("Authorization: Bearer {}", self.token),
                "-H",
                "Content-Type: application/octet-stream",
                "-H",
                "X-Goog-Upload-Protocol: raw",
                "--data-binary",
                &format!("@{}", tmp_path),
                "https://photoslibrary.googleapis.com/v1/uploads",
            ])?;
            let parts: Vec<&str> = out.rsplitn(2, '\n').collect();
            let status_code: i32 = parts
                .first()
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(code);
            let body = if parts.len() > 1 { parts[1] } else { "" };

            if status_code < 200 || status_code >= 300 {
                return Err(format!(
                    "Google Photos upload token failed ({}): {}",
                    status_code, body
                ));
            }

            Ok(body.trim().to_string())
        })();

        let _ = fs::remove_file(&tmp_path);
        let upload_token = upload_result?;

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
        let create_json = serde_json::to_string(&create_body)
            .map_err(|e| format!("Failed to serialize: {}", e))?;

        let (out, _err, code) = run_curl(&[
            "-s",
            "-w",
            "\n%{http_code}",
            "-X",
            "POST",
            "-H",
            &format!("Authorization: Bearer {}", self.token),
            "-H",
            "Content-Type: application/json",
            "-d",
            &create_json,
            "https://photoslibrary.googleapis.com/v1/mediaItems:batchCreate",
        ])?;

        let parts: Vec<&str> = out.rsplitn(2, '\n').collect();
        let status_code: i32 = parts
            .first()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(code);
        let body = if parts.len() > 1 { parts[1] } else { "" };

        if status_code < 200 || status_code >= 300 {
            return Err(format!(
                "Google Photos media item creation failed ({}): {}",
                status_code, body
            ));
        }

        let resp: serde_json::Value = serde_json::from_str(body)
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        let media_item = &resp["newMediaItemResults"][0]["mediaItem"];
        let base_url = media_item["baseUrl"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(base_url)
    }

    fn download_file(&self, remote_path: &str, local_path: &str) -> Result<(), String> {
        let url = format!(
            "https://photoslibrary.googleapis.com/v1/mediaItems/{}:download",
            remote_path
        );

        let (out, err, code) = run_curl(&[
            "-s",
            "-w",
            "\n%{http_code}",
            "-H",
            &format!("Authorization: Bearer {}", self.token),
            "-o",
            local_path,
            &url,
        ])?;

        let parts: Vec<&str> = out.rsplitn(2, '\n').collect();
        let status_code: i32 = parts
            .first()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(code);

        if status_code < 200 || status_code >= 300 {
            return Err(format!(
                "Google Photos download failed ({}): {}",
                status_code, err
            ));
        }

        Ok(())
    }

    fn delete_file(&self, remote_path: &str) -> Result<(), String> {
        let url = format!(
            "https://photoslibrary.googleapis.com/v1/mediaItems/{}",
            remote_path
        );

        let (out, err, code) = run_curl(&[
            "-s",
            "-w",
            "\n%{http_code}",
            "-X",
            "DELETE",
            "-H",
            &format!("Authorization: Bearer {}", self.token),
            &url,
        ])?;

        let status_code: i32 = out.trim().parse().unwrap_or(code);

        if status_code == 200 || status_code == 204 {
            Ok(())
        } else {
            Err(format!(
                "Google Photos delete failed ({}): {}",
                status_code, err
            ))
        }
    }

    fn list_files(&self, _prefix: &str) -> Result<Vec<RemoteFile>, String> {
        let url = "https://photoslibrary.googleapis.com/v1/mediaItems?pageSize=100";

        let (out, _err, code) = run_curl(&[
            "-s",
            "-w",
            "\n%{http_code}",
            "-H",
            &format!("Authorization: Bearer {}", self.token),
            url,
        ])?;

        let parts: Vec<&str> = out.rsplitn(2, '\n').collect();
        let status_code: i32 = parts
            .first()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(code);
        let body = if parts.len() > 1 { parts[1] } else { "" };

        if status_code < 200 || status_code >= 300 {
            return Err(format!(
                "Google Photos list failed ({}): {}",
                status_code, body
            ));
        }

        let resp: serde_json::Value = serde_json::from_str(body)
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        let items = resp["mediaItems"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        let mut files = Vec::new();
        for item in &items {
            let item_id = item["id"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let filename = item["filename"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();
            let base_url = item["baseUrl"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let modified = item["productUrl"]
                .as_str()
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
        // For Google Photos, return the baseUrl by fetching the media item
        let url = format!(
            "https://photoslibrary.googleapis.com/v1/mediaItems/{}",
            remote_path
        );

        let (out, _err, code) = run_curl(&[
            "-s",
            "-w",
            "\n%{http_code}",
            "-H",
            &format!("Authorization: Bearer {}", self.token),
            &url,
        ])?;

        let parts: Vec<&str> = out.rsplitn(2, '\n').collect();
        let status_code: i32 = parts
            .first()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(code);
        let body = if parts.len() > 1 { parts[1] } else { "" };

        if status_code < 200 || status_code >= 300 {
            return Err(format!(
                "Google Photos get URL failed ({}): {}",
                status_code, body
            ));
        }

        let resp: serde_json::Value = serde_json::from_str(body)
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        let base_url = resp["baseUrl"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(base_url)
    }

    fn test_connection(&self) -> Result<bool, String> {
        let (out, err, code) = run_curl(&[
            "-s",
            "-w",
            "\n%{http_code}",
            "-H",
            &format!("Authorization: Bearer {}", self.token),
            "https://photoslibrary.googleapis.com/v1/albums?pageSize=1",
        ])?;

        let parts: Vec<&str> = out.rsplitn(2, '\n').collect();
        let status_code: i32 = parts
            .first()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(code);

        if status_code == 200 {
            Ok(true)
        } else {
            Err(format!(
                "Google Photos connection test failed (HTTP {}). {}",
                status_code, err
            ))
        }
    }
}

// ===========================================================================
// URL-encoding helper (simple, no external dependency)
// ===========================================================================

fn urlencoding(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' {
                c.to_string()
            } else {
                format!("%{:02X}", c as u8)
            }
        })
        .collect()
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
    }
}