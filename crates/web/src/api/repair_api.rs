// Cybermanju Drive — durability routes (AGENT-7)
//
// Pre-created and pre-hooked by the supervisor: `crates/web/src/lib.rs` calls
// `route()` right after the auth gate, and `repair`/`scrub` are already in
// `security::ROUTED_SEGMENTS`. AGENT-7 implements the arms; nobody edits
// `lib.rs`, `security.rs` or `api/mod.rs`.
//
// Contract: return `Some(response)` for a path this family owns, `None` for
// anything else. Do NOT hold this call across long work (a scrub or repair run
// is a background task) — spawn it and return `202` with a task id.

use cybermanju_db::Database;

/// Dispatch `/api/repair/*`, `/api/scrub/*`, `/api/lease/*`.
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
