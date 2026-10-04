//! Cybermanju Drive — Reed–Solomon erasure coding (AGENT-7).
//!
//! Replaces the replication-only `parity` in striped placement with real
//! `k`-data + `m`-parity shard coding, so a chunk survives `m` provider losses
//! instead of needing a full copy on each.
//!
//! AGENT-7: build `codec.rs` here and declare it with `pub mod`.

/// Default number of parity shards when a config only says "parity: 1".
pub const DEFAULT_PARITY: u8 = 1;
