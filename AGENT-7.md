# AGENT-7 — Durability, Repair & Multi-Writer

**Part of the 3-agent Cybermanju OS push.** The product vision: a *decentralized OS* over
third-party providers where storage is striped, encrypted and compressed across many disks.
Striping without repair is just a wider way to lose data — this brief owns the layer that
makes the pool **survive provider failure**: scrub, repair, erasure coding, replicated
catalog, health-driven placement, garbage collection, and multi-writer safety.

- AGENT-6 — `.cybermanju` disks, allocator, volume merge, mount, `df`
- **AGENT-7 (this file)** — durability, scrub, repair, RS, catalog replication, health, GC, leases
- AGENT-8 — `cybsh` terminal, task model, compute fan-out, syscall boundary, UI

Read [`MISSING.md`](./MISSING.md) first — this brief closes **C, F4** there.

## Ground rules

1. **Three agents share this worktree right now.** Only touch the files in *Files you
   own*. Shared files were pre-wired by the supervisor and are **read-only** — request
   anything else under *Requests to the supervisor*.
2. **Never edit:** `crates/web/src/lib.rs`, `crates/web/src/security.rs`,
   `crates/web/src/api/mod.rs`, `crates/db/src/database.rs`, root `Cargo.toml`,
   `crates/tests/src/lib.rs`, `package.json`, `.github/**`, `src/**` (AGENT-8),
   `crates/os/**` (AGENT-8), `crates/disk/**` + `pipeline.rs` admission block (AGENT-6).
3. **Verify locally.** Export `ORT_CACHE_DIR="$HOME/.cache/ort"` before any cargo command,
   or `ort-sys` panics during `cargo check`.
4. **Do not run `git` commands.** The supervisor commits and pushes after each green gate.
5. Append a dated line to **Log** after each unit of work.

## Files you own

| Path | Notes |
|---|---|
| `crates/erasure/**` | **new** crate — Reed–Solomon encode/decode |
| `crates/sync/src/manifest.rs` | RS-backed parity, catalog serialization (AGENT-2 file — marked edits) |
| `crates/sync/src/scrub.rs` | **new** |
| `crates/sync/src/repair.rs` | **new** |
| `crates/sync/src/gc.rs` | **new** |
| `crates/sync/src/health.rs` | **new** |
| `crates/sync/src/lease.rs` | **new** |
| `crates/sync/src/lib.rs` | `mod` declarations only (append) |
| `crates/web/src/api/repair_api.rs` | **pre-created stub** — fill in `route()`, already hooked up |
| `crates/tests/src/repair.rs` | **pre-created stub** — your test module |
| `AGENT-7.md` | this file |

## Work items

### P0 — silent loss becomes detectable

- [ ] **1. Background scrubber** (`crates/sync/src/scrub.rs`)
      A scheduled pass (reuse `scheduler.rs`'s tick pattern) that, for every chunk copy in
      every manifest, downloads/heads the artifact and re-verifies its BLAKE3 against the
      recorded `artifact_hash`. Results land in the `scrub_runs` table (already declared):
      per-provider `{ checked, ok, corrupt, missing, duration_ms }`. Corrupt/missing copies
      are reported **and** queued for repair — never just logged.
- [ ] **2. Repair / rebuild** (`crates/sync/src/repair.rs`)
      Given a finding: reconstruct from a surviving replica; if all replicas are gone but
      erasure coding has enough shards, decode; if neither, mark the chunk `unrecoverable`
      and surface it. **Provider loss** (config disabled/401/quota) → re-place every chunk
      that lived only there. Progress must be observable by AGENT-8's `ps`/`top` (expose a
      task handle — coordinate under *Requests*).
- [ ] **3. Reed–Solomon erasure coding** (`crates/erasure/**`)
      Replace replication-only `parity` with real RS: `k` data + `m` parity shards
      (`reed-solomon-erasure`), configurable per config; `parity: u8` in `SyncConfig`
      keeps meaning "how many losses it survives" so existing configs still parse.
      Test: encode → drop any `m` shards → decode → byte-identical.
- [ ] **4. Catalog replication + rebuild-from-remote (C4)**
      The superblock/manifest must be written to **≥2 providers**, not just local redb.
      Implement `rebuild_from_remote()`: list content-addressed chunks on each provider,
      reconstruct manifests by hash and position, and restore the catalog **after the local
      redb file is deleted** (that is the test). This is what makes the pool survive losing
      the laptop.

### P1 — the pool manages itself

- [ ] **5. Provider health** (`crates/sync/src/health.rs`)
      Score each config on latency, success rate, quota headroom and last-seen; persist to
      the `health` view of the config row. Unhealthy → quarantine (no new placement) but
      **never** a silent delete; recovery re-admits it.
- [ ] **6. Eviction + rebalance (C5)** — when free space on the volume is low, evict
      least-valuable copies (parity copies first, never the last copy of anything) using
      AGENT-6's refcounts; when a disk is attached, rebalance toward it.
- [ ] **7. Chunk GC (C6 / MISSING B4)** (`crates/sync/src/gc.rs`)
      Sweep content-addressed chunks that **no manifest references**, using AGENT-6's
      refcounts as the authority. Hard rule: **never delete a referenced chunk** — prove it
      with a test that writes data, runs GC, and asserts byte-identical read-back.
- [ ] **8. Multi-writer safety (F4)** (`crates/sync/src/lease.rs`)
      Volume lease (single active writer, TTL + renewal + steal-after-expiry) plus version
      vectors on catalog entries so two devices cannot silently clobber each other. A
      conflicting write must surface as a **conflict** through the existing
      `ConflictPolicy::Skip/Overwrite/KeepBoth` machinery, not a lost update.

### P2 — polish

- [ ] **9. Wire scrub/repair into the REST surface** — `repair_api::route` gets
      `GET /api/repair/status`, `POST /api/repair/run`, `GET /api/scrub/runs`,
      `POST /api/lease/acquire|release`. Auth-gated by default (`Authenticated`).
- [ ] **10. Surfacing** — expose counters for AGENT-8's terminal (`scrub`, `repair`
      commands) via `crates/os`'s boundary; request the hook under *Requests*, do not edit
      `crates/os/**`.

## Contracts (do not change unilaterally)

- Chunk addressing stays `cybermanju_sync::manifest::chunk_remote_path(blake3)` —
  content-addressed, so repair and GC share one namespace with today's data.
- `restore()` in `manifest.rs` must keep working for **existing** striped files written
  before RS landed (format `MANIFEST_VERSION` bumps only with a backward-compatible read).
- Tables: `scrub_runs`, `repairs`, `chunk_refs`, `leases` — already declared in
  `crates/db/src/database.rs`; use `Database::get_*_table()`.
- `repair_api::route(db, method, path_segments, body, origin) -> Option<String>` — `None`
  for anything that is not yours.
- Error prefixes: `integrity:` `unrecoverable:` `conflict:` `unsupported:`.

## Verification

```bash
export ORT_CACHE_DIR="$HOME/.cache/ort"
cargo fmt --all -- --check
cargo clippy -p cybermanju-erasure --all-targets -- -D warnings
cargo test  -p cybermanju-erasure
cargo test  -p cybermanju-sync
cargo test  -p cybermanju-tests repair
```

## Definition of done

- [ ] Every checkbox above is `[x]`
- [ ] Tests prove: scrub detects a corrupted copy · RS reconstructs after `m` losses ·
      catalog rebuilds with the local redb **deleted** · GC never deletes referenced data ·
      lease expiry allows a steal
- [ ] `scripts/os-acceptance.sh` **Tier 1** passes (detach provider A → volume still reads
      degraded → `repair` re-stripes → corrupt a copy → `scrub` finds it → wipe local DB →
      `rebuild-from-remote` restores the catalog)
- [ ] No edit outside *Files you own*
- [ ] **Log** section has a dated entry per unit of work

## Requests to the supervisor

<!-- e.g. "R7-1: start the scrub loop from docker/server/src/main.rs and src-tauri/src/lib.rs" -->

## Log

- _(append dated entries: `YYYY-MM-DD — item N — gate status`)_
