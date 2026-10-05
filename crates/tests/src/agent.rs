// Native agent REST contract: providers, keyless configs, sessions,
// detached jobs with approvals, and the `cybsh ai` intercept.

use crate::web::{bearer, body_of, bootstrap_session, call, mint, mk_dashboard, now_secs, status_of};

fn authed() -> (tempfile::TempDir, std::sync::Arc<cybermanju_web::WebDashboard>, String) {
    let (dir, d) = mk_dashboard(3456);
    let token = bootstrap_session(&d, "agent", "correct horse battery");
    let auth = bearer(&token);
    (dir, d, auth)
}

fn make_config(d: &cybermanju_web::WebDashboard, auth: &str) -> String {
    let body = r#"{"config":{
        "id":"", "name":"Test", "providerId":"ollama", "model":"llama3.1:8b",
        "workingDir":"", "agentKind":"build",
        "permission":{"default":"ask","rules":{}}, "autoApprove":false, "maxTurns":5
    }}"#;
    let resp = call(d, "POST", "/api/agent/configs", body, Some(auth));
    assert_eq!(status_of(&resp), 200, "create config: {resp}");
    let value: serde_json::Value = serde_json::from_str(body_of(&resp)).expect("json");
    assert_eq!(value["hasKey"], false);
    assert!(value.get("apiKey").is_none(), "key material leaked: {value}");
    value["id"].as_str().expect("config id").to_string()
}

#[test]
fn agent_routes_are_auth_gated() {
    let (_dir, d) = mk_dashboard(3456);
    for (method, path, body) in [
        ("GET", "/api/agent/providers", ""),
        ("GET", "/api/agent/configs", ""),
        ("GET", "/api/agent/sessions", ""),
        ("GET", "/api/agent/jobs", ""),
        ("POST", "/api/agent/prompt", r#"{"configId":"x","prompt":"hi"}"#),
        ("POST", "/api/os/exec", r#"{"line":"ai ask hi"}"#),
    ] {
        let resp = call(&d, method, path, body, None);
        assert_eq!(status_of(&resp), 401, "{method} {path}: {resp}");
    }
}

#[test]
fn providers_list_without_secrets() {
    let (_dir, d, auth) = authed();
    let resp = call(&d, "GET", "/api/agent/providers", "", Some(&auth));
    assert_eq!(status_of(&resp), 200, "{resp}");
    let presets: Vec<serde_json::Value> = serde_json::from_str(body_of(&resp)).expect("json");
    assert_eq!(presets.len(), 10, "{presets:?}");
    assert!(presets.iter().any(|p| p["id"] == "ollama"));
    let body = body_of(&resp);
    assert!(!body.contains("sk-"), "no key material in catalog");
}

#[test]
fn config_keys_never_serialize_back() {
    let (_dir, d, auth) = authed();
    let id = make_config(&d, &auth);

    let resp = call(
        &d,
        "PUT",
        &format!("/api/agent/configs/{id}/key"),
        r#"{"apiKey":"sk-test-secret"}"#,
        Some(&auth),
    );
    assert_eq!(status_of(&resp), 200, "{resp}");

    for path in [format!("/api/agent/configs/{id}"), "/api/agent/configs".to_string()] {
        let resp = call(&d, "GET", &path, "", Some(&auth));
        assert_eq!(status_of(&resp), 200, "{resp}");
        let body = body_of(&resp);
        assert!(!body.contains("sk-test-secret"), "key leaked in {path}");
    }
    let resp = call(&d, "GET", &format!("/api/agent/configs/{id}"), "", Some(&auth));
    let value: serde_json::Value = serde_json::from_str(body_of(&resp)).expect("json");
    assert_eq!(value["hasKey"], true);

    // Empty keys are rejected, unknown configs 404.
    let resp = call(
        &d,
        "PUT",
        &format!("/api/agent/configs/{id}/key"),
        r#"{"apiKey":""}"#,
        Some(&auth),
    );
    assert_eq!(status_of(&resp), 400, "{resp}");
    let resp = call(&d, "GET", "/api/agent/configs/nope", "", Some(&auth));
    assert_eq!(status_of(&resp), 404, "{resp}");

    // Custom providers require an endpoint.
    let resp = call(
        &d,
        "POST",
        "/api/agent/configs",
        r#"{"config":{"id":"","name":"C","providerId":"custom","model":"m",
            "workingDir":"","agentKind":"build","permission":{"default":"ask","rules":{}},
            "autoApprove":false,"maxTurns":5}}"#,
        Some(&auth),
    );
    assert_eq!(status_of(&resp), 400, "{resp}");
}

#[test]
fn prompt_validates_before_spawning() {
    let (_dir, d, auth) = authed();
    // Unknown config → 404, no thread spawned.
    let resp = call(
        &d,
        "POST",
        "/api/agent/prompt",
        r#"{"configId":"missing","prompt":"hi"}"#,
        Some(&auth),
    );
    assert_eq!(status_of(&resp), 404, "{resp}");

    // Known config without a key (non-keyless) → 400 auth, no thread.
    let id = make_config(&d, &auth);
    let resp = call(
        &d,
        "POST",
        "/api/agent/prompt",
        &format!(r#"{{"configId":"{id}","prompt":"hi"}}"#),
        Some(&auth),
    );
    assert_eq!(status_of(&resp), 400, "{resp}");
    assert!(body_of(&resp).contains("auth:"), "{resp}");

    // Empty prompt → 400.
    let resp = call(
        &d,
        "POST",
        "/api/agent/prompt",
        &format!(r#"{{"configId":"{id}","prompt":""}}"#),
        Some(&auth),
    );
    assert_eq!(status_of(&resp), 400, "{resp}");

    // Unknown jobs 404 on every job route.
    for (method, path, body) in [
        ("GET", "/api/agent/jobs/nope", ""),
        ("POST", "/api/agent/jobs/nope/abort", ""),
        ("POST", "/api/agent/jobs/nope/approve", r#"{"approved":true}"#),
    ] {
        let resp = call(&d, method, path, body, Some(&auth));
        assert_eq!(status_of(&resp), 404, "{method} {path}: {resp}");
    }
}

#[test]
fn sessions_crud_and_import_rekey() {
    let (_dir, d, auth) = authed();
    let id = make_config(&d, &auth);

    let resp = call(
        &d,
        "POST",
        "/api/agent/sessions",
        &format!(r#"{{"configId":"{id}","title":"T"}}"#),
        Some(&auth),
    );
    assert_eq!(status_of(&resp), 200, "{resp}");
    let created: serde_json::Value = serde_json::from_str(body_of(&resp)).expect("json");
    let sid = created["id"].as_str().expect("session id").to_string();

    let resp = call(&d, "GET", "/api/agent/sessions", "", Some(&auth));
    assert_eq!(status_of(&resp), 200, "{resp}");

    // Import always re-keys — client ids are never trusted.
    let resp = call(
        &d,
        "POST",
        "/api/agent/sessions/import",
        &format!(
            r#"{{"session":{{"id":"{sid}","title":"T","configId":"{id}",
                "providerId":"ollama","model":"m","agentKind":"build","workingDir":"",
                "messages":[],"usage":{{"inputTokens":0,"outputTokens":0}},
                "createdAt":"","updatedAt":""}}}}"#
        ),
        Some(&auth),
    );
    assert_eq!(status_of(&resp), 200, "{resp}");
    let imported: serde_json::Value = serde_json::from_str(body_of(&resp)).expect("json");
    assert_ne!(imported["id"].as_str(), Some(sid.as_str()));

    let resp = call(&d, "DELETE", &format!("/api/agent/sessions/{sid}"), "", Some(&auth));
    assert_eq!(status_of(&resp), 200, "{resp}");
}

#[test]
fn cybsh_ai_without_configs_answers_honestly() {
    let (_dir, d, auth) = authed();
    // No configs exist: the intercept answers ok:false, HTTP stays 200.
    let resp = call(&d, "POST", "/api/os/exec", r#"{"line":"ai ask hello"}"#, Some(&auth));
    assert_eq!(status_of(&resp), 200, "{resp}");
    let body: serde_json::Value = serde_json::from_str(body_of(&resp)).expect("json");
    assert_eq!(body["ok"], false);
    assert!(
        body["output"].as_str().unwrap_or_default().contains("no agent configs"),
        "{body}"
    );
}

#[test]
fn code_parse_still_serves_alongside_agent_routes() {
    let (_dir, d, auth) = authed();
    let resp = call(
        &d,
        "POST",
        "/api/code/parse",
        r#"{"fileName":"a.rs","content":"fn a() {}"}"#,
        Some(&auth),
    );
    assert_eq!(status_of(&resp), 200, "{resp}");
}

#[test]
fn mcp_management_is_admin_gated_and_validated() {
    let (_dir, d) = mk_dashboard(3456);
    let admin = bearer(&mint(&d, "admin", now_secs() + 3_600, "jti-agent-mcp-admin"));
    let member = bearer(&mint(&d, "user", now_secs() + 3_600, "jti-agent-mcp-user"));

    // Any authenticated user may create a config; seed one as member.
    let body = r#"{"config":{
        "id":"","name":"MCP","providerId":"ollama","model":"llama3.1:8b",
        "workingDir":"","agentKind":"build","permission":{"default":"ask","rules":{}},
        "autoApprove":false,"maxTurns":5}}"#;
    let resp = call(&d, "POST", "/api/agent/configs", body, Some(&member));
    assert_eq!(status_of(&resp), 200, "{resp}");
    let id: String = serde_json::from_str::<serde_json::Value>(body_of(&resp))
        .expect("json")["id"]
        .as_str()
        .expect("id")
        .to_string();

    let attach = |auth: &str, payload: &str| {
        call(
            &d,
            "POST",
            &format!("/api/agent/configs/{id}/mcp"),
            payload,
            Some(auth),
        )
    };
    // Member attach → 403 (stdio spawns processes).
    let resp = attach(
        &member,
        r#"{"name":"fs","server":{"transport":"stdio","command":"npx","args":[]}}"#,
    );
    assert_eq!(status_of(&resp), 403, "{resp}");

    // Unknown transport → 400 even for admins.
    let resp = attach(
        &admin,
        r#"{"name":"fs","server":{"transport":"ssh"}}"#,
    );
    assert_eq!(status_of(&resp), 400, "{resp}");

    // Slash in the name → 400 (names become tool ids).
    let resp = attach(
        &admin,
        r#"{"name":"a/b","server":{"transport":"stdio","command":"x","args":[]}}"#,
    );
    assert_eq!(status_of(&resp), 400, "{resp}");

    // Unknown config → 404.
    let resp = call(
        &d,
        "POST",
        "/api/agent/configs/nope/mcp",
        r#"{"name":"fs","server":{"transport":"stdio","command":"x","args":[]}}"#,
        Some(&admin),
    );
    assert_eq!(status_of(&resp), 404, "{resp}");

    // Detach of nothing → 404; detach path is admin-gated too.
    let resp = call(
        &d,
        "DELETE",
        &format!("/api/agent/configs/{id}/mcp/fs"),
        "",
        Some(&admin),
    );
    assert_eq!(status_of(&resp), 404, "{resp}");
    let resp = call(
        &d,
        "DELETE",
        &format!("/api/agent/configs/{id}/mcp/fs"),
        "",
        Some(&member),
    );
    assert_eq!(status_of(&resp), 403, "{resp}");

    // Tools discovery on unknown config → 404.
    let resp = call(&d, "GET", "/api/agent/configs/nope/mcp/tools", "", Some(&admin));
    assert_eq!(status_of(&resp), 404, "{resp}");
}

#[test]
fn compact_validates_before_any_network() {
    let (_dir, d) = mk_dashboard(3456);
    let token = bootstrap_session(&d, "agent-compact", "correct horse battery");
    let auth = bearer(&token);
    let id = make_config(&d, &auth);

    // Unknown session → 404 without touching the network.
    let resp = call(
        &d,
        "POST",
        "/api/agent/sessions/nope/compact",
        &format!(r#"{{"configId":"{id}"}}"#),
        Some(&auth),
    );
    assert_eq!(status_of(&resp), 404, "{resp}");

    // Empty transcript → 400, no provider call.
    let resp = call(
        &d,
        "POST",
        "/api/agent/sessions",
        &format!(r#"{{"configId":"{id}","title":"Empty"}}"#),
        Some(&auth),
    );
    assert_eq!(status_of(&resp), 200, "{resp}");
    let sid: String = serde_json::from_str::<serde_json::Value>(body_of(&resp)).expect("json")["id"]
        .as_str()
        .expect("sid")
        .to_string();
    let resp = call(
        &d,
        "POST",
        &format!("/api/agent/sessions/{sid}/compact"),
        &format!(r#"{{"configId":"{id}"}}"#),
        Some(&auth),
    );
    assert_eq!(status_of(&resp), 400, "{resp}");
    assert!(body_of(&resp).contains("no messages"), "{resp}");
}
