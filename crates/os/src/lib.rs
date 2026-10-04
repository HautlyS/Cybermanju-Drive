//! Cybermanju OS layer (AGENT-8): the system terminal, task table, compute
//! fan-out and the typed syscall boundary every app-facing surface sits on.
//!
//! AGENT-8: build the modules here — `shell` (`cybsh`), `task` (`ps`/`top`),
//! `compute` (fan-out scheduler), `api` (open/read/write/…/df). Declare each
//! with `pub mod` as you create the file.

/// Canonical shell prompt.
pub const PROMPT: &str = "cybsh> ";

/// Shell/OS layer format version reported by `cybsh`'s `version` command.
pub const OS_VERSION: u32 = 1;
