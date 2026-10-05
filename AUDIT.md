# Cybermanju Drive — Stub / Missing / Working Audit

> Generated from a full static review of the repository (no local builds; CI is the
> verification path). Every finding below is referenced by `file:line`.

---

## 1. What is genuinely working

| Area | Evidence |
|------|----------|
| `crates/types` | Real `serde` schema for FileNode/Account/Collection/… with camelCase renaming + tests (`crates/tests/src/types.rs`, 21 tests) |
| `crates/db` | Real `redb` with **16** table definitions, parent index, trash/audit/versions/share tables (`crates/db/src/database.rs`) |
| `crates/search` | Real **Tantivy**: schema field handles, `delete_term`+`add_document`+`commit`, BM25 `QueryParser`, term-dictionary prefix suggest (`crates/search/src/lib.rs`) |
| `crates/compression` | Real triple pipeline LZ4 → ZSTD → Brotli with per-layer stats and incompressible short-circuit (`crates/compression/src/triple.rs:54`) |
| `crates/crypto` | Real **ML-KEM-1024** (`pqcrypto-mlkem`), **ML-DSA-44/65/87** (`ml-dsa`), ChaCha20Poly1305 + HKDF-SHA256 (`crates/crypto/src/pqc.rs`) |
| `crates/faces` | Real clustering (bruteforce, simhash, chinese whispers, HDBSCAN, auto) + real ONNX SCRFD/MobileFaceNet when `onnx-face` (`crates/faces/src/lib.rs:1669`) |
| `crates/web` | Real hand-rolled HTTP/1.1 server: JWT (argon2+HS256), rate limit, CORS, 30+ routes |
| `src-tauri` | **87** registered commands, all real handlers; sync backends do real `reqwest` calls (GitHub/GitLab/Drive/Photos/Telegram/Local) |
| Frontend | 40 Vue components, 72 Pinia actions — everything routes through `invoke()`, **no hardcoded demo datasets** |
| Tests | 121 `#[test]` in `crates/tests` + web/route tests |

---

## 2. Stub / broken / missing findings

### F1 — GitHub Pages ships an app with no backend (critical)
`npm run build:wasm` → `vite build --config vite.config.wasm.ts`. **No Rust→WASM
compilation exists anywhere.** `crates/drive-wasm` is never built, never imported by
the frontend. On Pages, `invoke()` falls through to `http://localhost:3456`
(`src/composables/useTauri.ts:50`) → network error → empty UI.
`crates/drive-wasm` itself only exposes primitive crypto/compression helpers.

### F2 — Web/Docker mode 401s on every endpoint (critical)
`crates/web/src/lib.rs:517` requires a JWT for **all** `/api/*` except
health/login/register. The frontend never sends one:
`src/stores/app.ts:16` imports `setAuthToken` but **never calls it**; the `authToken`
ref at `src/stores/app.ts:74` is dead code. Login succeeds, the returned token is
dropped. Nothing auto-opens the login popup in web mode either.

### F3 — 20 frontend commands have no REST route
They are neither in `REST_ROUTES` nor in `WRITE_ONLY_COMMANDS`
(`src/composables/useTauri.ts:144`, `:342`) → `invoke()` throws
`Unknown Tauri command`:
`search_files_paginated`, `list_trash`, `restore_from_trash`, `empty_trash`,
`delete_from_trash`, `get_audit_log`, `list_file_versions`, `create_file_version`,
`revert_file_version`, `snapshot_all_versions`, `create_user`, `delete_user`,
`update_user_role`, `batch_delete`, `batch_encrypt`, `batch_compress`,
`generate_share_link`, `list_share_links`, `import_from_url`, `rebuild_parent_index`.

### F4 — Hardcoded REST stubs
- `crates/web/src/lib.rs:598` — `GET /api/sync/configs` returns literal `[]`
  ("sync configs not yet persisted to a table") **even though
  `SYNC_CONFIGS_TABLE` exists** (`crates/db/src/database.rs:27`).
- `crates/web/src/lib.rs:601` — `GET /api/sync/status` returns a fixed
  `syncEnabled:false` payload.

### F5 — `/api/search` is a substring scan, not Tantivy
`crates/web/src/lib.rs:962` `search_files` does `to_lowercase().contains()` over the
files table. README advertises "Tantivy … BM25 ranking".

### F6 — Broken command name
`src/components/FilePermissionsPanel.vue:94` invokes `list_file_permissions`;
the backend registers `get_file_permissions` (`src-tauri/src/lib.rs:175`).

### F7 — Face detection fabricates results
`crates/faces/src/lib.rs:1511` — when ONNX fails **or finds zero faces**, control
falls through to `detect_faces_blake3_fallback` which invents 1–3 pseudo-faces from
a BLAKE3 hash and then clusters them as if they were real detections.

### F8 — `tree_sitter` has no tree-sitter
`src-tauri/src/tree_sitter/mod.rs:5` admits "For now, heuristic regex-based
extraction". README claims `tree-sitter 0.24` as a dependency (README:104).

### F9 — Telegram backend silently lies
`src-tauri/src/sync/backends.rs:1547` `delete_file` returns `Ok(())` as a no-op;
`list_files` returns an empty `Vec`. The UI reports success.

### F10 — CI never runs the test suite
`.github/workflows/ci.yml:52-58` runs `cargo fmt/clippy/test` with
`working-directory: src-tauri`, which resolves to the *current package* only. The
whole `crates/tests` crate (121 tests) is never executed. Stale
`src-tauri/Cargo.lock` sits beside the workspace lock.

### F11 — Docker/ZimaOS advertises features it does not ship
`Dockerfile` stage 2 builds only `docker/server` (redb + argon2 + jsonwebtoken + uuid).
No Tantivy, no PQC, no compression, no faces — yet `docker-compose.yml` `x-casaos`
description promises all of them.

### F12 — Three byte-identical 1702-line routers
`crates/web/src/lib.rs`, `src-tauri/src/web_dashboard/mod.rs`,
`docker/server/src/web_dashboard.rs` have the same MD5. Every REST fix must be
applied 3× or the copies drift.

### F13 — Frontend mapping gaps (non-fatal but wrong)
- `dashboard_status` → `/api/health` and fabricates `activeConnections: 0`
  (`src/composables/useTauri.ts:324`).
- 19 registered backend commands are never called from the UI (dead surface), and
  `WRITE_ONLY_COMMANDS` blocks 31 more in web mode.

### F14 — Docs drift
README's project tree still lists `src-tauri/src/search/tantivy_index.rs`,
`crypto/pqc.rs`, `compression/triple.rs`, `faces/mod.rs`, `db/mod.rs` — all of which
moved to `crates/` (the `src-tauri` copies are 1–2 line re-exports).
`worklog.md` claims "Zero mocks remaining" while F1–F6 exist.

### F15 — Desktop opens the same redb file twice (startup panic risk) *(found during Phase 0)*
`src-tauri/src/lib.rs:42` opens `cybermanju.db` for `AppState`, then
`src-tauri/src/lib.rs:83` builds `WebDashboard::new(port, "cybermanju.db")`, which
opened the **same file again**
(`crates/web/src/lib.rs:114`, `RedbDb::open(...).or_else(create)`). redb takes an
exclusive file lock (fs4), so the second `expect("Failed to open web dashboard
database")` panics on startup. The same double-open exists in
`src-tauri/src/commands/dashboard.rs:94`.

### F16 — `start_dashboard` spawned a second server on the same port
`src-tauri/src/commands/dashboard.rs:94` created a *new* `WebDashboard` inside a
thread even though `run()` had already started one on `DEFAULT_PORT`, so the second
bind always failed (and re-triggered F15).

### F17 — Sync cancel and progress were wired to nothing
- `cancel_sync` (`src-tauri/src/commands/sync.rs`) set `SyncState.cancel_flag`, but
  `SyncPipeline` polled its **own** private `cancelled` flag
  (`src-tauri/src/sync/pipeline.rs:34`) — cancellation never reached the pipeline.
- `get_sync_progress` read `SyncState.progress`, which the pipeline never wrote
  (it updated a private `SyncProgressInner`) — pollers only ever saw the
  start/end snapshots.

### F18 — Cancelled sync reported as `completed`
`start_sync` unconditionally overwrote `status = Completed` after
`pipeline.sync_all` returned (`src-tauri/src/commands/sync.rs:165-171`), including
when the run had been cancelled. The frontend poller also never stops on
`completed`/`cancelled` (`src/stores/app.ts` `pollSyncProgress`) — it re-arms its
timer forever.

### F19 — `import_from_url` downloads bytes and throws them away
`src-tauri/src/commands/import.rs:501-505` reads the whole response body, then never
writes it to disk — the returned `FileNode` describes a file that does not exist.

### F20 — Argon2 + `register_user` implemented twice
`crates/web/src/lib.rs` (`argon2_hash`/`register_user_web`) and
`src-tauri/src/commands/users.rs` (`argon2_hash_password`/`register_user`) had two
independent copies of the same logic. Parameters matched by luck, not by design.
*Closing: both now call `cybermanju_web::api::users`.*

### F21 — `DELETE /api/files/{id}` bypassed the trash
The generic REST helper `delete_by_id` removed the redb row outright, while the Tauri
`delete_file` command soft-deleted to the trash — the same UI action permanently
destroyed data in web mode and only trashed it on desktop.
*Closing: `DELETE /api/files/{id}` now calls `api::files::delete`.*

### F22 — Share links pointed at a route that did not exist
`ShareLinkResult.url` was `/api/share/{token}` (Tauri) and the router had **no**
`/api/shared/{token}` handler at all, so every generated link 404'd. The auth gate
already exempted `["api","shared",_]`, but nothing served it.

---

## 3. Decisions taken

| # | Question | Decision |
|---|----------|----------|
| D1 | Docker backend scope | **Add the real crates** (`cybermanju-search`, `cybermanju-crypto`, `cybermanju-compression`) to `docker/server` so the advertised search/encryption/compression are real server-side. Faces stay desktop-only (ONNX runtime + model downloads do not belong in the image) — wording corrected. |
| D2 | Code parsing module | **Add real tree-sitter** (core + rust/python/js/ts/go/c/cpp grammars) behind a default-on `real-treesitter` feature, with the heuristics kept as fallback. |
| D3 | Triple router copy | **Single source of truth** — delete the two copies; both depend on `cybermanju-web`. |
| D4 | WASM depth | **Full command surface** — `drive-wasm` implements the same dispatcher as Tauri, persisted to localStorage. |

---

## 4. Implementation phases

| Phase | Work | Closes |
|-------|------|--------|
| **0** | Single-source the router: new `cybermanju-sync` crate, `src-tauri` + `docker/server` depend on `cybermanju-web`, delete both copies, add `docker/server` to the workspace, rewrite Dockerfile stage 2, **one shared DB handle** for app + dashboard, **one shared sync state** | F12, F15, F16, F17, F18 |
| **1** | Auth wiring (`setAuthToken`, token persistence, 401→login), **single-source business logic** in `cybermanju_web::api::*` (Tauri commands become thin wrappers, REST routes call the same fns), the 20 missing routes + write routes (create/rename/move), `list_file_permissions`→`get_file_permissions`, real sync config/status routes, fix `dashboard_status` mapping | F2, F3, F4, F6, F13, F20, F21, F22 |
| **2** | Real search everywhere: `/api/search` + suggest use the shared index (`cybermanju_search` is already a plain dependency of `cybermanju-web`, no feature gate); `import_from_url` written to disk | F5, F19 |
| **3** | Real tree-sitter behind `real-treesitter` | F8 |
| **4** | WASM: store + BM25 + full command dispatcher + seed + localStorage persistence; `wasm-pack` build script; third transport in `useTauri.ts`; Settings shows active transport | F1 |
| **5** | Remove face pseudo-embedding fabrication; Telegram returns explicit unsupported errors | F7, F9 |
| **6** | CI: workspace-wide `fmt/clippy/test`, wasm build assertions, `cargo check --target wasm32-unknown-unknown`, delete stale lockfile | F10, F11(CI) |
| **7** | README/ARCHITECTURE/docker-compose/worklog alignment | F11, F14 |

**Verification policy:** no local builds or dependency downloads — every phase is
verified by the GitHub Actions pipeline (rust-check → docker-build → wasm-build →
platform builds → deploy-pages), plus new route/unit tests added under
`crates/tests` and `src-tauri`.

---

## 5. Status log

> Updated as phases land. **Nothing is verified until GitHub Actions is green** —
> no local builds are run (by policy).

### Phase 0 — single-source the router — ✅ DONE (code + self-review)

- New crate `crates/sync` (`cybermanju-sync`): `SyncState` (poison-recovering
  progress + cancellation), `SyncPipeline` (reports `Cancelled`, not `Completed`),
  backends, oauth.
- Deleted the three duplicate routers: `src-tauri/src/web_dashboard/{mod.rs→shim}`,
  `docker/server/src/web_dashboard.rs` → both now `pub use cybermanju_web::*`.
- One redb handle: `AppState.db: Arc<RwLock<Database>>` handed to
  `WebDashboard::new_shared`; `start_dashboard` reuses the managed instance (F16).
- One `Arc<SyncState>` shared by Tauri commands and the REST layer (F17/F18).
- `docker/server` joined the workspace; Dockerfile stage 2 rebuilt
  (toolchain `build-base pkgconf cmake perl` for `ring`).
- `cargo fmt --all` run → workspace-wide `--check` passes (also proves every
  manifest and path dep resolves).

### Phase 1 — REST parity + auth wiring — 🟡 IN PROGRESS

| Sub-item | State |
|----------|-------|
| `crates/web/src/api/` shared modules (`files`, `trash`, `versions`, `audit`, `share`, `batch`, `users`, `accounts`, `collections`, `sync_api`, `search_api`) | ✅ written |
| `handle_request` route arms — ~40 new routes (files write ops, trash, versions, audit, share + `/api/shared/{token}`, batch, accounts, collections, users roles, sync config/CRUD, search/suggest/paginated) | ✅ written |
| Lockless sync dispatch **before** the DB lock (start/cancel/progress/status/test/remote-files) — avoids the deadlock of holding `RwLock` across a pipeline run | ✅ written |
| `json_body!` / `json_ok` / `api_response` helpers; 404-vs-400 mapping | ✅ written |
| `register_user_web` now delegates to `api::users::register` (kills F20); `delete_by_id` (F21) and the old substring-only `search_files` deleted | ✅ written |
| `src-tauri/src/commands/{trash,versions,audit,share,batch,files}.rs` → thin wrappers | ✅ written |
| `src-tauri/src/commands/{users,accounts,collections,sync,search}.rs` → thin wrappers | ✅ written |
| `AppState.tantivy_index` → `Arc<RwLock<SearchIndex>>` + `dashboard.set_search_index(...)` (search routed through the shared index; Docker falls back to substring scan) | ✅ written |
| Frontend `REST_ROUTES`: +35 mappings (files write ops, trash, versions, audit, share links, batch, users, accounts, collections, sync, paginated search); `WRITE_ONLY_COMMANDS` shrunk to 18 genuinely desktop-only commands | ✅ written |
| Frontend auth: `setAuthToken` persists to `localStorage` + restored on load; any HTTP 401 dispatches `cybermanju:unauthorized` → `showLoginPopup` (F2) | ✅ written |
| `LoginPopup` stores the JWT via `store.setSessionToken` after login (F2) | ✅ written |
| `dashboard_status` → `GET /api/dashboard/status` (was `/api/health`, lied about port/connections) | ✅ written |
| `pollSyncProgress` stops on every terminal status incl. `completed`/`cancelled` (F18) | ✅ written |
| `SyncStatusType` extended to match the Rust enum (`syncing`/`completed`/`cancelled`); `StatusBar.isSyncActive` now only flags live statuses | ✅ written |
| `FilePermissionsPanel` calls `get_file_permissions` (F6 — the old name did not exist) | ✅ written |
| Mapping audit: all 69 `invoke('...')` call sites resolve to a REST route or an explicitly write-only command; no unmapped commands | ✅ written |
| Unused-import sweep across converted wrappers (no `#[allow]` needed; only `versions.rs`'s dead alias block removed) | ✅ written |
| `cargo fmt --all` after the edits (also proves manifests resolve) | ✅ written |
| CI run | ⬜ push pending — this commit is the first Phase 1 build |

### Remaining phases

2 → search/import · 3 → tree-sitter · 4 → WASM/GH Pages · 5 → faces/Telegram
honesty · 6 → workspace-wide CI + stale `src-tauri/Cargo.lock` deletion · 7 → docs.

### OS push — 2026-10-05 (no toolchain on box; static review, CI must prove)

- Disk/volume (AGENT-6): done + verified in-brief; Tauri `commands::disk::*7` registered; `REST_FIRST` covers desktop.
- Durability (AGENT-7): modules + 202 task routes existed; this pass added the missing client
  surface (`repair_status/tasks/health/run/rebuild/gc`, `scrub_run/runs`, `lease_acquire/release/status`
  in `REST_ROUTES` + `REST_FIRST` + Pinia + `src/types`), ticked brief `[x]`.
- OS/terminal (AGENT-8): `cybsh`/Kernel/tasks/compute/os_api/WASM/dock/hotkey/statusbar existed;
  this pass added sync-job + durability store surface, full `SyncPanel.vue` wizard
  (create/test/start/cancel/quota/restore/remote-delete/browse, OAuth, striped placement,
  `describeSyncError` hints), Settings active-transport display, `VITE_TRANSPORT` typing,
  `user-scalable=no` removal, WASM base `/cybermanju-drive/`.
- Honesty: F7 closed (no BLAKE3 pseudo-faces — empty set + log), F19 closed
  (`import_from_url` persists bytes to `imports/{id}_{name}` + `original_path`),
  tree-sitter claim corrected in README (heuristic regex, grammar integration pending),
  `cybsh sync start` refusal now points at `POST /api/sync/start → 202` (R8-1 resolved as documented).
- Docs: `docs/OPERATIONS.md` added; README + ARCHITECTURE §12 cover disks/volume/cybsh/tasks/compute;
  version single-source verified (`package.json` truth; `/api` version uses `CARGO_PKG_VERSION`).
- Left for CI (no `cargo`/`node_modules` on this box): workspace fmt/clippy/test,
  `npm run typecheck/lint`, `scripts/os-acceptance.sh 0/1/2`, `scripts/check-version.sh`.
