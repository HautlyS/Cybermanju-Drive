// Cybermanju Drive — Shared command logic
//
// Single source of truth for business logic used by BOTH the Tauri IPC
// commands (`src-tauri/src/commands`) and the HTTP REST routes
// (`crates/web/src/lib.rs`). Commands take an already-locked `&Database`;
// callers are responsible for acquiring the right lock for the operation.

pub mod accounts;
pub mod audit;
pub mod batch;
pub mod collections;
pub mod files;
pub mod oauth;
pub mod search_api;
pub mod share;
pub mod sync_api;
pub mod trash;
pub mod users;
pub mod versions;
