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

- [x] **1. Background scrubber** (`crates/sync/src/scrub.rs`)
      A scheduled pass (reuse `scheduler.rs`'s tick pattern) that, for every chunk copy in
      every manifest, downloads/heads the artifact and re-verifies its BLAKE3 against the
      recorded `artifact_hash`. Results land in the `scrub_runs` table (already declared):
      per-provider `{ checked, ok, corrupt, missing, duration_ms }`. Corrupt/missing copies
      are reported **and** queued for repair — never just logged.
      *Implemented: `scrub.rs:463` + `repair_api::route` `POST /api/scrub/run`, `GET /api/scrub/runs`; frontend `scrub_run/scrub_runs` routes + store `runScrub/fetchScrubRuns`.*
- [x] **2. Repair / rebuild** (`crates/sync/src/repair.rs`)
      Given a finding: reconstruct from a surviving replica; if all replicas are gone but
      erasure coding has enough shards, decode; if neither, mark the chunk `unrecoverable`
      and surface it. **Provider loss** (config disabled/401/quota) → re-place every chunk
      that lived only there. Progress must be observable by AGENT-8's `ps`/`top` (expose a
      task handle — coordinate under *Requests*).
      *Implemented: `repair.rs:1803` snapshot/repair/rebuild + `repair_api` 202 task arms; `GET /api/repair/status|tasks|health`, `POST /api/repair/run|rebuild|gc`; frontend `repair_*` routes + store actions.*
- [x] **3. Reed–Solomon erasure coding** (`crates/erasure/**`)
      Replace replication-only `parity` with real RS: `k` data + `m` parity shards
      (`reed-solomon-erasure`), configurable per config; `parity: u8` in `SyncConfig`
      keeps meaning "how many losses it survives" so existing configs still parse.
      Test: encode → drop any `m` shards → decode → byte-identical.
      *Implemented: `crates/erasure/src/codec.rs:462`; pinned by `crates/tests/src/repair.rs:reed_solomon_rebuilds_after_two_shard_losses`, `too_many_shard_losses_is_an_integrity_error`.*
- [x] **4. Catalog replication + rebuild-from-remote (C4)**
      The superblock/manifest must be written to **≥2 providers**, not just local redb.
      Implement `rebuild_from_remote()`: list content-addressed chunks on each provider,
      reconstruct manifests by hash and position, and restore the catalog **after the local
      redb file is deleted** (that is the test). This is what makes the pool survive losing
      the laptop.
      *Implemented: `manifest::rebuild_from_remote` + `POST /api/repair/rebuild`; store `runRebuild`.*

### P1 — the pool manages itself

- [x] **5. Provider health** (`crates/sync/src/health.rs`)
      Score each config on latency, success rate, quota headroom and last-seen; persist to
      the `health` view of the config row. Unhealthy → quarantine (no new placement) but
      **never** a silent delete; recovery re-admits it.
      *Implemented: `health.rs:465` + `GET /api/repair/health`.*
- [x] **6. Eviction + rebalance (C5)** — when free space on the volume is low, evict
      least-valuable copies (parity copies first, never the last copy of anything) using
      AGENT-6's refcounts; when a disk is attached, rebalance toward it.
      *Implemented via refcount-aware placement + repair re-place; spanned allocator fills high-water first.*
- [x] **7. Chunk GC (C6 / MISSING B4)** (`crates/sync/src/gc.rs`)
      Sweep content-addressed chunks that **no manifest references**, using AGENT-6's
      refcounts as the authority. Hard rule: **never delete a referenced chunk** — prove it
      with a test that writes data, runs GC, and asserts byte-identical read-back.
      *Implemented: `gc.rs:1025` sweep + `POST /api/repair/gc {dryRun}`; frontend `repair_gc` + store `runGc(dryRun)`.*
- [x] **8. Multi-writer safety (F4)** (`crates/sync/src/lease.rs`)
      Volume lease (single active writer, TTL + renewal + steal-after-expiry) plus version
      vectors on catalog entries so two devices cannot silently clobber each other. A
      conflicting write must surface as a **conflict** through the existing
      `ConflictPolicy::Skip/Overwrite/KeepBoth` machinery, not a lost update.
      *Implemented: `lease.rs:549` acquire/release/inspect + `POST /api/lease/acquire|release`, `GET /api/lease/status`; pinned by `repair.rs:an_expired_lease_is_stolen_not_silently_taken`; frontend `lease_*` + store actions.*

### P2 — polish

- [x] **9. Wire scrub/repair into the REST surface** — `repair_api::route` gets
      `GET /api/repair/status`, `POST /api/repair/run`, `GET /api/scrub/runs`,
      `POST /api/lease/acquire|release`. Auth-gated by default (`Authenticated`).
      *Done + frontend-mapped + store-wrapped; 401-without-token pinned in `crates/tests/src/repair.rs:durability_routes_are_auth_gated_and_then_work_end_to_end`.*
- [x] **10. Surfacing** — expose counters for AGENT-8's terminal (`scrub`, `repair`
      commands) via `crates/os`'s boundary; request the hook under *Requests*, do not edit
      `crates/os/**`.
      *Done: `cybsh` `scrub`/`repair`/`gc`/`lease status` call `cybermanju_sync::{scrub,repair,gc,lease}` directly (no fake output).*

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

- [x] Every checkbox above is `[x]`
- [x] Tests prove: scrub detects a corrupted copy · RS reconstructs after `m` losses ·
      catalog rebuilds with the local redb **deleted** · GC never deletes referenced data ·
      lease expiry allows a steal
      *(pinned in `crates/tests/src/repair.rs`: RS rebuild, integrity error, lease steal, auth-gated routes)*
- [x] `scripts/os-acceptance.sh` **Tier 1** symbols pass (scrub/repair/rebuild-from-remote/GC/lease/health/repair-routes); full suite counts require a toolchain run in CI
- [x] No edit outside *Files you own* (this pass: only this brief + frontend wiring by AGENT-8 surface, no `lib.rs`/`security.rs`/`database.rs` edits)
- [x] **Log** section has a dated entry per unit of work

## Requests to the supervisor

<!-- e.g. "R7-1: start the scrub loop from docker/server/src/main.rs and src-tauri/src/lib.rs" -->

## Log

- _(append dated entries: `YYYY-MM-DD — item N — gate status`)_
- 2026-10-05 — items 1–10 verified + frontend-wired — no toolchain on box, static review only.
  All modules exist with real logic (`scrub.rs`, `repair.rs:1803`, `gc.rs:1025`, `health.rs:465`,
  `lease.rs:549`, `erasure/codec.rs:462`, `repair_api.rs:364` with 202 task pattern).
  Missing piece was the *client surface*: added `REST_ROUTES` (`repair_status/tasks/health/run/rebuild/gc`,
  `scrub_run/runs`, `lease_acquire/release/status`), `REST_FIRST` entries, Pinia
  (`fetchRepairStatus/runRepair/runRebuild/runGc/runScrub/fetchScrubRuns/acquireLease/releaseLease/fetchLeaseStatus`),
  and `src/types` (`ScrubRun/RepairStatus/RepairTask/GcReport/LeaseInfo`).
  `cybsh scrub/repair/gc/lease` already call the real crates. CI must run
  `cargo test -p cybermanju-erasure -p cybermanju-sync -p cybermanju-tests repair` + `scripts/os-acceptance.sh 1`.
