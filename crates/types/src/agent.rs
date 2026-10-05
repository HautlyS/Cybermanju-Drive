// Cybermanju Drive — shared AI agent types.
//
// Used by the Tauri desktop app, the Docker web server, and the WASM
// dispatcher. Raw provider keys NEVER appear here: configs carry no key
// field at all (keys live in the `sync_secrets` side table under
// `agent:key:<config_id>`); list/get responses report `hasKey` only.

use serde::{Deserialize, Serialize};

/// How a provider authenticates.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AuthScheme {
    /// `Authorization: Bearer <key>`.
    #[default]
    Bearer,
    /// Custom header, e.g. `x-api-key`.
    Header,
    /// `?key=<key>` query parameter.
    Query,
    /// No credential (local runtimes like Ollama).
    None,
}

/// Which chat wire format a provider speaks.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LlmDialect {
    /// OpenAI `/chat/completions` shape (also serves OpenRouter, Ollama
    /// `/v1`, Gemini OpenAI-compat, Groq, Mistral, DeepSeek, xAI, Cerebras).
    #[default]
    OpenAi,
    /// Anthropic `/v1/messages` shape.
    Anthropic,
}

/// One provider preset. `baseUrl` is the chat-completions-compatible root
/// (dialect decides the suffix); every field is user-overridable per config.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderPreset {
    pub id: String,
    pub label: String,
    pub family: String,
    pub base_url: String,
    pub default_model: String,
    pub dialect: LlmDialect,
    pub auth: AuthScheme,
    /// Header name when `auth == Header`, query name when `Query`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_name: Option<String>,
    /// Env var hint for BYOK paste (e.g. `ANTHROPIC_API_KEY`).
    pub key_env: String,
    /// True when no key is needed at all.
    #[serde(default)]
    pub keyless: bool,
    /// Extra headers always sent (e.g. OpenRouter attribution).
    #[serde(default)]
    pub extra_headers: Vec<(String, String)>,
}

/// One approval decision.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PermissionAction {
    /// Run without asking.
    Allow,
    /// Pause the job and ask (default).
    #[default]
    Ask,
    /// Block, even in auto mode.
    Deny,
}

/// One tool's rule: either a single action or granular pattern → action
/// pairs (`*` matches any run, `?` one char; last match wins — put the
/// catch-all first, specifics after, like opencode/omp).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum PermissionRule {
    Simple(PermissionAction),
    Granular(Vec<(String, PermissionAction)>),
}

/// Full permission set for a config. Absent tools fall back to `default`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRuleset {
    #[serde(default = "default_ask")]
    pub default: PermissionAction,
    #[serde(default)]
    pub rules: std::collections::BTreeMap<String, PermissionRule>,
}

fn default_ask() -> PermissionAction {
    PermissionAction::Ask
}

impl Default for PermissionRuleset {
    fn default() -> Self {
        Self {
            default: PermissionAction::Ask,
            rules: std::collections::BTreeMap::new(),
        }
    }
}

/// Which agent persona runs a session.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AgentKind {
    /// Full access within the permission ruleset.
    #[default]
    Build,
    /// Read-only: edits/writes/bash denied regardless of rules.
    Plan,
}

/// A saved agent configuration. No key material — see module docs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfig {
    pub id: String,
    pub name: String,
    pub provider_id: String,
    pub model: String,
    /// Override the preset endpoint (self-hosted gateways, proxies).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url_override: Option<String>,
    /// Override the preset dialect (required when `provider_id` is custom).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dialect_override: Option<LlmDialect>,
    /// Override the preset auth scheme (required when custom).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_scheme_override: Option<AuthScheme>,
    /// Header/query name when the scheme needs one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_name_override: Option<String>,
    /// Working root for tools. Empty = volume root.
    #[serde(default)]
    pub working_dir: String,
    #[serde(default)]
    pub agent_kind: AgentKind,
    #[serde(default)]
    pub permission: PermissionRuleset,
    /// Auto-approve `ask` (never overrides `deny`).
    #[serde(default)]
    pub auto_approve: bool,
    #[serde(default = "default_max_turns")]
    pub max_turns: u32,
    /// A key is stored server-side (never echoed back).
    #[serde(default)]
    pub has_key: bool,
    pub created_at: String,
    pub updated_at: String,
}

fn default_max_turns() -> u32 {
    25
}

/// One transcript message (provider-neutral; mapped per dialect on send).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_input: Option<serde_json::Value>,
}

/// A tool invocation the model requested.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
}

/// Token/cost accounting for one turn or session.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
}

/// A persisted agent session: config snapshot + full transcript.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentSession {
    pub id: String,
    pub title: String,
    pub config_id: String,
    pub provider_id: String,
    pub model: String,
    #[serde(default)]
    pub agent_kind: AgentKind,
    #[serde(default)]
    pub working_dir: String,
    #[serde(default)]
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub usage: TokenUsage,
    pub created_at: String,
    pub updated_at: String,
}
