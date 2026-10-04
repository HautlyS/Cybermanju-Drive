# AGENT-4 — Platform: CI, Tests, Release & Docker/Ops

**Part of the 5-agent production push.** Nothing is true until CI proves it: today
**0 of 148 tests actually run**, the Docker image ships without the features it
advertises, and no release artifact is ever published to a registry. This agent owns the
**verification and delivery machinery** plus the server/ops surface.

- AGENT-1 — provider adapters + transfer reliability
- AGENT-2 — sync engine, persistence, jobs, restore, placement
- AGENT-3 — identity, secrets, OAuth, API hardening
- AGENT-4 (this file) — CI, tests, release, Docker/ops
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
| `.github/workflows/ci.yml`, `release.yml` | full ownership |
| `.github/dependabot.yml` | **new** |
| `docker/server/**` (`main.rs`, `Cargo.toml`) | wire search/crypto/security helpers, shutdown |
| `Dockerfile`, `docker-compose.yml` | image + CasaOS metadata |
| `crates/tests/**` | the workspace test suite |
| `package.json` | **scripts + devDependencies only** (AGENT-5 owns `dependencies`) |
| `src-tauri/Cargo.lock` | delete (stale duplicate) |
| `aur/**`, `LICENSE` (new) | packaging metadata |
| `scripts/backup.sh` (new), `docs/OPERATIONS.md` (new) | ops runbook |
| `crates/web/src/lib.rs` | **only** the health/dashboard/encryption-status arms (`:884-896`, `:995-1030`) and your `// <<< AGENT-4 OPS >>>` marker for the access-log call |

**You may NOT touch:** `crates/sync/**` logic, `crates/web/src/api/**` (except adding
tests via `crates/tests`), `src/**` frontend, `README/ARCHITECTURE/AUDIT/worklog`
(AGENT-5), the auth regions of `lib.rs` (AGENT-3), the sync arms of `lib.rs` (AGENT-2).

## Work items

### P0 — F10: CI does not run the test suite (worse than AUDIT states)

1. `ci.yml:49,53,57` and `release.yml:50,54,58` run `cargo fmt/clippy/test` with
   `working-directory: src-tauri` → current-package-only. `src-tauri` has **0 tests**, so
   the "Run tests" step executes nothing; all **121** `crates/tests` tests + 26 faces + 1
   oauth test never run. Replace with workspace-wide:
   ```yaml
   - run: cargo fmt --all -- --check
   - run: cargo clippy --workspace --all-targets -- -D warnings
   - run: cargo test --workspace --no-fail-fast
   ```
   (run from repo root; keep the apt/webkit deps and `Swatinem/rust-cache` with
   `workspaces: "."`).
2. **Delete stale `src-tauri/Cargo.lock`** (187 KB duplicate) and rekey every cache that
   points at it (`ci.yml:45,135,185,391`, `release.yml:45,166,350` → root `Cargo.lock`).
3. Expect fallout: workspace clippy will surface warnings in `crates/**` and
   `docker/server` that were never linted. Fix them **in your own files only**; for
   findings in AGENT-1/2/3 files, file a request (they may already be fixing them —
   check the branch first).

### P0 — F11: Docker advertises features it does not ship

4. `docker/server/Cargo.toml` depends only on `cybermanju-db` + `cybermanju-web` → no
   search index, no crypto (F11). Append `cybermanju-search` (+ `cybermanju-crypto` if
   AGENT-3's status endpoint needs it), then in `docker/server/src/main.rs` construct a
   `SearchIndex` and call `set_search_index` (`crates/web/src/lib.rs:173`) — currently
   never called, so `/api/search` silently degrades to substring scan
   (`api/search_api.rs:42-54`).
5. `GET /api/encryption/status` returns a hardcoded Kyber/ML-DSA payload
   (`crates/web/src/lib.rs:884-896` — your region): report the *build*'s real
   capabilities instead.
6. Rewrite `docker-compose.yml:34-45` `x-casaos` description to match what the image
   actually does after 4–5 (faces stay desktop-only — that part of the wording is
   already a decision in `AUDIT.md` D1).

### P0 — Docker security wiring (depends on AGENT-3's helpers)

7. `docker/server/src/main.rs` bypasses rate limiting, body-size cap and CORS allowlist
   (`:96-103`, `:105` vs the desktop guards at `crates/web/src/lib.rs:328,448-451`).
   Wire AGENT-3's `cybermanju_web::security::{enforce_body_limit, cors_origin_allowed,
  enforce_rate_limit, security_headers}` — one call each, exact signatures will be in
   AGENT-3's Log. Do not design your own guards.
8. Compose hardening: `read_only`, `cap_drop: [ALL]`, `security_opt: [no-new-privileges]`,
   `mem_limit`, publish `3456` on LAN interfaces only, keep the healthcheck but point it
   at the real readiness endpoint (item 10).

### P1 — tests (the suite is yours)

9. **REST/auth matrix** in `crates/tests/src/web.rs` (14 shallow tests today): 401 on
   every protected route; 200 on health/login/register/shared; 400 malformed JSON; 404 vs
   400; 413 over `MAX_BODY_SIZE`; rate-limit 429; CORS allow/deny; static path-traversal
   403; argon2 verify; JWT expiry; RBAC 403 matrix — encode the status-code table AGENT-3
   publishes. Register new test modules in `crates/tests/src/lib.rs` (you own it):
   `api_users`, `share`, `sync_routes` (AGENT-2's job API), `backends_contract`.
10. **Backend contract tests** for AGENT-1: a local mock HTTP server (add a dev-dep to
    `crates/tests/Cargo.toml` — append-only) asserting: pagination loops, size-limit
    preflight, error-prefix classification, no `Ok` on 404/401, retry/backoff on 429.
11. **Ops endpoints**: replace the static `/api/health` JSON (`lib.rs:1011-1026`) with a
    real readiness check (redb open + index open + disk writable) and add `/readyz`;
    keep `/api/health` shape backward-compatible for the container healthcheck.
12. **Access logging**: one middleware-style call in `handle_request` inside your
    `// <<< AGENT-4 OPS >>>` marker → method, path, status, latency, request id, auth
    outcome, via `log` (web crate already depends on it). Ship `scripts/backup.sh`
    (redb + tantivy index snapshot, safe while running) and `docs/OPERATIONS.md`.
13. **Graceful shutdown** for `docker/server` (bare accept loop, `:200-212`): SIGTERM →
    stop accepting, drain in-flight, flush DB. Desktop already does this
    (`crates/web/src/lib.rs:262-301`).

### P1 — release & packaging

14. **Docker publish**: release notes say `docker pull hautlythird211/cybermanju-drive`
    (`release.yml:514,519`) but **no job ever pushes** (no login, `push: false` at
    `:448`). Add `docker/login-action` + `push: true` (GHCR and/or Docker Hub), or delete
    the claims.
15. **Version single-source**: 8 conflicting values (`package.json`/`tauri.conf.json`/
    `src-tauri/Cargo.toml` = 0.0.1; `README`/`docker-compose`/`PKGBUILD`/crates = 0.1.0;
    `lib.rs:1000` = 1.0.0). Pick one source (`package.json`), add a CI check that tags,
    `tauri.conf.json` and the status endpoint agree; add the missing **`LICENSE`** file
    (README:7 and `PKGBUILD:62` already reference it).
16. **AUR**: `aur/PKGBUILD` has no `.SRCINFO` (unpublishable), `sha256sums=('SKIP')`,
    placeholder maintainer, `pkgver` mismatch. Generate `.SRCINFO`, fix fields.
17. **WASM job honesty**: `wasm-build` (`ci.yml:86-121`) compiles no Rust. Add
    `cargo check --target wasm32-unknown-unknown -p cybermanju-drive-wasm` + a
    `wasm-pack build crates/drive-wasm` assertion (AGENT-5 implements the crate; your job
    makes CI fail if it stops building). Rename the job to match reality in the meantime.
18. **Hardening**: pin Actions by commit SHA; add `dependabot.yml` (cargo + npm + actions);
    `cargo audit`/`npm audit` (allow-fail initially, tighten later); remove `|| true`
    artifact handoffs in `release.yml` (`:533-594`) and set `if-no-files-found: error`;
    add a macOS job to `ci.yml` (release has one, CI does not).

### P2

19. Frontend test harness: add `vitest` (+ config) and `"test"` script to `package.json`
    (scripts/devDeps are yours) with one smoke test; AGENT-5 adds real tests on top.
20. Metrics counters (requests, 4xx/5xx, active connections — `lib.rs:108` already has
    the gauge) exposed at `/api/metrics` in text format; no new heavy deps without a
    request line here first.

## Contracts

**You provide:** green workspace CI (everyone's gate); the test suite; Docker image with
real search/crypto; ops runbook. Publish in your Log: how each agent should interpret a
red `rust-check` job (which job = whose area).

**You consume:** AGENT-3's `security.rs` signatures (item 7); AGENT-1's error prefixes
(contract tests, item 10); AGENT-2's REST contract (route tests, item 9); AGENT-5's
`crates/drive-wasm` build (item 17).

**Sequencing:** land item 1–2 **first** (everyone benefits immediately), then 3→8 in any
order; items 7, 9 (RBAC part), 17 wait for the respective producers' merges.

## Verification

- Your own changes *are* the verification: a PR that makes `cargo test --workspace` run
  and pass is the deliverable. No local builds — push and read the Actions log.
- `cargo fmt --all --check` locally.

## Definition of done

- [ ] CI runs workspace fmt/clippy/test; stale lock deleted; caches rekeyed.
- [ ] All pre-existing 148 tests green in CI, plus your new REST/auth/backend-contract
      suites.
- [ ] Docker image contains search+crypto and its metadata is honest; guards wired;
      real readiness endpoint; graceful shutdown.
- [ ] Releases actually publish a container; versions agree everywhere; LICENSE and
      `.SRCINFO` exist.
- [ ] Ops runbook + backup script shipped.

## Requests to other agents

_(append here; do not edit their files)_

## Log

- _(append dated entries: `YYYY-MM-DD — item N — commit — CI status`)_
- **2026-10-04 — run 37212587257 (`f5d9316`) — 22 test failures + 4 red jobs — CI: pending.**
  *Red jobs:* Rust Lint & Test (22 failures), Build WASM, Docker Build, Arch/CachyOS.
  *`backends_contract` (6):* the `starts_with("GitLab …")` asserts contradicted the
  AGENT-1 prefix contract — rewritten to `classify_error(&err)`/`ErrorClass` (NotFound
  + `delete failed`, Network for 500/upload, RateLimited for 429), which is what the
  suite is actually pinning. *`sync_routes`:* POST body wrapped in the `ConfigRequest
  { config }` envelope the route deserializes. *`compression` (4):* camelCase serde,
  empty-input `decompress_triple`, and incompressible test data — AGENT-2/5 files,
  details in their Logs. *`crypto` (4):* AES-256 encrypt hit `unreachable!` and
  `created_at` lacked `Z` — AGENT-3/5 files. *`search` (4):* stale Tantivy reader
  (`ReloadPolicy::OnCommitWithDelay`) — `reader.reload()` now runs after every
  commit (`crates/search/src/lib.rs::refresh`). *`web` (2):* unknown paths answered
  `401` before the router, so the 404 arm was unreachable — fixed by AGENT-3's
  `is_known_route` gate. *Jobs:* Docker died in PQClean's `compat.h` —
  `#if !__GNUC_PREREQ(7,1)` is glibc-only and musl's `<features.h>` defines nothing,
  so `CFLAGS=-D__GNUC_PREREQ(major,minor)=1` is set in the `rust:alpine` stage
  (reachable only since AGENT-3 wired `cybermanju-crypto` into the server graph);
  Arch died in `linuxdeploy` because the container has no `/dev/fuse` —
  `APPIMAGE_EXTRACT_AND_RUN=1` makes the AppImage tools unpack instead of mount.
- **2026-10-04 — run 37220719686 (`7f1a530`) green except Arch → Arch `NO_STRIP`
  fix — CI: pending.**
  Eleven of twelve jobs passed: Rust Lint & Test (clippy `-D warnings`, fmt, full
  workspace suite), Build WASM, Docker Build, Deploy to Pages, both audits, Android,
  Flatpak, RPM, deb+AppImage+rpm, Windows, macOS and Version & Metadata. Only
  `Build Linux (Arch/CachyOS)` stayed red with the opaque
  `failed to run linuxdeploy`. The previous entry's FUSE diagnosis was wrong —
  `APPIMAGE_EXTRACT_AND_RUN=1` was already exported on that run (it never got past
  download). The real cause matches tauri#8929/#13113 exactly, including the same
  one-line message and reports from Garuda: linuxdeploy's *bundled* `strip` cannot
  read the `.relr.dyn` sections Arch-built system libraries carry
  (`unknown type [0x13] section .relr.dyn`), and the Tauri bundler throws that
  stderr away at the default log level. Fix in this entry's commit: `NO_STRIP=true`
  in the `Build Arch package` step, `patchelf` + `desktop-file-utils` added to the
  pacman list (neither is in `base-devel`), and
  `npm run tauri build || npm run tauri -- build --verbose` so a residual failure
  prints linuxdeploy's actual output instead of the one-liner while a green run
  stays quiet.
