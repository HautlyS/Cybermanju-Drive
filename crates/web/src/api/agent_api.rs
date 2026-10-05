// Cybermanju Drive — native AI agent runtime (shared by Tauri IPC and REST)
//
// Configs (keyless rows, sealed keys) → sessions (transcripts) → detached
// jobs (`prompt_async` → 202-style job, poll `jobs/{id}`) with ask-approval
// parking, abort, and per-call permission decisions. Tool execution runs
// against the host filesystem under the config working root (contained,
// audited). The pure loop math (providers, permissions, protocol, anchored
// edits) lives in `cybermanju-agent`; this module owns threads, locks and
// the database.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::{Duration, Instant};

use cybermanju_agent::{agent_loop, config as agent_config, edit as agent_edit, protocol, providers};
use cybermanju_db::Database;
use cybermanju_types::agent::{
    AgentConfig, AgentKind, AgentSession, ChatMessage, TokenUsage, ToolCall,
};
use redb::ReadableTable;
use serde::{Deserialize, Serialize};

// ─── tunables ────────────────────────────────────────────────────────────

/// How long an `ask` waits for the UI before auto-denying.
const APPROVAL_TIMEOUT_SECS: u64 = 600;
/// Single tool output cap (the model drowns past this).
const TOOL_OUTPUT_CAP: usize = 65_536;
/// Read/write/edit size cap (mirrors the editor).
const MAX_TOOL_BYTES: usize = 1024 * 1024;
/// Grep bounds.
const MAX_GREP_MATCHES: usize = 50;
const MAX_GREP_FILES: usize = 500;
/// Directory listing cap.
const MAX_LIST_ENTRIES: usize = 200;
/// Repo-overview cap for the system prompt.
const OVERVIEW_CAP: usize = 200;
/// Nested `task` runs get a short leash.
const SUBAGENT_MAX_TURNS: u32 = 5;

// ─── wire types ──────────────────────────────────────────────────────────

/// One parked approval (or question) awaiting the UI.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingApproval {
    pub tool: String,
    pub input: serde_json::Value,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
}

/// Pollable job snapshot (202-style lifecycle, like sync jobs).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobSnapshot {
    pub job_id: String,
    pub session_id: String,
    pub config_id: String,
    pub status: String,
    pub turns_used: u32,
    pub max_turns: u32,
    pub usage: TokenUsage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending: Option<PendingApproval>,
}

// ─── secrets (never serialized) ──────────────────────────────────────────

fn agent_key_name(config_id: &str) -> String {
    format!("agent:key:{config_id}")
}

fn load_key(db: &Database, config_id: &str) -> Result<String, String> {
    crate::security::validate_id(config_id)?;
    Ok(db
        .get_sync_secret(&agent_key_name(config_id))
        .map_err(|e| e.to_string())?
        .unwrap_or_default())
}

// ─── config CRUD ─────────────────────────────────────────────────────────

fn with_has_key(db: &Database, mut config: AgentConfig) -> Result<AgentConfig, String> {
    let key = db
        .get_sync_secret(&agent_key_name(&config.id))
        .map_err(|e| e.to_string())?;
    config.has_key = key.as_ref().map(|k| !k.is_empty()).unwrap_or(false);
    Ok(config)
}

/// List all agent configs (keys never included — `hasKey` only).
pub fn list_configs(db: &Database) -> Result<Vec<AgentConfig>, String> {
    let tx = db.begin_read().map_err(|e| e.to_string())?;
    let table = tx
        .open_table(Database::get_agent_configs_table())
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for entry in table.iter().map_err(|e| e.to_string())? {
        let (_, value) = entry.map_err(|e| e.to_string())?;
        let config: AgentConfig =
            serde_json::from_str(value.value()).map_err(|e| e.to_string())?;
        out.push(with_has_key(db, config)?);
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Load one config by id.
pub fn get_config(db: &Database, config_id: &str) -> Result<AgentConfig, String> {
    crate::security::validate_id(config_id)?;
    let tx = db.begin_read().map_err(|e| e.to_string())?;
    let table = tx
        .open_table(Database::get_agent_configs_table())
        .map_err(|e| e.to_string())?;
    let value = table
        .get(config_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Agent config not found: {}", config_id))?;
    let config: AgentConfig = serde_json::from_str(value.value()).map_err(|e| e.to_string())?;
    with_has_key(db, config)
}

fn validate_config(config: &AgentConfig) -> Result<(), String> {
    if config.name.trim().is_empty() {
        return Err("invalid: name is required".to_string());
    }
    if config.model.trim().is_empty() {
        return Err("invalid: model is required".to_string());
    }
    if config.working_dir.contains("..") {
        return Err("invalid: working_dir must not contain '..'".to_string());
    }
    // Merges endpoint + dialect; fails on unknown providers and keyless
    // custom configs alike.
    providers::resolve(config)?;
    Ok(())
}

/// Create or overwrite a config. The row never holds key material.
pub fn save_config(db: &Database, mut config: AgentConfig) -> Result<AgentConfig, String> {
    if config.id.is_empty() {
        config.id = uuid::Uuid::new_v4().to_string();
    }
    crate::security::validate_id(&config.id)?;
    validate_config(&config)?;
    config.max_turns = config.max_turns.clamp(1, agent_loop::MAX_TURNS_HARD_CAP);
    let now = chrono::Utc::now().to_rfc3339();
    if config.created_at.is_empty() {
        // Preserve the original timestamp on updates.
        let keep = db
            .begin_read()
            .ok()
            .and_then(|tx| tx.open_table(Database::get_agent_configs_table()).ok())
            .and_then(|table| table.get(config.id.as_str()).ok().flatten())
            .and_then(|v| serde_json::from_str::<AgentConfig>(v.value()).ok())
            .map(|old| old.created_at)
            .unwrap_or_default();
        config.created_at = if keep.is_empty() { now.clone() } else { keep };
    }
    config.updated_at = now;
    config.has_key = false; // computed on read; never persisted as true
    let serialized = serde_json::to_string(&config).map_err(|e| e.to_string())?;
    let tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = tx
            .open_table(Database::get_agent_configs_table())
            .map_err(|e| e.to_string())?;
        table
            .insert(config.id.as_str(), serialized.as_str())
            .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    with_has_key(db, config)
}

/// Delete a config and its sealed key.
pub fn delete_config(db: &Database, config_id: &str) -> Result<bool, String> {
    crate::security::validate_id(config_id)?;
    let tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = tx
            .open_table(Database::get_agent_configs_table())
            .map_err(|e| e.to_string())?;
        let removed = table
            .remove(config_id)
            .map_err(|e| e.to_string())?
            .is_some();
        if !removed {
            return Err(format!("Agent config not found: {}", config_id));
        }
        let mut secrets = tx
            .open_table(Database::get_sync_secrets_table())
            .map_err(|e| e.to_string())?;
        let _ = secrets
            .remove(agent_key_name(config_id).as_str())
            .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(true)
}

/// Store (seal) a provider key. Empty keys are rejected — clearing happens
/// via config delete. 4 KiB cap keeps accidental pastes sane.
pub fn save_key(db: &Database, config_id: &str, api_key: &str) -> Result<bool, String> {
    crate::security::validate_id(config_id)?;
    get_config(db, config_id)?; // 404 on unknown configs
    if api_key.trim().is_empty() {
        return Err("invalid: apiKey is required".to_string());
    }
    if api_key.len() > 4096 {
        return Err("invalid: apiKey looks pasted-wrong (over 4 KiB)".to_string());
    }
    db.put_sync_secret(&agent_key_name(config_id), api_key.trim())
        .map_err(|e| e.to_string())?;
    Ok(true)
}

/// Refresh the model list from the provider (OpenAI dialect only —
/// Anthropic has no list API, answered honestly as `unsupported:`).
pub fn list_models(db: &Database, config_id: &str) -> Result<Vec<String>, String> {
    let config = get_config(db, config_id)?;
    let endpoint = providers::resolve(&config)?;
    let models_url = providers::models_url(&endpoint.base_url, endpoint.dialect).ok_or_else(|| {
        format!(
            "unsupported: provider '{}' has no models endpoint — enter the model id manually",
            config.provider_id
        )
    })?;
    let key = load_key(db, config_id)?;
    let headers = endpoint_headers(&endpoint, &key);
    let body = protocol::get_json(&models_url, &headers)?;
    if let Some(data) = body.get("data").and_then(|d| d.as_array()) {
        let mut models: Vec<String> = data
            .iter()
            .filter_map(|m| m.get("id").and_then(|id| id.as_str()).map(str::to_string))
            .collect();
        models.sort();
        models.dedup();
        return Ok(models);
    }
    Err("integrity: provider models reply had no data[]".to_string())
}

// ─── sessions ────────────────────────────────────────────────────────────

fn save_session_row(db: &Database, session: &AgentSession) -> Result<(), String> {
    let serialized = serde_json::to_string(session).map_err(|e| e.to_string())?;
    let tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = tx
            .open_table(Database::get_agent_sessions_table())
            .map_err(|e| e.to_string())?;
        table
            .insert(session.id.as_str(), serialized.as_str())
            .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

/// Newest-first session list (capped — transcripts can be large).
pub fn list_sessions(db: &Database) -> Result<Vec<AgentSession>, String> {
    let tx = db.begin_read().map_err(|e| e.to_string())?;
    let table = tx
        .open_table(Database::get_agent_sessions_table())
        .map_err(|e| e.to_string())?;
    let mut rows = Vec::new();
    for entry in table.iter().map_err(|e| e.to_string())? {
        let (_, value) = entry.map_err(|e| e.to_string())?;
        rows.push(serde_json::from_str::<AgentSession>(value.value()).map_err(|e| e.to_string())?);
    }
    rows.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    rows.truncate(100);
    Ok(rows)
}

/// Load one session (export-compatible JSON).
pub fn get_session(db: &Database, session_id: &str) -> Result<AgentSession, String> {
    crate::security::validate_id(session_id)?;
    let tx = db.begin_read().map_err(|e| e.to_string())?;
    let table = tx
        .open_table(Database::get_agent_sessions_table())
        .map_err(|e| e.to_string())?;
    let value = table
        .get(session_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Agent session not found: {}", session_id))?;
    serde_json::from_str(value.value()).map_err(|e| e.to_string())
}

/// Delete a session and its transcript.
pub fn delete_session(db: &Database, session_id: &str) -> Result<bool, String> {
    crate::security::validate_id(session_id)?;
    let tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = tx
            .open_table(Database::get_agent_sessions_table())
            .map_err(|e| e.to_string())?;
        let removed = table
            .remove(session_id)
            .map_err(|e| e.to_string())?
            .is_some();
        if !removed {
            return Err(format!("Agent session not found: {}", session_id));
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(true)
}

/// Create a session bound to a config snapshot.
pub fn create_session(
    db: &Database,
    config_id: &str,
    title: Option<String>,
) -> Result<AgentSession, String> {
    let config = get_config(db, config_id)?;
    let now = chrono::Utc::now().to_rfc3339();
    let session = AgentSession {
        id: uuid::Uuid::new_v4().to_string(),
        title: title
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| "Untitled session".to_string()),
        config_id: config.id.clone(),
        provider_id: config.provider_id.clone(),
        model: config.model.clone(),
        agent_kind: config.agent_kind,
        working_dir: config.working_dir.clone(),
        messages: Vec::new(),
        usage: TokenUsage::default(),
        created_at: now.clone(),
        updated_at: now,
    };
    save_session_row(db, &session)?;
    Ok(session)
}

/// Import a session transcript (always re-keyed — never trust a client id).
pub fn import_session(db: &Database, mut session: AgentSession) -> Result<AgentSession, String> {
    if session.messages.len() > 2000 {
        return Err("invalid: session transcript is too large".to_string());
    }
    let now = chrono::Utc::now().to_rfc3339();
    session.id = uuid::Uuid::new_v4().to_string();
    session.updated_at = now.clone();
    if session.created_at.is_empty() {
        session.created_at = now;
    }
    save_session_row(db, &session)?;
    Ok(session)
}

// ─── job registry ────────────────────────────────────────────────────────

struct JobState {
    status: String,
    turns_used: u32,
    max_turns: u32,
    usage: TokenUsage,
    result: Option<String>,
    error: Option<String>,
    pending: Option<PendingApproval>,
}

pub struct AgentJob {
    pub job_id: String,
    pub session_id: String,
    pub config_id: String,
    pub started_at: String,
    state: Mutex<JobState>,
    cancel: AtomicBool,
}

#[derive(Debug, Clone)]
struct ApprovalAnswer {
    approved: bool,
    answer: Option<String>,
}

static JOBS: OnceLock<Mutex<HashMap<String, Arc<AgentJob>>>> = OnceLock::new();
static APPROVALS: OnceLock<Mutex<HashMap<String, ApprovalAnswer>>> = OnceLock::new();

fn jobs() -> &'static Mutex<HashMap<String, Arc<AgentJob>>> {
    JOBS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn approvals() -> &'static Mutex<HashMap<String, ApprovalAnswer>> {
    APPROVALS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn snapshot(job: &AgentJob) -> JobSnapshot {
    let state = job.state.lock().unwrap_or_else(|p| p.into_inner());
    JobSnapshot {
        job_id: job.job_id.clone(),
        session_id: job.session_id.clone(),
        config_id: job.config_id.clone(),
        status: state.status.clone(),
        turns_used: state.turns_used,
        max_turns: state.max_turns,
        usage: state.usage.clone(),
        result: state.result.clone(),
        error: state.error.clone(),
        pending: state.pending.clone(),
    }
}

fn set_state(job: &AgentJob, f: impl FnOnce(&mut JobState)) {
    match job.state.lock() {
        Ok(mut state) => f(&mut state),
        Err(poisoned) => f(&mut poisoned.into_inner()),
    }
}

// ─── endpoints, headers, working root ────────────────────────────────────

fn set_state(job: &AgentJob, f: impl FnOnce(&mut JobState)) {
    match job.state.lock() {
        Ok(mut state) => f(&mut state),
        Err(poisoned) => f(&mut poisoned.into_inner()),
    }
}

/// Headers for a chat call from a resolved endpoint. Mirrors
/// `protocol::auth_headers` without needing a catalog preset (custom
/// providers have none).
fn endpoint_headers(endpoint: &providers::ResolvedEndpoint, api_key: &str) -> Vec<(String, String)> {
    use cybermanju_types::agent::AuthScheme;
    let mut headers = endpoint.extra_headers.clone();
    match endpoint.auth {
        AuthScheme::Bearer => {
            if !api_key.is_empty() {
                headers.push(("Authorization".into(), format!("Bearer {api_key}")));
            }
        }
        AuthScheme::Header => {
            let name = endpoint
                .auth_name
                .clone()
                .unwrap_or_else(|| "x-api-key".into());
            headers.push((name, api_key.to_string()));
        }
        AuthScheme::Query | AuthScheme::None => {}
    }
    headers.push(("Content-Type".into(), "application/json".into()));
    headers
}

/// The host directory tools run against: volume root + config working dir.
fn working_root(config: &AgentConfig) -> Result<PathBuf, String> {
    let vol = cybermanju_os::api::Kernel::global().root().to_path_buf();
    let w = config.working_dir.trim().trim_matches('/');
    if w.is_empty() {
        return Ok(vol);
    }
    Ok(vol.join(w))
}

/// Lexically normalize path parts (no symlink resolution — documented).
fn normalize_join(base: &Path, user: &str) -> PathBuf {
    let mut out = base.to_path_buf();
    for part in user.split(['/', '\\']) {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            out.pop();
            continue;
        }
        out.push(part);
    }
    out
}

/// Join a tool path onto a root, refusing escapes. Leading `/` means the
/// volume root (cybsh convention); anything else is working-dir-relative.
fn join_contained(root: &Path, vol: &Path, user: &str) -> Result<PathBuf, String> {
    let trimmed = user.trim();
    if trimmed.is_empty() {
        return Err("invalid: empty path".to_string());
    }
    // Leading `/` addresses the volume root (cybsh convention), anything
    // else is working-dir-relative. Either way the result must stay inside
    // the volume; relative paths must additionally stay inside the root.
    let absolute = trimmed.starts_with('/');
    let path = normalize_join(if absolute { vol } else { root }, trimmed);
    if path != *vol && !path.starts_with(vol) {
        return Err("deny: path escapes the volume".to_string());
    }
    if !absolute && path != *root && !path.starts_with(root) {
        return Err("deny: path escapes the working root".to_string());
    }
    Ok(path)
}

fn volume_root() -> PathBuf {
    cybermanju_os::api::Kernel::global().root().to_path_buf()
}

// ─── tool executors (native) ─────────────────────────────────────────────

fn tool_read(root: &Path, vol: &Path, path: &str) -> Result<String, String> {
    let full = join_contained(root, vol, path)?;
    let bytes = std::fs::read(&full)
        .map_err(|_| format!("not_found: '{}' does not exist", path.trim()))?;
    if bytes.len() > MAX_TOOL_BYTES {
        return Err(format!(
            "too_large: '{}' is {} bytes, tool limit is {}",
            path.trim(),
            bytes.len(),
            MAX_TOOL_BYTES
        ));
    }
    String::from_utf8(bytes)
        .map_err(|_| format!("binary: '{}' is not valid UTF-8 text", path.trim()))
}

fn tool_list(root: &Path, vol: &Path, path: &str) -> Result<String, String> {
    let dir = if path.trim().is_empty() || path.trim() == "/" {
        if path.trim().starts_with('/') {
            vol.to_path_buf()
        } else {
            root.to_path_buf()
        }
    } else {
        join_contained(root, vol, path)?
    };
    let entries = std::fs::read_dir(&dir)
        .map_err(|_| format!("not_found: '{}' is not a directory", path.trim()))?;
    let mut names = Vec::new();
    for entry in entries.flatten().take(MAX_LIST_ENTRIES + 1) {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let suffix = if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            "/"
        } else {
            ""
        };
        names.push(format!("{name}{suffix}"));
    }
    names.sort();
    let mut out = names.into_iter().take(MAX_LIST_ENTRIES).collect::<Vec<_>>().join("\n");
    if out.is_empty() {
        out.push_str("(empty directory)");
    }
    Ok(out)
}

fn tool_grep(root: &Path, vol: &Path, pattern: &str, sub: &str, limit: usize) -> Result<String, String> {
    if pattern.is_empty() {
        return Err("invalid: pattern is required".to_string());
    }
    let base = if sub.trim().is_empty() {
        root.to_path_buf()
    } else {
        join_contained(root, vol, sub)?
    };
    const SKIP_DIRS: &[&str] = &[".git", "node_modules", "target", "dist", "build", ".hg", ".svn"];
    let limit = limit.clamp(1, MAX_GREP_MATCHES);
    let mut matches = Vec::new();
    let mut files_seen = 0usize;
    let mut stack = vec![base];
    while let Some(dir) = stack.pop() {
        if matches.len() >= limit || files_seen >= MAX_GREP_FILES {
            break;
        }
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            if matches.len() >= limit || files_seen >= MAX_GREP_FILES {
                break;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || SKIP_DIRS.contains(&name.as_str()) {
                continue;
            }
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            files_seen += 1;
            let bytes = match std::fs::read(&path) {
                Ok(bytes) => bytes,
                Err(_) => continue,
            };
            if bytes.len() > MAX_TOOL_BYTES || bytes.contains(&0) {
                continue;
            }
            let text = match String::from_utf8(bytes) {
                Ok(text) => text,
                Err(_) => continue,
            };
            for (i, line) in text.lines().enumerate() {
                if line.contains(pattern) {
                    let rel = path.strip_prefix(root).unwrap_or(&path).to_string_lossy();
                    let snippet: String = line.trim().chars().take(240).collect();
                    matches.push(format!("{}:{}: {}", rel, i + 1, snippet));
                    if matches.len() >= limit {
                        break;
                    }
                }
            }
        }
    }
    if matches.is_empty() {
        return Ok(format!("no matches for `{pattern}`"));
    }
    Ok(matches.join("\n"))
}

fn tool_write(
    db: &Database,
    root: &Path,
    vol: &Path,
    path: &str,
    content: &str,
) -> Result<String, String> {
    if content.len() > MAX_TOOL_BYTES {
        return Err(format!(
            "too_large: content is {} bytes, tool limit is {}",
            content.len(),
            MAX_TOOL_BYTES
        ));
    }
    let full = join_contained(root, vol, path)?;
    if let Some(parent) = full.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("not_found: cannot create '{}': {}", parent.display(), e))?;
        }
    }
    let existed = full.exists();
    std::fs::write(&full, content.as_bytes())
        .map_err(|e| format!("not_found: cannot write '{}': {}", path.trim(), e))?;
    let hash = agent_edit::blake3_hex(content.as_bytes());
    let _ = db.log_audit(
        "agent_write",
        "file",
        path.trim(),
        None,
        Some(serde_json::json!({ "bytes": content.len(), "existed": existed, "hash": hash })),
    );
    Ok(format!(
        "wrote {} ({} bytes, blake3:{})",
        path.trim(),
        content.len(),
        &hash[..16.min(hash.len())]
    ))
}

fn tool_bash(root: &Path, command: &str, timeout_secs: u64) -> Result<String, String> {
    use std::process::{Command, Stdio};
    let command = command.trim();
    if command.is_empty() {
        return Err("invalid: empty command".to_string());
    }
    let timeout = timeout_secs.clamp(5, 600);
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("error: cannot spawn shell: {e}"))?;
    // Drain both pipes on threads: a chatty child would otherwise block on
    // a full pipe buffer while the parent only polls for exit.
    let stdout = child.stdout.take().map(|mut out| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            use std::io::Read;
            let _ = out.read_to_end(&mut buf);
            buf
        })
    });
    let stderr = child.stderr.take().map(|mut err| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            use std::io::Read;
            let _ = err.read_to_end(&mut buf);
            buf
        })
    });
    let deadline = Instant::now() + Duration::from_secs(timeout);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("timeout: command killed after {timeout}s"));
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return Err(format!("error: waiting on shell: {e}")),
        }
    };
    let mut out = stdout
        .and_then(|t| t.join().ok())
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default();
    let err = stderr
        .and_then(|t| t.join().ok())
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default();
    if !err.trim().is_empty() {
        out.push_str("\n--- stderr ---\n");
        out.push_str(&err);
    }
    if out.len() > TOOL_OUTPUT_CAP {
        out.truncate(TOOL_OUTPUT_CAP);
        out.push_str("\n… truncated at 64 KiB");
    }
    let code = status.code().map(|c| c.to_string()).unwrap_or_else(|| "signal".into());
    Ok(format!("exit {code}\n{out}"))
}

/// Dispatch one approved tool call to native execution.
fn exec_tool(
    db: &Database,
    root: &Path,
    vol: &Path,
    call: &ToolCall,
) -> Result<String, String> {
    let get = |key: &str| call.input.get(key).and_then(|v| v.as_str()).unwrap_or("").to_string();
    match call.name.as_str() {
        "read" => tool_read(root, vol, &get("path")),
        "list" => tool_list(root, vol, &get("path")),
        "grep" => {
            let limit = call
                .input
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(MAX_GREP_MATCHES as u64) as usize;
            tool_grep(root, vol, &get("pattern"), &get("path"), limit)
        }
        "write" => tool_write(db, root, vol, &get("path"), call.input.get("content").and_then(|v| v.as_str()).unwrap_or("")),
        "edit" => {
            let path = get("path");
            let old_block = call.input.get("old_block").and_then(|v| v.as_str()).unwrap_or("");
            let new_block = call.input.get("new_block").and_then(|v| v.as_str()).unwrap_or("");
            let expected = call.input.get("expected_hash").and_then(|v| v.as_str());
            let current = tool_read(root, vol, &path)?;
            let updated = agent_edit::apply_edit(&current, old_block, new_block, expected)?;
            tool_write(db, root, vol, &path, &updated)?;
            Ok(format!(
                "edited {} (blake3:{})",
                path.trim(),
                &agent_edit::blake3_hex(updated.as_bytes())[..16]
            ))
        }
        "bash" => {
            let timeout = call
                .input
                .get("timeout_secs")
                .and_then(|v| v.as_u64())
                .unwrap_or(120);
            tool_bash(root, &get("command"), timeout)
        }
        other => Err(format!("unsupported: unknown tool '{other}'")),
    }
}

// ─── jobs (detached runs) ─────────────────────────────────────────────────

/// Start an agent run on a worker thread; returns immediately with a job
/// snapshot (the REST `202`-style path — the request thread never waits on
/// a provider, same contract as `POST /api/sync/start`).
pub fn start_job(
    db: &Arc<RwLock<Database>>,
    config_id: &str,
    session_id: Option<String>,
    prompt: String,
) -> Result<JobSnapshot, String> {
    if prompt.trim().is_empty() {
        return Err("invalid: prompt is required".to_string());
    }
    if prompt.len() > 64 * 1024 {
        return Err("invalid: prompt is too long (64 KiB cap)".to_string());
    }
    // Validate everything on the request thread: bad configs are 4xx, not
    // jobs that fail in the dark.
    let config = {
        let guard = db.read().map_err(|e| e.to_string())?;
        get_config(&guard, config_id)?
    };
    let key = {
        let guard = db.read().map_err(|e| e.to_string())?;
        let key = load_key(&guard, config_id)?;
        if key.is_empty() && !config.provider_id.is_empty() {
            let endpoint = providers::resolve(&config)?;
            if !endpoint.keyless {
                return Err("auth: no API key saved for this config — add one first".to_string());
            }
        }
        key
    };
    let session = {
        let guard = db.read().map_err(|e| e.to_string())?;
        match session_id {
            Some(id) => {
                crate::security::validate_id(&id)?;
                get_session(&guard, &id)?
            }
            None => {
                drop(guard);
                let guard = db.read().map_err(|e| e.to_string())?;
                let title: String = prompt
                    .trim()
                    .chars()
                    .take(60)
                    .collect::<String>()
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                create_session(&guard, config_id, Some(title))?
            }
        }
    };

    let job = Arc::new(AgentJob {
        job_id: format!("agent-{}", uuid::Uuid::new_v4()),
        session_id: session.id.clone(),
        config_id: config.id.clone(),
        started_at: chrono::Utc::now().to_rfc3339(),
        state: Mutex::new(JobState {
            status: "running".to_string(),
            turns_used: 0,
            max_turns: config.max_turns,
            usage: TokenUsage::default(),
            result: None,
            error: None,
            pending: None,
        }),
        cancel: AtomicBool::new(false),
    });
    {
        let mut registry = jobs().lock().unwrap_or_else(|p| p.into_inner());
        registry.insert(job.job_id.clone(), Arc::clone(&job));
    }

    let db2 = Arc::clone(db);
    let job2 = Arc::clone(&job);
    let spawned = std::thread::Builder::new()
        .name(format!("agent-{}", job.job_id))
        .spawn(move || {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                run_agent_job(&db2, &job2, config, key, session, prompt)
            }));
            if outcome.is_err() {
                set_state(&job2, |s| {
                    s.status = "error".to_string();
                    s.error = Some("agent worker panicked".to_string());
                    s.pending = None;
                });
            }
        });
    if let Err(e) = spawned {
        set_state(&job, |s| {
            s.status = "error".to_string();
            s.error = Some(format!("could not start agent worker: {e}"));
        });
        return Err(format!("could not start agent worker: {e}"));
    }
    Ok(snapshot(&job))
}

/// Poll one job.
pub fn job_status(job_id: &str) -> Result<JobSnapshot, String> {
    crate::security::validate_id(job_id)?;
    let registry = jobs().lock().unwrap_or_else(|p| p.into_inner());
    registry
        .get(job_id)
        .map(|job| snapshot(job))
        .ok_or_else(|| format!("Agent job not found: {}", job_id))
}

/// All known jobs (capped; registry order is not chronological — the UI
/// sorts by recency from status polls).
pub fn list_jobs() -> Vec<JobSnapshot> {
    let registry = jobs().lock().unwrap_or_else(|p| p.into_inner());
    let mut out: Vec<JobSnapshot> = registry.values().map(|job| snapshot(job)).collect();
    out.sort_by(|a, b| b.job_id.cmp(&a.job_id));
    out.truncate(50);
    out
}

/// Request cancellation. Idempotent: unknown ids are 404, finished jobs
/// report success without side effects.
pub fn abort_job(job_id: &str) -> Result<bool, String> {
    let job = {
        let registry = jobs().lock().unwrap_or_else(|p| p.into_inner());
        registry
            .get(job_id)
            .cloned()
            .ok_or_else(|| format!("Agent job not found: {}", job_id))?
    };
    job.cancel.store(true, Ordering::SeqCst);
    Ok(true)
}

/// Answer a parked approval (`approved`) or question (`answer`).
/// Returns false when nothing is waiting — the UI polls, so a stale tap
/// must not error loudly.
pub fn approve_job(job_id: &str, approved: bool, answer: Option<String>) -> Result<bool, String> {
    crate::security::validate_id(job_id)?;
    {
        let registry = jobs().lock().unwrap_or_else(|p| p.into_inner());
        if !registry.contains_key(job_id) {
            return Err(format!("Agent job not found: {}", job_id));
        }
    }
    let mut pending = approvals().lock().unwrap_or_else(|p| p.into_inner());
    pending.insert(
        job_id.to_string(),
        ApprovalAnswer { approved, answer },
    );
    Ok(true)
}

fn wait_approval(job: &AgentJob) -> Option<ApprovalAnswer> {
    let deadline = Instant::now() + Duration::from_secs(APPROVAL_TIMEOUT_SECS);
    loop {
        if job.cancel.load(Ordering::SeqCst) {
            return None;
        }
        if let Ok(mut pending) = approvals().lock() {
            if let Some(answer) = pending.remove(&job.job_id) {
                return Some(answer);
            }
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

// ─── worker ────────────────────────────────────────────────────────────────

/// Repo overview for the system prompt: top-level working-root listing.
fn repo_overview(root: &Path) -> String {
    match tool_list(root, root, "") {
        Ok(listing) => {
            let entries: Vec<String> = listing.lines().take(OVERVIEW_CAP).map(str::to_string).collect();
            agent_loop::repo_overview_snippet(&entries, OVERVIEW_CAP)
        }
        Err(_) => "(working root is not readable)".to_string(),
    }
}

fn persist_turn(db: &Arc<RwLock<Database>>, session: &AgentSession) {
    if let Ok(guard) = db.read() {
        let mut session = session.clone();
        session.updated_at = chrono::Utc::now().to_rfc3339();
        let _ = save_session_row(&guard, &session);
    }
}

/// The blocking agent loop. Every provider call, tool run and approval wait
/// funnels through cancel checks; the transcript persists after each turn.
fn run_agent_job(
    db: &Arc<RwLock<Database>>,
    job: &Arc<AgentJob>,
    config: AgentConfig,
    api_key: String,
    mut session: AgentSession,
    prompt: String,
) {
    let fail = |job: &Arc<AgentJob>, message: String| {
        set_state(job, |s| {
            s.status = "error".to_string();
            s.error = Some(message);
            s.pending = None;
        });
    };

    let endpoint = match providers::resolve(&config) {
        Ok(endpoint) => endpoint,
        Err(e) => {
            fail(job, e);
            return;
        }
    };
    let vol = volume_root();
    let root = match working_root(&config) {
        Ok(root) => root,
        Err(e) => {
            fail(job, e);
            return;
        }
    };
    if !root.exists() {
        fail(
            job,
            format!("not_found: working root '{}' does not exist", root.display()),
        );
        return;
    }

    // Seed the user turn from the queued prompt.
    session.messages.push(ChatMessage {
        role: "user".into(),
        content: prompt,
        tool_call_id: None,
        tool_name: None,
        tool_input: None,
    });
    persist_turn(db, &session);
    let mut turn =
        agent_loop::AgentTurn::new(session.messages.clone(), config.max_turns, 0);
    let kind = match config.agent_kind {
        AgentKind::Build => "build",
        AgentKind::Plan => "plan",
    };
    let system = agent_loop::system_prompt(
        &root.to_string_lossy(),
        kind,
        &repo_overview(&root),
    );
    let headers = endpoint_headers(&endpoint, &api_key);
    let model = config.model.clone();

    loop {
        if job.cancel.load(Ordering::SeqCst) {
            set_state(job, |s| {
                s.status = "cancelled".to_string();
                s.pending = None;
            });
            break;
        }
        let (mut url, headers, body) =
            turn.build_request(&endpoint.base_url, endpoint.dialect, &model, &system, headers.clone(), true);
        if endpoint.auth == cybermanju_types::agent::AuthScheme::Query {
            let name = endpoint.auth_name.as_deref().unwrap_or("key");
            url = protocol::with_query_key(&url, name, &api_key);
        }
        let reply = match protocol::post_json(&url, &headers, &body) {
            Ok(reply) => reply,
            Err(e) => {
                fail(job, e);
                break;
            }
        };
        let event = match turn.ingest_reply(endpoint.dialect, &reply) {
            Ok(event) => event,
            Err(e) => {
                fail(job, e);
                break;
            }
        };
        session.messages = turn.messages.clone();
        session.usage = turn.usage.clone();
        persist_turn(db, &session);
        set_state(job, |s| {
            s.turns_used = turn.turns_used;
            s.usage = turn.usage.clone();
        });

        match event {
            agent_loop::LoopEvent::TextDone => {
                let text = turn
                    .messages
                    .iter()
                    .rev()
                    .find(|m| m.role == "assistant")
                    .map(|m| m.content.clone())
                    .unwrap_or_default();
                let short: String = text.chars().take(4000).collect();
                set_state(job, |s| {
                    s.status = "done".to_string();
                    s.result = Some(short);
                });
                break;
            }
            agent_loop::LoopEvent::LimitReached => {
                set_state(job, |s| {
                    s.status = "done".to_string();
                    s.result = Some(format!(
                        "turn budget exhausted after {} turns — last state saved",
                        turn.turns_used
                    ));
                });
                break;
            }
            agent_loop::LoopEvent::ToolCalls(calls) => {
                let mut stop = false;
                for call in &calls {
                    if job.cancel.load(Ordering::SeqCst) {
                        stop = true;
                        break;
                    }
                    match run_one_tool(db, job, &config, &root, &vol, &turn, call) {
                        ToolOutcome::Continue(output) => {
                            turn.append_tool_result(call, output);
                        }
                        ToolOutcome::Stop => {
                            stop = true;
                            break;
                        }
                    }
                }
                session.messages = turn.messages.clone();
                session.usage = turn.usage.clone();
                persist_turn(db, &session);
                if stop {
                    break;
                }
            }
        }
    }

    // Final transcript + usage write (best effort — the run already ended).
    session.messages = turn.messages.clone();
    session.usage = turn.usage.clone();
    persist_turn(db, &session);
    let _ = db.read().map(|guard| {
        let _ = guard.log_audit(
            "agent_run",
            "agent_session",
            &session.id,
            None,
            Some(serde_json::json!({ "turns": turn.turns_used })),
        );
    });
}

enum ToolOutcome {
    /// Keep looping with this tool output appended.
    Continue(String),
    /// Stop the run (cancelled, fatal denial, finished subagent edge).
    Stop,
}

/// Permission-check, approval-park and execute one tool call.
#[allow(clippy::too_many_arguments)]
fn run_one_tool(
    db: &Arc<RwLock<Database>>,
    job: &Arc<AgentJob>,
    config: &AgentConfig,
    root: &Path,
    vol: &Path,
    _turn: &agent_loop::AgentTurn,
    call: &ToolCall,
) -> ToolOutcome {
    // Nested subagents run a bounded inline loop — no registry, no parking.
    if call.name == "task" {
        return run_subagent(db, job, config, root, vol, turn, call);
    }

    let decision = agent_config::decide(&config.permission, config.agent_kind, &call.name, &call.input);
    match decision {
        agent_config::PermissionDecision::Allow => {}
        agent_config::PermissionDecision::Deny { reason } => {
            return ToolOutcome::Continue(format!("{reason} — adjust the permission ruleset to allow it"));
        }
        agent_config::PermissionDecision::Ask { summary } => {
            if config.auto_approve {
                // Auto mode approves asks, never denies.
            } else {
                let question = if call.name == "question" {
                    call.input
                        .get("question")
                        .and_then(|q| q.as_str())
                        .map(str::to_string)
                } else {
                    None
                };
                set_state(job, |s| {
                    s.status = "waiting_approval".to_string();
                    s.pending = Some(PendingApproval {
                        tool: call.name.clone(),
                        input: call.input.clone(),
                        summary: summary.clone(),
                        question,
                    });
                });
                match wait_approval(job) {
                    Some(answer) if answer.approved => {
                        set_state(job, |s| {
                            s.status = "running".to_string();
                            s.pending = None;
                        });
                        if call.name == "question" {
                            let text = answer
                                .answer
                                .filter(|a| !a.trim().is_empty())
                                .unwrap_or_else(|| "approved without comment".to_string());
                            return ToolOutcome::Continue(format!("user answered: {text}"));
                        }
                    }
                    Some(_) => {
                        set_state(job, |s| {
                            s.status = "running".to_string();
                            s.pending = None;
                        });
                        return ToolOutcome::Continue(format!(
                            "denied: user rejected `{}` — work around it or explain",
                            call.name
                        ));
                    }
                    None => {
                        if job.cancel.load(Ordering::SeqCst) {
                            set_state(job, |s| {
                                s.status = "cancelled".to_string();
                                s.pending = None;
                            });
                            return ToolOutcome::Stop;
                        }
                        set_state(job, |s| {
                            s.status = "running".to_string();
                            s.pending = None;
                        });
                        return ToolOutcome::Continue(format!(
                            "denied: approval for `{}` timed out after {}s",
                            call.name, APPROVAL_TIMEOUT_SECS
                        ));
                    }
                }
            }
        }
    }

    if call.name == "question" {
        // Auto mode cannot answer questions — say so honestly.
        return ToolOutcome::Continue("declined: auto-approve cannot answer questions".to_string());
    }

    let guard = match db.read() {
        Ok(guard) => guard,
        Err(e) => return ToolOutcome::Continue(format!("error: database unavailable: {e}")),
    };
    match exec_tool(&guard, root, vol, call) {
        Ok(output) => {
            let mut output = output;
            if output.len() > TOOL_OUTPUT_CAP {
                output.truncate(TOOL_OUTPUT_CAP);
                output.push_str("\n… truncated at 64 KiB");
            }
            ToolOutcome::Continue(output)
        }
        Err(e) => ToolOutcome::Continue(format!("error: {e}")),
    }
}

/// Inline bounded subagent: same config, read-only tool subset, short leash.
/// Shares the parent transcript afterwards as one summarized tool result.
#[allow(clippy::too_many_arguments)]
fn run_subagent(
    db: &Arc<RwLock<Database>>,
    job: &Arc<AgentJob>,
    config: &AgentConfig,
    root: &Path,
    vol: &Path,
    turn: &agent_loop::AgentTurn,
    call: &ToolCall,
) -> ToolOutcome {
    if turn.task_depth >= agent_loop::MAX_TASK_DEPTH {
        return ToolOutcome::Continue(
            "deny: max subagent depth reached — finish this level yourself".to_string(),
        );
    }
    let decision = agent_config::decide(&config.permission, config.agent_kind, "task", &call.input);
    if matches!(decision, agent_config::PermissionDecision::Deny { .. }) {
        return ToolOutcome::Continue("deny: `task` is denied by the permission ruleset".to_string());
    }
    if !config.auto_approve {
        // Subagents spawn workers — always ask first unless auto mode.
        set_state(job, |s| {
            s.status = "waiting_approval".to_string();
            s.pending = Some(PendingApproval {
                tool: "task".into(),
                input: call.input.clone(),
                summary: "Spawn a subagent for this goal?".into(),
                question: None,
            });
        });
        match wait_approval(job) {
            Some(answer) if answer.approved => {
                set_state(job, |s| {
                    s.status = "running".to_string();
                    s.pending = None;
                });
            }
            Some(_) => {
                set_state(job, |s| {
                    s.status = "running".to_string();
                    s.pending = None;
                });
                return ToolOutcome::Continue("denied: user rejected the subagent".to_string());
            }
            None => {
                set_state(job, |s| {
                    s.status = "running".to_string();
                    s.pending = None;
                });
                return ToolOutcome::Continue("denied: subagent approval timed out".to_string());
            }
        }
    }

    let goal = call
        .input
        .get("goal")
        .and_then(|g| g.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if goal.is_empty() {
        return ToolOutcome::Continue("invalid: task needs a goal".to_string());
    }
    let endpoint = match providers::resolve(config) {
        Ok(endpoint) => endpoint,
        Err(e) => return ToolOutcome::Continue(format!("error: {e}")),
    };
    let api_key = match db.read().ok().and_then(|guard| load_key(&guard, &config.id).ok()) {
        Some(key) => key,
        None => return ToolOutcome::Continue("error: database unavailable".to_string()),
    };
    let context = call
        .input
        .get("context")
        .and_then(|c| c.as_str())
        .unwrap_or("");
    let mut sub = agent_loop::AgentTurn::new(
        vec![cybermanju_types::agent::ChatMessage {
            role: "user".into(),
            content: format!("Subagent goal: {goal}\nContext: {context}"),
            tool_call_id: None,
            tool_name: None,
            tool_input: None,
        }],
        SUBAGENT_MAX_TURNS,
        turn.task_depth + 1,
    );
    let system = agent_loop::system_prompt(
        &root.to_string_lossy(),
        "build",
        &repo_overview(root),
    );
    let headers = endpoint_headers(&endpoint, &api_key);
    let model = config.model.clone();
    let mut summary = String::from("(subagent produced no text)");
    for _ in 0..SUBAGENT_MAX_TURNS {
        if job.cancel.load(Ordering::SeqCst) {
            return ToolOutcome::Continue("subagent cancelled with the parent run".to_string());
        }
        let (mut url, headers, body) =
            sub.build_request(&endpoint.base_url, endpoint.dialect, &model, &system, headers.clone(), true);
        if endpoint.auth == cybermanju_types::agent::AuthScheme::Query {
            let name = endpoint.auth_name.as_deref().unwrap_or("key");
            url = protocol::with_query_key(&url, name, &api_key);
        }
        // Strip mutating tools: subagents read and report.
        let mut body = body;
        if let Some(tools) = body.get_mut("tools").and_then(|t| t.as_array_mut()) {
            let keep = |name: &str| matches!(name, "read" | "list" | "grep");
            tools.retain(|t| {
                t.get("function")
                    .and_then(|f| f.get("name"))
                    .or_else(|| t.get("name"))
                    .and_then(|n| n.as_str())
                    .map(keep)
                    .unwrap_or(false)
            });
        }
        let reply = match protocol::post_json(&url, &headers, &body) {
            Ok(reply) => reply,
            Err(e) => return ToolOutcome::Continue(format!("subagent transport failed: {e}")),
        };
        match sub.ingest_reply(endpoint.dialect, &reply) {
            Ok(agent_loop::LoopEvent::TextDone) => {
                summary = sub
                    .messages
                    .iter()
                    .rev()
                    .find(|m| m.role == "assistant")
                    .map(|m| m.content.clone())
                    .unwrap_or(summary);
                break;
            }
            Ok(agent_loop::LoopEvent::ToolCalls(calls)) => {
                for call in &calls {
                    // Belt and braces: the stripped schema above plus a
                    // runtime gate — a subagent never mutates.
                    let output = match call.name.as_str() {
                        "read" | "list" | "grep" => {
                            let guard = match db.read() {
                                Ok(guard) => guard,
                                Err(e) => {
                                    sub.append_tool_result(call, format!("error: database unavailable: {e}"));
                                    continue;
                                }
                            };
                            match exec_tool(&guard, root, vol, call) {
                                Ok(output) => output,
                                Err(e) => format!("error: {e}"),
                            }
                        }
                        other => format!("deny: subagents cannot run `{other}`"),
                    };
                    sub.append_tool_result(call, output);
                }
            }
            Ok(agent_loop::LoopEvent::LimitReached) | Err(_) => break,
        }
    }
    let short: String = summary.chars().take(4000).collect();
    ToolOutcome::Continue(format!("subagent result:\n{short}"))
}

// ─── cybsh `ai …` (lockless REST intercept) ────────────────────────────────

/// Terminal-shaped answer for `POST /api/os/exec` agent lines. A command
/// that ran and failed is still HTTP 200 with `ok: false` — the terminal
/// renders the message instead of failing the request.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AiExecResult {
    ok: bool,
    line: String,
    output: String,
    prompt: &'static str,
}

fn ai_ok(line: String, output: String, origin: Option<&str>) -> String {
    crate::json_ok(
        &AiExecResult {
            ok: true,
            line,
            output,
            prompt: "cybsh> ",
        },
        origin,
    )
}

fn ai_err(line: String, output: String, origin: Option<&str>) -> String {
    crate::json_ok(
        &AiExecResult {
            ok: false,
            line,
            output,
            prompt: "cybsh> ",
        },
        origin,
    )
}

/// Dispatch one `ai …` terminal line without holding the request lock.
///
/// Called from `route_request` before any lock is taken (job start spawns a
/// worker that takes its own locks; the registry itself is process-global).
/// Returns `None` for non-`ai` lines so they flow to the normal locked path.
pub fn try_ai_exec(
    shared: &Arc<RwLock<Database>>,
    body: &str,
    origin: Option<&str>,
) -> Option<String> {
    #[derive(Deserialize)]
    struct ExecLine {
        #[serde(default)]
        line: String,
    }
    let req: ExecLine = serde_json::from_str(body).ok()?;
    let parsed = cybermanju_os::shell::parse_ai_command(&req.line)?;
    let line = req.line.clone();

    match parsed {
        cybermanju_os::shell::AiCommand::Ask {
            prompt,
            config_id,
            session_id,
        } => {
            let config_id = match config_id {
                Some(id) => id,
                None => {
                    let guard = shared.read().ok()?;
                    let configs = list_configs(&guard).ok()?;
                    drop(guard);
                    match configs.into_iter().next().map(|c| c.id) {
                        Some(id) => id,
                        None => {
                            return Some(ai_err(
                                line,
                                "no agent configs — create one in the Agent panel first".to_string(),
                                origin,
                            ))
                        }
                    }
                }
            };
            match start_job(shared, &config_id, session_id, prompt) {
                Ok(job) => Some(ai_ok(
                    line,
                    format!(
                        "started agent job {} (session {}) — poll with `ai status`",
                        job.job_id, job.session_id
                    ),
                    origin,
                )),
                Err(message) => Some(ai_err(line, message, origin)),
            }
        }
        cybermanju_os::shell::AiCommand::Status { job_id } => {
            let snapshot = match job_id {
                Some(id) => job_status(&id).ok(),
                None => list_jobs().into_iter().find(|j| {
                    j.status == "running" || j.status == "waiting_approval"
                }),
            };
            match snapshot {
                Some(job) => Some(ai_ok(
                    line,
                    format!(
                        "job {} · {} · turn {}/{} · in {} out {} tokens{}",
                        job.job_id,
                        job.status,
                        job.turns_used,
                        job.max_turns,
                        job.usage.input_tokens,
                        job.usage.output_tokens,
                        job.error
                            .as_ref()
                            .map(|e| format!(" · error: {e}"))
                            .unwrap_or_default(),
                    ),
                    origin,
                )),
                None => Some(ai_ok(line, "no agent jobs running".to_string(), origin)),
            }
        }
        cybermanju_os::shell::AiCommand::Abort { job_id } => {
            let id = match job_id {
                Some(id) => id,
                None => match list_jobs().into_iter().find(|j| {
                    j.status == "running" || j.status == "waiting_approval"
                }) {
                    Some(job) => job.job_id,
                    None => return Some(ai_ok(line, "no running agent job to abort".to_string(), origin)),
                },
            };
            match abort_job(&id) {
                Ok(true) => Some(ai_ok(line, format!("agent job {id} aborted"), origin)),
                Ok(false) => Some(ai_ok(line, format!("agent job {id} already finished"), origin)),
                Err(message) => Some(ai_err(line, message, origin)),
            }
        }
        cybermanju_os::shell::AiCommand::Sessions => {
            let guard = shared.read().ok()?;
            match list_sessions(&guard) {
                Ok(sessions) if sessions.is_empty() => {
                    Some(ai_ok(line, "no agent sessions yet — `ai ask \"…\"` starts one".to_string(), origin))
                }
                Ok(sessions) => {
                    let mut out = format!("{} session(s):\n", sessions.len());
                    for session in sessions.iter().take(20) {
                        out.push_str(&format!(
                            "  {} · {} · {} msgs · {}\n",
                            session.id,
                            session.title,
                            session.messages.len(),
                            session.updated_at,
                        ));
                    }
                    Some(ai_ok(line, out.trim_end().to_string(), origin))
                }
                Err(message) => Some(ai_err(line, message, origin)),
            }
        }
    }
}
