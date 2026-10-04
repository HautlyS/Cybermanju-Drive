// Cybermanju Drive — quota / usage probing (AGENT-1)
//
// `quota::usage(config)` asks the provider how much room is left. Where an
// endpoint exists (Drive quotaInfo, GitHub rate-limit) the numbers are real;
// where the provider exposes none (Telegram, Photos, local disk) the answer
// is an honest `QuotaUsage` with `None` fields plus a `detail` note — never
// a fabricated number.
//
// <<< AGENT-1 QUOTA >>>

use crate::backends::{gitlab_instance_base, http_client, send_classified, urlencoding};
use crate::oauth;
use crate::rate_limit;
use cybermanju_types::sync::{SyncBackendType, SyncConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QuotaUsage {
    pub backend_type: SyncBackendType,
    /// Storage quota in bytes when the provider publishes one.
    pub total_bytes: Option<u64>,
    /// Storage already consumed in bytes when the provider publishes it.
    pub used_bytes: Option<u64>,
    /// Remaining API requests in the current window (rolling limits only).
    pub remaining_requests: Option<u64>,
    /// Window size for `remaining_requests`.
    pub request_limit: Option<u64>,
    /// Unix seconds at which the request window resets.
    pub reset_at: Option<u64>,
    /// How the numbers were obtained (or why they are unknown).
    pub detail: String,
}

impl QuotaUsage {
    fn unknown(backend_type: SyncBackendType, detail: &str) -> Self {
        Self {
            backend_type,
            total_bytes: None,
            used_bytes: None,
            remaining_requests: None,
            request_limit: None,
            reset_at: None,
            detail: detail.to_string(),
        }
    }

    /// Bytes still available, when both sides of the quota are known.
    pub fn remaining_bytes(&self) -> Option<u64> {
        Some(self.total_bytes?.saturating_sub(self.used_bytes?))
    }
}

/// Probe provider quota/usage for a sync configuration.
pub fn usage(config: &SyncConfig) -> Result<QuotaUsage, String> {
    match config.backend_type {
        SyncBackendType::GoogleDrive => drive_usage(config),
        SyncBackendType::GitHub => github_usage(config),
        SyncBackendType::GitLab => gitlab_usage(config),
        SyncBackendType::GooglePhotos => Ok(QuotaUsage::unknown(
            SyncBackendType::GooglePhotos,
            "Google Photos publishes no storage-quota endpoint (best-effort: unknown)",
        )),
        SyncBackendType::Telegram => Ok(QuotaUsage::unknown(
            SyncBackendType::Telegram,
            "Telegram Bot API publishes no quota endpoint (best-effort: unknown)",
        )),
        SyncBackendType::Local => Ok(QuotaUsage::unknown(
            SyncBackendType::Local,
            "local filesystem quota is not tracked (best-effort: unknown)",
        )),
    }
}

// ─── Google Drive: about?fields=quotaInfo ────────────────────────────

fn drive_usage(config: &SyncConfig) -> Result<QuotaUsage, String> {
    let _permit = rate_limit::acquire(&SyncBackendType::GoogleDrive)?;
    let token = oauth::resolve_token(config)?;
    let client = http_client()?;
    let url = "https://www.googleapis.com/drive/v3/about?fields=user,quotaInfo";

    let resp = send_classified("Google Drive", "quota", &[], || {
        Ok(client
            .get(url)
            .header("Authorization", format!("Bearer {}", token)))
    })?;

    let body = resp
        .text()
        .map_err(|e| format!("network: Google Drive quota response unreadable: {}", e))?;
    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("network: Google Drive quota response unparseable: {}", e))?;

    let quota = &json["quotaInfo"];
    let used = quota["usage"]
        .as_u64()
        .or_else(|| quota["usage"].as_str().and_then(|s| s.parse().ok()));
    let trash = quota["usageInDriveTrash"]
        .as_u64()
        .or_else(|| {
            quota["usageInDriveTrash"]
                .as_str()
                .and_then(|s| s.parse().ok())
        })
        .unwrap_or(0);
    let total = quota["limit"]
        .as_u64()
        .or_else(|| quota["limit"].as_str().and_then(|s| s.parse().ok()));

    let mut usage = QuotaUsage::unknown(SyncBackendType::GoogleDrive, "Drive v3 about/quotaInfo");
    usage.used_bytes = used.map(|u| u.saturating_add(trash));
    usage.total_bytes = total;
    if usage.used_bytes.is_none() && usage.total_bytes.is_none() {
        usage.detail = "Drive returned no quotaInfo for this account".to_string();
    }
    Ok(usage)
}

// ─── GitHub: /rate_limit ─────────────────────────────────────────────

fn github_usage(config: &SyncConfig) -> Result<QuotaUsage, String> {
    let _permit = rate_limit::acquire(&SyncBackendType::GitHub)?;
    let token = oauth::resolve_token(config)?;
    let client = http_client()?;
    let url = "https://api.github.com/rate_limit";

    let resp = send_classified("GitHub", "quota", &[], || {
        Ok(client
            .get(url)
            .header("Authorization", format!("token {}", token))
            .header("Accept", "application/vnd.github+json"))
    })?;

    let body = resp
        .text()
        .map_err(|e| format!("network: GitHub rate-limit response unreadable: {}", e))?;
    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("network: GitHub rate-limit response unparseable: {}", e))?;

    let core = &json["resources"]["core"];
    let mut usage = QuotaUsage::unknown(
        SyncBackendType::GitHub,
        "GitHub REST rate-limit window (storage quota not published)",
    );
    usage.request_limit = core["limit"].as_u64();
    usage.remaining_requests = core["remaining"].as_u64();
    usage.reset_at = core["reset"].as_u64();
    Ok(usage)
}

// ─── GitLab: project statistics (best-effort) ────────────────────────

fn gitlab_usage(config: &SyncConfig) -> Result<QuotaUsage, String> {
    let _permit = rate_limit::acquire(&SyncBackendType::GitLab)?;
    let token = oauth::resolve_token(config)?;
    let project_id = config
        .repo_name
        .as_deref()
        .ok_or("not_found: GitLab backend requires project_id (use repo_name field)")?;

    // Same instance-URL resolution the backend uses (see backends.rs).
    let base = gitlab_instance_base(config);
    let client = http_client()?;
    let url = format!(
        "{}/api/v4/projects/{}?statistics=true",
        base,
        urlencoding(project_id)
    );

    // 401 stays un-allowed so a bad token surfaces as `auth:`; 403/404 mean
    // "statistics not visible to this token" — best-effort, not a failure.
    let resp = send_classified("GitLab", "quota", &[403, 404], || {
        Ok(client.get(&url).header("PRIVATE-TOKEN", &token))
    })?;

    let status = resp.status().as_u16();
    if status != 200 {
        return Ok(QuotaUsage::unknown(
            SyncBackendType::GitLab,
            &format!(
                "project statistics need maintainer access (HTTP {}) — best-effort: unknown",
                status
            ),
        ));
    }

    let body = resp
        .text()
        .map_err(|e| format!("network: GitLab quota response unreadable: {}", e))?;
    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("network: GitLab quota response unparseable: {}", e))?;

    let stats = &json["statistics"];
    let used = stats["storage_size"]
        .as_u64()
        .or_else(|| stats["repository_size"].as_u64());
    let mut usage = QuotaUsage::unknown(
        SyncBackendType::GitLab,
        "project statistics.storage_size (project storage has no provider quota)",
    );
    usage.used_bytes = used;
    Ok(usage)
}
