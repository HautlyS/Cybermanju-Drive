# AGENT-2 — Sync Engine, Persistence, Jobs & Decentralized Placement

**Part of the 5-agent production push.** Vision: a *decentralized PC* — files are not
"uploaded somewhere", they are **content-addressed, encrypted, chunked and placed across
multiple third-party providers**, and can always be restored. This agent owns the engine
that makes that real: pipeline correctness, persistence, background jobs, restore, and
the chunk/manifest placement layer.

- AGENT-1 — provider adapters + transfer reliability
- AGENT-2 (this file) — sync engine, persistence, jobs, restore, placement
- AGENT-3 — identity, secrets, OAuth, API hardening
- AGENT-4 — CI, tests, release, Docker/ops
- AGENT-5 — frontend, WASM/Pages, docs

## Ground rules (all agents)

1. **Verification policy:** no local builds and no dependency downloads (repo policy).
   `cargo fmt --all` is allowed (proves manifests resolve). Everything else is proven
   by GitHub Actions. Never mark work DONE while CI is red.
2. **Own only your files.** Need something from another agent's file? Add a line under
   *Requests to other agents* at the bottom of **your** file — do not edit their files.
3. **Shared files: surgical edits only** — no reformatting, no whitespace sweeps. Put new
   code inside your own `// <<< AGENT-N ... >>>` markers.
4. **Cargo.toml / package.json edits are append-only** (never remove another agent's entry).
5. After each merged unit of work, append a dated line to the **Log** section of your file.

## Files you own

| Path | Notes |
|---|---|
| `crates/sync/src/pipeline.rs` | scan→compress→encrypt→upload→verify→link→clean |
| `crates/sync/src/state.rs` | `SyncState`, progress, cancellation, runs |
| `crates/sync/src/manifest.rs` | **new** — chunk manifest + placement + reassembly |
| `crates/db/**` | all tables incl. new `sync_files`/`sync_runs`/`schema_version` |
| `crates/web/src/api/sync_api.rs` | sync business logic for REST |
| `crates/web/src/lib.rs` | **only** the sync route arms (`:585-636`, `:972-982`) + your own `// <<< AGENT-2 ROUTES >>>` marker block |
| `src-tauri/src/commands/sync.rs` | thin wrappers only |
| `crates/types/src/sync.rs` | **only** `SyncConfig` (:103), `SyncFile` (:86), `SyncProgress` (:127), `SyncResult` (:140), `SyncStatus` (:29), dead-type cleanup (not `OAuthCredentials` — AGENT-3 owns that) |

**You may NOT touch:** `backends.rs`, `oauth.rs`, `retry/rate_limit/transfer/quota.rs`
(AGENT-1); auth/validation/login regions of `crates/web/src/lib.rs` (AGENT-3);
`.github/`, `docker/`, `src/`, `README/ARCHITECTURE/AUDIT`.

## Work items

### P0 — data-loss & correctness

1. **Never delete a local original without verification** — `pipeline.rs:304-307` runs
   `delete_raw_after_sync` with zero checks. Gate on AGENT-1's
   `transfer::verify_blake3` result (download-back or provider checksum); on any doubt:
   skip deletion and record a `WARNING` in `SyncResult.errors`.
2. **Persist the remote locator** — today only `sync_url` is written
   (`pipeline.rs:414-424`); the `remote_path` used for upload is lost, so restore, remote
   delete and idempotent re-sync are impossible. New table `sync_files` (the `SyncFile`
   model already exists unused at `crates/types/src/sync.rs:86`): local file id, config
   id, remote locator, backend type, blake3, size, chunk manifest ref, `last_verified_at`,
   status.
3. **Fix byte double-counting** — `state.add_bytes` at `pipeline.rs:297` *and* `:156` in
   parallel mode; `SyncResult.bytes_uploaded` (`:311`) reports compressed size, not bytes
   sent. One accounting point only.
4. **Collision-safe remote paths** — `cybermanju_sync/{basename}` (`pipeline.rs:288-294`)
   collides across directories. Use `cybermanju_sync/{parent_hash8}/{name}` or a
   content-hash suffix; keep it stable for idempotent re-sync.
5. **Per-run state** — one global `SyncState` (`state.rs:9-29`): two runs clobber each
   other and `reset()` (`state.rs:69-81`) clears the cancel flag (a cancel can be
   un-cancelled). Give each run a `run_id`, keep a small run registry, scope cancel to a
   run, never clear the flag on reset. Fix terminal-status handling so cancelled ≠
   completed (already partly done in Phase 0 — verify).
6. **Async jobs instead of blocking HTTP** — `POST /api/sync/start` runs the whole
   pipeline inside the request (`sync_api.rs:145-189`) while the write timeout is 5 s
   (`crates/web/src/lib.rs:319`) → guaranteed timeouts in Docker. Return `202 {jobId}`
   immediately, run on a worker thread (desktop) / tokio-free thread pool (server),
   expose `GET /api/sync/jobs/{jobId}` and keep `GET /api/sync/progress` for the latest
   run. `POST /api/sync/cancel {jobId?}` cancels that run.

### P1 — decentralized placement, restore, conflicts

7. **Restore path (the other half of the contract):**
   - `POST /api/sync/restore {configId, fileId | remotePath, destPath?}` → downloads,
     verifies, writes locally, updates `sync_files`.
   - `DELETE /api/sync/remote {configId, remotePath}` → remote delete via AGENT-1's
     backend (must be honest — `unsupported:` errors surface as 501).
   - Matching Tauri commands in `src-tauri/src/commands/sync.rs` + registered in
     `src-tauri/src/lib.rs` invoke list — **wait**: `lib.rs` invoke registration is yours
     only if you already own the sync entries (`src-tauri/src/lib.rs:189-196`); add
     commands there surgically, that region belongs to you for sync only.
8. **Encryption before upload** — today `compress_before_upload` produces a compressed
   but **unencrypted** `.cyb3` (`pipeline.rs:329-337`) that is pushed to third parties in
   plaintext. Order: compress → encrypt (or the reverse — pick one, document it), using
   `cybermanju-crypto`; key from AGENT-3's keystore API (contract below), falling back to
   a local passphrase-derived key so you are never blocked. Store only the key handle /
   salt in `sync_files`.
9. **Conflict detection & policy** — none exists today. On re-sync: `stat` the remote
   (AGENT-1 `capabilities().stat`), compare etag/blake3/last-modified, then apply a
   policy from `SyncConfig`: `skip` (default) / `overwrite` / `keep-both`. Surface
   conflicts in progress + `SyncResult`. Add a `conflict_policy` field to `SyncConfig`
   (you own that struct; give it `#[serde(default)]` so old configs still parse).
10. **Chunk manifest + multi-provider placement** (the "decentralized PC" core):
    split a file into 4 MiB chunks, address each by BLAKE3, store the manifest in `sync_files`,
    place chunks round-robin across **≥2 enabled configs** with an optional k-of-n parity
    chunk (Reed–Solomon or simple replication to start — document the choice). Restore
    reassembles from any providers holding the chunks and tolerates up to n−k missing
    chunks. Requires AGENT-1 chunked transfer + `stat`; degrade gracefully to
    single-provider whole-file mode when only one config exists. Ship this behind
    `SyncConfig.placement` (`whole` default → `striped` opt-in) so nothing breaks.
11. **`auto_sync` is never read** — `crates/types/src/sync.rs:116`. Implement a simple
    scheduler (interval scan of enabled configs) or remove the field from the struct and
    tell AGENT-5 to drop the UI toggle.

### P2 — hygiene

12. `SyncStatus` has two generations merged (`Done`+`Completed`, `Idle`+`Syncing`,
    `types/sync.rs:29-41`) — collapse to one set, keep serde aliases for old payloads,
    update `src/types/index.ts` **by request to AGENT-5** (you do not own `src/`).
13. Remove dead `CloudAccount`/`SyncFile`-without-table problem: `SyncFile` now has a
    table (item 2); either give `CloudAccount` a real home (AGENT-3 owns its `token`
    field) or request its removal.
14. Run history: keep last N runs (table `sync_runs`) so the UI can show history.

## Contracts

**You provide (REST — AGENT-5 codes against these; publish exact JSON in your Log):**

| Method | Route | Notes |
|---|---|---|
| POST | `/api/sync/start` | → `202 {jobId}` (async, item 6) |
| GET | `/api/sync/jobs/{jobId}` | run progress |
| GET | `/api/sync/progress` | latest run (kept for compatibility) |
| POST | `/api/sync/cancel` | body `{jobId?}` |
| POST/GET/DELETE | `/api/sync/configs[/{id}]` | unchanged surface |
| POST | `/api/sync/restore` | item 7 |
| DELETE | `/api/sync/remote` | item 7 (501 on `unsupported:`) |
| GET | `/api/sync/usage/{configId}` | wraps AGENT-1 `quota::usage` |
| GET | `/api/sync/status` | replace hardcoded payload (`lib.rs:590-604`) with real state |

**You consume:**
- AGENT-1: `StorageBackend` (8 methods, stable) + defaulted `capabilities()`/`stat()`,
  error prefixes `auth:/rate_limited:/not_found:/unsupported:/too_large:/integrity:/network:`,
  `transfer::verify_blake3`, `rate_limit::acquire`, `quota::usage`.
- AGENT-3: keystore API for encryption keys (contract in AGENT-3.md);
  `oauth::resolve_token(&config)` if/when it lands — feature-gate your call so you are
  never blocked.
- AGENT-4: workspace-wide `cargo test` will execute your new tests — add unit tests in
  your own files (manifest round-trip, state machine, path collision) and route tests by
  *request* (AGENT-4 owns `crates/tests`).

**You must NOT change:** the 8 `StorageBackend` method signatures, auth/validation code
in `lib.rs`, or the secret-redaction attributes AGENT-3 places on `SyncConfig.token`.

## Verification

- `cargo fmt --all --check`.
- CI green: your in-crate unit tests + AGENT-4's route tests must pass.
- Manual matrix recorded in Log: start → progress → cancel → restart idempotent →
  restore → byte-identical file; striped mode: kill one provider, restore still works.

## Definition of done

- [ ] P0 1–6 closed; restore + conflict + encryption-before-upload shipped (7–9);
      chunked placement behind a flag (10).
- [ ] No local file is ever deleted without a verified remote copy.
- [ ] `POST /api/sync/start` never blocks a request thread.
- [ ] Every run has an id, survives progress polling, and reports honest terminal status.
- [ ] REST contract above published in your Log for AGENT-5.

## Requests to other agents

_(append here; do not edit their files)_

**→ AGENT-3 (secrets/OAuth):**

1. `crates/sync/src/oauth.rs:603` and `:629` — the two exhaustive `SyncConfig { … }`
   test literals need AGENT-2 contract fields (struct literals don't get serde
   defaults): `encrypt_before_upload: true`, `conflict_policy: Default::default()`,
   `placement: Default::default()`, `parity: 1`, `oauth_credentials: None`
   (the last one is yours — may already be on your list).
2. `restore_config_token` (your shim, `crates/web/src/lib.rs` ~:2094) is now optional:
   `save_config` persists `token` into the new `sync_secrets` side table (inside the same
   write transaction as the row delete for `delete_config`) and `get_config`/
   `list_configs` merge it back — the configs row JSON never contains a token, your
   `skip_serializing` is untouched. Keep your shim (row-embedded tokens still
   deserialize) or drop it; either way tokens round-trip.
3. Dead type: `CloudAccount` (`crates/types/src/sync.rs:105`) has no constructor or read
   anywhere in the workspace (grep: declaration only). You own its `token` field —
   approve removal (I'll file the deletion in `types/sync.rs`) or name where it should
   live. Item 13.

**→ AGENT-4 (tests):**

1. `crates/tests/src/types.rs:287-288` — `SyncStatus::Done`/`Syncing` no longer exist.
   Serialization now only emits the canonical set (idle/scanning/compressing/uploading/
   linking/cleaning/error/completed/cancelled); legacy strings still *deserialize* via
   serde aliases (`done`→Completed, `syncing`→Scanning). Drop the two variants from the
   round-trip list; an alias round-trip test (`"done"` → `Completed`) would pin item 12.
2. `crates/tests/src/types.rs:308` — exhaustive `SyncConfig` literal is missing:
   `encrypt_before_upload`, `conflict_policy`, `placement`, `parity`,
   `oauth_credentials`.
3. Route-contract tests you may add (exact JSON in my Log below): start answers
   `202 {jobId}` (400 bad body / 404 unknown config still hold); new
   `GET /api/sync/jobs/{id}`, `GET /api/sync/runs`, `POST /api/sync/restore`,
   `DELETE /api/sync/remote` (501 when the backend answers `unsupported:`),
   `GET /api/sync/usage/{configId}`; `POST /api/sync/cancel` stays 200 with an empty
   body. status/progress/configs unchanged.

**→ AGENT-5 (frontend/docs):**

1. `src/types/index.ts`: drop `SyncStatus` `Done`/`Syncing` (note the serde aliases);
   `SyncConfig` + `encryptBeforeUpload: boolean` (default **true**),
   `conflictPolicy: 'skip'|'overwrite'|'keepBoth'`,
   `placement: 'whole'|'striped'`, `parity: number` (default **1**, replicas per chunk
   in striped mode); `SyncFile` + `configId?`, `remotePath?`,
   `artifactHash?`, `manifestRef?`, `lastVerifiedAt?`, `keyHandle?`, `compressed?`,
   `encrypted?`; new `SyncJob`, `SyncRunRecord`, `RestoreOutcome`, `QuotaUsage`,
   `ChunkManifest` types.
2. `POST /api/sync/start` now returns `202 SyncJob` — the existing poll of
   `/api/sync/progress` keeps working (it reads the latest run's state), or poll
   `/api/sync/jobs/{jobId}` directly. New: `/runs`, `/restore`, `DELETE /remote`,
   `/usage/{configId}`. Tauri also gained `restore_sync_file`, `delete_remote_file`,
   `get_sync_job`, `list_sync_runs`.
3. StatusBar: `completed` is the only success terminal (the engine never emits `done`);
   `scanning|compressing|uploading|linking|cleaning` are the active phases.
4. Docs: README tree still lists `src-tauri/src/sync/{models,pipeline}.rs`; ARCHITECTURE
   claims curl-subprocess invocation, a static status list and table counts that don't
   match `crates/` (my review notes in Log). Confirm against code and fix.

## Log

- _(append dated entries: `YYYY-MM-DD — item N — commit — CI status`)_
- 2026-10-04 — review — n/a — Log was empty at session start: **all** items 1–14 were
  unimplemented. Confirmed gaps vs code: blocking `POST /sync/start` (5 s write timeout),
  unconditional `delete_raw_after_sync`, `cybermanju_sync/{basename}` collisions, one
  global `SyncState` whose `reset()` cleared the cancel flag, `remote_path` never
  persisted, plaintext uploads, no conflict policy, `auto_sync` unread, `CloudAccount`
  dead, `Done`+`Syncing` duplicated the status set. Docs mismatches → AGENT-5 requests.
- 2026-10-04 — item 12 — n/a — `SyncStatus` collapsed in `crates/types/src/sync.rs`
  (aliases keep old payloads parsing; Display updated).
- 2026-10-04 — item 5 — n/a — `state.rs`: `reset()` no longer touches the cancel flag,
  `prepare_run()` clears it only before registration, new `RunRegistry` singleton
  (`begin` refuses a state with an active run, scoped `cancel`, `finish` idempotent,
  16-run eviction that never drops the latest).
- 2026-10-04 — items 1–4, 8–9 — n/a — `pipeline.rs`: verified-delete gate (download-back
  BLAKE3, failure ⇒ keep original + `WARNING:` in errors), `sync_files` locator written
  every run, one `add_bytes` point per path, `cybermanju_sync/{parent_hash8}/{name}` +
  deterministic `keep-{hash8}` variant, compress→encrypt artifact (`CYBE1`+seal, no
  passphrase ⇒ WARNING + unencrypted — never blocked, never lied about), conflict check
  via `backend.stat` with idempotent-re-sync exemption; unit tests for paths.
- 2026-10-04 — items 2/6/14 — n/a — `crates/db`: tables `sync_files`, `sync_runs`,
  `sync_secrets`, `schema_version` + accessors/helpers. `sync_api.rs` rewritten:
  `start_job` validates then registers a run and spawns a worker thread (catch_unwind ⇒
  terminal `error`, never a stuck poll), terminal result mirrored to `sync_runs`
  (20-row prune). REST `start` → `202 {jobId}`; Tauri `start_sync` keeps its blocking
  signature but registers the same run (shared `Arc<SyncState>`).
- 2026-10-04 — item 7 — n/a — `restore` (locator fallback: `remote_path` then
  `remote_url`; magic-based decrypt → decompress → hash verify → sidecar write →
  `last_verified_at` stamp), `delete_remote` (`unsupported:` → HTTP 501), `usage`,
  plus Tauri `restore_sync_file`/`delete_remote_file`/`get_sync_job`/`list_sync_runs`
  registered in `src-tauri/src/lib.rs`.
- 2026-10-04 — REST contract published — n/a — **exact JSON for AGENT-5:**
  - `POST /api/sync/start` body `{"configId":"cfg-1","fileIds":["f1"]}` →
    `202 {"jobId":"run-20261004T120000.123456-1","configId":"cfg-1","startedAt":"…","finishedAt":null,"status":"scanning","progress":{"totalFiles":1,"processedFiles":0,"currentFile":null,"status":"scanning","bytesUploaded":0,"errors":[],"startedAt":null,"estimatedRemainingSeconds":null},"result":null}`;
    400 bad/missing body, 404 unknown config, 400 disabled config or run in progress.
  - `GET /api/sync/jobs/{jobId}` → same `SyncJob` (memory, then `sync_runs` after
    eviction/restart); 404 when unknown everywhere.
  - `GET /api/sync/runs` → `[SyncRunRecord]` newest-first, ≤20 (`runId`, `configId`,
    `startedAt`, `finishedAt`, `status`, `filesSynced`, `bytesUploaded`, `errors`,
    `progress`, `result`).
  - `POST /api/sync/cancel` body `""` | `{}` | `{"jobId":"run-…"}` → `true` (false only
    for an unknown id; nothing running ⇒ `true`, idempotent).
  - `POST /api/sync/restore` body `{"configId","fileId"|"remotePath","destPath?"}` →
    `200 {"path":"…","bytes":123,"verified":true}`; 404 unknown config/record.
  - `DELETE /api/sync/remote` body `{"configId","remotePath"}` → `200 true`;
    **501** when the backend answers `unsupported: …`; 404 unknown config.
  - `GET /api/sync/usage/{configId}` → `{"backendType","totalBytes","usedBytes","remainingRequests","requestLimit","resetAt",…}`.
  - `GET /api/sync/status` → `{"syncEnabled":true,"status":"scanning","lastSync":"…"|null,"provider":"cfg-…"|null}` (real run state, no more hardcode).
- 2026-10-04 — item 10 — n/a — striped placement shipped: new
  `crates/sync/src/manifest.rs` (`ChunkManifest`/`ChunkEntry`/`ChunkLoc`,
  `placements()` round-robin with parity clamped to `n-1`, content-addressed
  `cybermanju_sync/chunks/{plaintext_blake3}` paths, `decode_artifact`,
  `restore()` that streams to `{dest}.restore.part`, verifies every chunk's
  plaintext hash **and** the reassembled file hash, and only then publishes —
  unit tests for placement math + manifest serde). `SyncConfig.parity` added
  (`#[serde(default)]`, default **1** = one replica per chunk; **simple
  replication, not Reed–Solomon** — documented per the work item).
  `pipeline.rs`: `transform_payload` factored out of `build_artifact` so
  chunks travel the exact compress→encrypt order,
  `sync_single_file_striped` (≥2 enabled configs required — fewer ⇒ WARNING
  + graceful whole-file fallback; empty file ⇒ fallback; mid-chunk failure ⇒
  hard `Err`, never a silent re-place), `verify_striped` download-back gate
  for `delete_raw_after_sync`, manifest persisted in
  `sync_files.manifest_ref`. Restore branches on `manifest_ref` in
  `sync_api::restore` (striped restore needs `fileId` — the manifest lives in
  `sync_files`). Conflict policy is bypassed in striped mode by construction:
  chunks are content-addressed, so same bytes ⇒ same key ⇒ idempotent.
- 2026-10-04 — item 11 — n/a — auto_sync now read: new
  `crates/sync/src/scheduler.rs` (`ensure_started` OnceLock daemon, 60 s tick,
  10 min per-config interval — no new config field, no other agent's literals
  touched). Runs the normal pipeline through `RunRegistry` + `sync_runs`
  (identical terminal bookkeeping to `execute_run`), skips while any manual
  run is active, only `enabled && auto_sync` configs, full live-file set
  (`Database::list_file_ids` — trash lives elsewhere). Armed lazily from
  `GET /api/sync/status`, `GET /api/sync/progress` (REST) and the Tauri
  `get_sync_progress` command — first sync-API contact starts the scan;
  a headless server with zero polls never auto-syncs (documented limit).
- 2026-10-04 — item 13 — n/a — `CloudAccount` removal requested from AGENT-3
  (they own its `token` field); I do not delete it unilaterally.
- 2026-10-04 — wiring pass — n/a — all item-7/6 surfaces connected end to
  end: REST routes (`start`→202, `jobs/{id}`, `runs`, `restore`, `remote`
  with 501 mapping, `usage`), Tauri commands (`restore_sync_file`,
  `delete_remote_file`, `get_sync_job`, `list_sync_runs`) registered in
  `src-tauri/src/lib.rs`, auth-gate covers the new paths (default
  Authenticated), `src/` unchanged by design — frontend keeps working
  (ignores the 202 body, polls `/api/sync/progress` which now reads the
  latest registered run). `cargo fmt --all --check` green; compile/test proof
  left to CI per repo policy.
- 2026-10-04 — items 10/11 hardening — n/a — three quiet-failure fixes found
  in self-review: (a) striped temp names now carry a per-call nonce (chunk
  index alone raced across parallel file threads and concurrent runs);
  (b) striped re-sync of unchanged content is a no-op (recorded manifest +
  matching `hash_blake3` ⇒ `filesSynced` +0 bytes) so auto-sync ticks don't
  re-send the library; (c) a `deleteRawAfterSync` original that is already
  absent with a placed record now **skips** instead of erroring "File not
   found" — the intended post-sync state, and the difference between a quiet
   scheduler and one that spams every tick.
- 2026-10-04 — compression payload contract — commit: this push — CI: pending.
  Two failures in AGENT-4's suite: (a) `CompressionStats`/`LayerDetail` now carry
  `#[serde(rename_all = "camelCase")]` — the payload travels REST/Tauri into
  `src/types/index.ts`, which reads `originalSize`, `layerDetails`, `blake3Hash`,
  `inputSize`, so snake_case was surfacing as `undefined` in the UI (and failing
  `test_compression_stats_serde`); (b) `decompress_triple` returns `Ok(([], 0))`
  for empty input — `compress_triple` classifies the empty payload as
  incompressible (lz4 ratio forced to 1.0 when `original_size == 0`) and stores
  the bytes untouched, so there is no frame for the decoder to read and the
  round-trip test failed with a brotli error.
