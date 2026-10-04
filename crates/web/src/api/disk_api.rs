// Cybermanju Drive — disk & volume routes (AGENT-6)
//
// Pre-created and pre-hooked by the supervisor: `crates/web/src/lib.rs` calls
// `route()` right after the auth gate, and `os`/`disk`/`volume` are already in
// `security::ROUTED_SEGMENTS`. AGENT-6 implements the arms; nobody edits
// `lib.rs`, `security.rs` or `api/mod.rs`.
//
// Contract: return `Some(response)` for a path this family owns, `None` for
// anything else (the router then tries `repair_api`, then `os_api`, then the
// main handler). Do NOT hold this call across long work — spawn a task and
// return immediately.

use cybermanju_db::Database;

/// Dispatch `/api/disk/*` and `/api/volume/*`.
pub fn route(
    db: &Database,
    method: &str,
    path_segments: &[&str],
    body: &str,
    origin: Option<&str>,
) -> Option<String> {
    let _ = (db, method, path_segments, body, origin);
    None
}
