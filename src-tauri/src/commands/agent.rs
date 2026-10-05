use tauri::State;

use crate::AppState;

// ---------------------------------------------------------------------------
// Tauri commands (thin wrappers over the shared agent API)
// ---------------------------------------------------------------------------

/// List provider presets (endpoints, families, default models).
#[tauri::command]
pub fn list_agent_providers() -> Result<Vec<cybermanju_types::agent::ProviderPreset>, String> {
    Ok(cybermanju_agent::providers::all_presets())
}

/// List saved agent configs (keyless rows, `hasKey` only).
#[tauri::command]
pub fn list_agent_configs(
    state: State<'_, AppState>,
) -> Result<Vec<cybermanju_types::agent::AgentConfig>, String> {
    let db = state.db.read().map_err(|e| e.to_string())?;
    cybermanju_web::api::agent_api::list_configs(&db)
}

/// Create or overwrite an agent config.
#[tauri::command]
pub fn save_agent_config(
    config: cybermanju_types::agent::AgentConfig,
    state: State<'_, AppState>,
) -> Result<cybermanju_types::agent::AgentConfig, String> {
    let db = state.db.read().map_err(|e| e.to_string())?;
    cybermanju_web::api::agent_api::save_config(&db, config)
}

/// Delete an agent config and its sealed key.
#[tauri::command]
pub fn delete_agent_config(
    config_id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let db = state.db.read().map_err(|e| e.to_string())?;
    cybermanju_web::api::agent_api::delete_config(&db, &config_id)
}

/// Seal a provider key for a config (never echoed back).
#[tauri::command]
pub fn save_agent_key(
    config_id: String,
    api_key: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let db = state.db.read().map_err(|e| e.to_string())?;
    cybermanju_web::api::agent_api::save_key(&db, &config_id, &api_key)
}

/// Refresh the model list from the provider.
#[tauri::command]
pub fn list_agent_models(
    config_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let db = state.db.read().map_err(|e| e.to_string())?;
    cybermanju_web::api::agent_api::list_models(&db, &config_id)
}

/// List sessions (newest first).
#[tauri::command]
pub fn list_agent_sessions(
    state: State<'_, AppState>,
) -> Result<Vec<cybermanju_types::agent::AgentSession>, String> {
    let db = state.db.read().map_err(|e| e.to_string())?;
    cybermanju_web::api::agent_api::list_sessions(&db)
}

/// Load one session transcript.
#[tauri::command]
pub fn get_agent_session(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<cybermanju_types::agent::AgentSession, String> {
    let db = state.db.read().map_err(|e| e.to_string())?;
    cybermanju_web::api::agent_api::get_session(&db, &session_id)
}

/// Create a session bound to a config snapshot.
#[tauri::command]
pub fn create_agent_session(
    config_id: String,
    title: Option<String>,
    state: State<'_, AppState>,
) -> Result<cybermanju_types::agent::AgentSession, String> {
    let db = state.db.read().map_err(|e| e.to_string())?;
    cybermanju_web::api::agent_api::create_session(&db, &config_id, title)
}

/// Import a session transcript (re-keyed server-side).
#[tauri::command]
pub fn import_agent_session(
    session: cybermanju_types::agent::AgentSession,
    state: State<'_, AppState>,
) -> Result<cybermanju_types::agent::AgentSession, String> {
    let db = state.db.read().map_err(|e| e.to_string())?;
    cybermanju_web::api::agent_api::import_session(&db, session)
}

/// Delete a session.
#[tauri::command]
pub fn delete_agent_session(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let db = state.db.read().map_err(|e| e.to_string())?;
    cybermanju_web::api::agent_api::delete_session(&db, &session_id)
}

/// Start a detached agent run; returns immediately with a job snapshot.
/// Poll with `agent_job_status`; cancel with `abort_agent_job`.
#[tauri::command]
pub fn start_agent_run(
    config_id: String,
    session_id: Option<String>,
    prompt: String,
    state: State<'_, AppState>,
) -> Result<cybermanju_web::api::agent_api::JobSnapshot, String> {
    cybermanju_web::api::agent_api::start_job(&state.db, &config_id, session_id, prompt)
}

/// Poll one job.
#[tauri::command]
pub fn agent_job_status(
    job_id: String,
) -> Result<cybermanju_web::api::agent_api::JobSnapshot, String> {
    cybermanju_web::api::agent_api::job_status(&job_id)
}

/// List known jobs (newest first).
#[tauri::command]
pub fn list_agent_jobs() -> Result<Vec<cybermanju_web::api::agent_api::JobSnapshot>, String> {
    Ok(cybermanju_web::api::agent_api::list_jobs())
}

/// Request cancellation (idempotent).
#[tauri::command]
pub fn abort_agent_job(job_id: String) -> Result<bool, String> {
    cybermanju_web::api::agent_api::abort_job(&job_id)
}

/// Answer a parked approval or question.
#[tauri::command]
pub fn approve_agent_job(
    job_id: String,
    approved: bool,
    answer: Option<String>,
) -> Result<bool, String> {
    cybermanju_web::api::agent_api::approve_job(&job_id, approved, answer)
}
