# AGENT-5 — Frontend, WASM/Pages & Documentation

**Part of the 5-agent production push.** The backend agents make the *decentralized PC*
real; this agent makes it **usable and truthfully documented**: a working sync control
surface (create provider configs, connect OAuth, run/monitor/restore), a GitHub Pages
build that is not an empty shell (AUDIT F1), frontend production hygiene, and docs that
match the code (AUDIT F14).

- AGENT-1 — provider adapters + transfer reliability
- AGENT-2 — sync engine, persistence, jobs, restore, placement
- AGENT-3 — identity, secrets, OAuth, API hardening
- AGENT-4 — CI, tests, release, Docker/ops
- AGENT-5 (this file) — frontend, WASM/Pages, docs

## Ground rules (all agents)

1. **Verification policy:** no local builds and no dependency downloads (repo policy).
   `cargo fmt --all` is allowed for Rust; for TS, `npx vue-tsc --noEmit` if
   `node_modules` already exists (AGENT-4's `wasm-build` job runs it regardless).
   Everything else is proven by GitHub Actions. Never mark work DONE while CI is red.
2. **Own only your files.** Need something from another agent's file? Add a line under
   *Requests to other agents* at the bottom of **your** file — do not edit their files.
3. **Shared files: surgical edits only** — no reformatting, no whitespace sweeps. Put new
   code inside your own `// <<< AGENT-N ... >>>` markers.
4. **Cargo.toml / package.json edits are append-only** — in `package.json` you own only
   `dependencies` (AGENT-4 owns `scripts` + `devDependencies`).
5. After each merged unit of work, append a dated line to the **Log** section of your file.

## Files you own

| Path | Notes |
|---|---|
| `src/**` | all 40 Vue components, Pinia store, composables, types |
| `crates/drive-wasm/**` | the WASM command dispatcher (AUDIT F1/D4) |
| `vite.config.ts`, `vite.config.wasm.ts` | builds + base path |
| `env.d.ts`, `index.html`, `src-tauri/tauri.conf.json` (frontend-facing bits only) | |
| `README.md`, `ARCHITECTURE.md`, `AUDIT.md`, `worklog.md` | **only agent allowed** to edit these |
| `src-tauri/src/tree_sitter/mod.rs` | F8 (tree-sitter vs heuristics) if you take it |
| `package.json` | `dependencies` only, append-only |

**You may NOT touch:** any `crates/**` except `crates/drive-wasm`; any `src-tauri/src/**`
except `tree_sitter/mod.rs` (and even then only after checking AGENT-1..4 requests);
`.github/`, `docker/`, `crates/tests`, `AGENT-1..4.md`.

## Work items

### P0 — F1: GitHub Pages ships an app with no backend

1. `npm run build:wasm` compiles **no Rust**: `crates/drive-wasm` (13 lines of crypto
   helpers) is never built or imported; on Pages `invoke()` falls through to
   `http://localhost:3456` (`src/composables/useTauri.ts:62-71`) → empty UI. Implement
   decision **D4** (`AUDIT.md:166`): `drive-wasm` grows the same command dispatcher the
   Tauri shell has, persisted to `localStorage`, including a BM25-lite search over the
   stored file tree so search works offline (no Tantivy in WASM).
2. Third transport in `useTauri.ts`: `tauri | rest | wasm` — detect (WASM build forces
   `__TAURI__ = false`, `vite.config.wasm.ts:45-50`), add the wasm branch to `invoke()`
   (`:626-667`), keep `REST_ROUTES` (`:168`) as the model for the dispatcher's command
   table (mirror it — do not fork it; read it from a shared module if practical).
3. Seed + Settings: Settings page shows the **active transport**; first-run seed of a
   demo tree so the Pages build is never empty.
4. **Base path bug**: `vite.config.wasm.ts:13` hardcodes `/Cybermanju-Drive/` but the
   repo slug is `cybermanju-drive` (README:127) → assets 404. Fix the base (or make it
   env-driven from `VITE_BASE`).
5. `build:wasm` must actually build the wasm crate (`wasm-pack build crates/drive-wasm`
   → `--target web` into `public/`); AGENT-4's CI job asserts it compiles — coordinate
   via *Requests* so the two halves land together.

### P0 — sync control surface (today: read-only panel, zero wired actions)

6. **Provider picker + config wizard**: `SYNC_BACKEND_INFO`
   (`src/types/index.ts:435-472`, all 6 providers) is defined and **never imported**;
   `SyncPanel.vue` only renders cards (`:13-35`) — no create/edit/delete/test/start/cancel
   anywhere; `createSyncConfig`/`startSync`/`cancelSync`/`testSyncConnection`/
   `listRemoteFiles` in `src/stores/app.ts:578-656` have **zero call sites**.
   Build: config modal (backend select, token input, provider-specific fields —
   repo/branch for GitHub+GitLab, folder for Drive, album for Photos, chat for Telegram,
   path for Local), test-connection button, save/edit/delete.
7. **OAuth connect buttons** for Google/GitHub/GitLab → AGENT-3's
   `GET /api/sync/oauth/{provider}/start` (opens the returned `authorizeUrl`), then
   re-fetch configs. Fall back to manual token paste when the route is absent
   (feature-detect on 404, never break the UI while agents land).
8. **Run/monitor/restore**: start sync → poll AGENT-2's job API
   (`POST /api/sync/start` → `202 {jobId}` → `GET /api/sync/jobs/{jobId}`; keep the old
   `get_sync_progress` path as fallback), live progress bar + per-file errors mapped from
   AGENT-1's error prefixes (`auth:`, `rate_limited:`, `unsupported:`, …), cancel button,
   remote-file browser with **download/restore** and remote delete (AGENT-2's
   `/api/sync/restore`, `/api/sync/remote`).
9. **Status truth**: `StatusBar.vue:62-71` always renders `SYNC:IDLE` (progress only
   written by actions nothing calls); `Sidebar.vue:148` advertises 4 providers (GitLab +
   Telegram missing); `GET /api/sync/status` (`crates/web/src/lib.rs:590`) is never
   called and returns a hardcoded payload. Wire real status end-to-end.

### P1 — frontend production hygiene

10. **Global error boundary**: `src/main.ts:7-9` mounts bare — add
    `app.config.errorHandler` + `unhandledrejection` → NotificationStack (store already
    routes errors via `lastError`, `app.ts:101`).
11. **Code splitting**: all 39 components are statically imported by `App.vue:14-24` →
    one monolithic chunk; `vite.config.ts` has no production `build` block at all (the
    `manualChunks` config lives only in the wasm config). Add `defineAsyncComponent` for
    heavy panels (MapView, FaceGrouping, CodeIntelligence, WebDashboard…) and a real
    production build config.
12. **Typed env**: `env.d.ts` declares no `ImportMetaEnv` — add `VITE_BASE`,
    `VITE_TRANSPORT`, etc. Remove `user-scalable=no, maximum-scale=1.0` from
    `index.html:6`; honor `prefers-reduced-motion` (MatrixRain, animations).
13. **Session UX**: logout button + expired-token handling (401 already dispatches
    `cybermanju:unauthorized` → login popup; verify it re-issues the request after
    login rather than dropping it).
14. **Share pages**: public share links return metadata only today — once AGENT-3 serves
    content at `GET /api/shared/{token}`, add the download/preview page and share-link
    management (create/copy/revoke) in the UI.
15. **ESLint + component tests**: request devDeps (AGENT-4 owns `devDependencies`;
    they are adding `vitest` — build on it) and add `src/**` tests for
    `useTauri` transport mapping and the sync store actions.

### P2 — docs & honesty (AUDIT F14; you are the only docs editor)

16. **README**: project tree still lists moved files (`src-tauri/src/search/tantivy_index.rs`,
    `crypto/pqc.rs`, `compression/triple.rs`, `faces/mod.rs`, `db/mod.rs`,
    `src-tauri/src/sync/{models,pipeline}.rs` — all deleted/moved to `crates/` in Phase 0);
    version claims; `yay -S cybermanju-drive` (AGENT-4 makes it publishable); WASM/Pages
    section must describe the transport you actually shipped.
17. **ARCHITECTURE**: §4 documents 11 tables but `crates/db` defines 17 (ask AGENT-2 for
    the final list); "curl subprocess" claim for sync backends (actual: `reqwest::blocking`);
    §10 endpoint list; "JWT-like UUID v4" token description (`:687`).
18. **AUDIT.md**: update the status log from agents' `AGENT-N.md` logs (Phase 1 "CI run ⬜"
    etc.), mark F-items closed with commits, keep the phase table current — *you* are the
    single writer of this file; never let others edit it.
19. **worklog.md**: claims "Zero mocks remaining" while F1–F6 exist — rewrite honestly.
20. Optional stretch: F8 (`src-tauri/src/tree_sitter/mod.rs` — README promises
    tree-sitter, code is regex heuristics) and i18n scaffolding (only via
    `package.json dependencies`, append-only).

## Contracts

**You consume (code against these; if a route is missing, feature-detect and degrade —
never block):**

| Source | Contract |
|---|---|
| AGENT-1 | error prefixes `auth:/rate_limited:/not_found:/unsupported:/too_large:/integrity:/network:` — map to user-facing toasts/retry affordances |
| AGENT-2 | `POST /api/sync/start → 202 {jobId}`, `GET /api/sync/jobs/{jobId}`, `POST /api/sync/restore`, `DELETE /api/sync/remote`, `GET /api/sync/usage/{configId}`, redacted `GET /api/sync/configs` (no `token` in responses) |
| AGENT-3 | `GET /api/sync/oauth/{provider}/start → {authorizeUrl, state}`, logout/revocation, `403` for role violations, share content at `GET /api/shared/{token}` |
| AGENT-4 | `vitest` harness + `npm run test`; CI jobs `wasm-build`/`deploy-pages` assert your build |

**You provide:** no backend surface. You do own `crates/drive-wasm`'s command table —
publish it in your Log so AGENT-4 can align the CI assertion.

**Sequencing:** items 6–9 can start immediately against the published contracts (all four
P0 backend contracts are specified in their briefs); land UI in two PRs — "wizard +
manual tokens" first, "OAuth + jobs + restore" after the producers merge.

## Verification

- `npx vue-tsc --noEmit` (if `node_modules` present) + CI `wasm-build` and
  `deploy-pages` green.
- Manual matrix in your Log: Pages build loads with seed data and works offline (wasm
  transport); desktop flow: create config → test → start → progress → cancel → restore;
  web (Docker) flow: login 401→popup→retry works.

## Definition of done

- [ ] F1 closed: Pages build is a real, offline-capable app with a stated transport.
- [ ] Sync panel is a full control surface; zero dead store actions remain
      (`createSyncConfig`/`startSync`/`cancelSync`/`testSyncConnection`/`listRemoteFiles`
      all reachable from the UI).
- [ ] Error boundary, code splitting, typed env, a11y fixes shipped.
- [ ] README/ARCHITECTURE/AUDIT/worklog reflect the code as of the final merge.
- [ ] Every `invoke('...')` call site still resolves to a REST route or a write-only
      command (re-run the Phase-1 mapping audit after your changes).

## Requests to other agents

_(append here; do not edit their files)_

## Log

- _(append dated entries: `YYYY-MM-DD — item N — commit — CI status`)_
- **2026-10-04 — drive-wasm build (WASM job) — commit: this push — CI: pending.**
  `ml-dsa` → `crypto-common 0.2` resolves getrandom **0.4**, which hard-errors on
  `wasm32-unknown-unknown` unless its `wasm_js` backend is selected, so the job died
  in `cargo check` before `wasm-pack` ever ran. Added `getrandom-wasm` (package
  `getrandom`, `0.4`, features `["wasm_js"]`) to `crates/drive-wasm/Cargo.toml`:
  cargo unifies *features* per resolved version, which turns `wasm_js` on for the
  instance `crypto-common` links without touching the existing `0.2`/`js` line.
  `Cargo.lock` gained the dep edge plus getrandom 0.4's `js-sys`/`wasm-bindgen`.
  No C/asm crates are in the wasm graph (the ML-KEM C build belongs to
  `cybermanju-crypto`, which `drive-wasm` does not depend on).
