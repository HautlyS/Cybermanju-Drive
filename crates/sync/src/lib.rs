// Cybermanju Drive — Storage Sync Engine
// Shared by the Tauri desktop app, the web dashboard and the Docker server.
//
// Single source of truth for:
//   * the sync models (re-exported from cybermanju-types)
//   * the five storage backends
//   * the OAuth2 token refresh helpers
//   * the scan → compress → upload → link pipeline

pub mod backends;
pub mod oauth;
pub mod pipeline;
pub mod state;

pub use backends::create_backend;
pub use cybermanju_types::sync::*;
pub use pipeline::SyncPipeline;
pub use state::SyncState;
