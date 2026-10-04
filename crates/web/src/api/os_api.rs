// Cybermanju Drive — OS routes (AGENT-8)
//
// Pre-created and pre-hooked by the supervisor: `crates/web/src/lib.rs` calls
// `route()` right after the auth gate, and `os` is already in
// `security::ROUTED_SEGMENTS`. AGENT-8 implements the arms; nobody edits
// `lib.rs`, `security.rs` or `api/mod.rs`.
//
// Contract: return `Some(response)` for a path this family owns, `None` for
// anything else. `POST /api/os/exec` (the `cybsh` terminal) is the important
// one — it must be authenticated like every other route.

use cybermanju_db::Database;

/// Dispatch `/api/os/*` (exec, stat, ls, df, ps, top, jobs, workers).
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
