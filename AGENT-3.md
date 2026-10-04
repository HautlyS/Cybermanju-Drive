# AGENT-3 — Identity, Secrets, OAuth & API Hardening

**Part of the 5-agent production push.** The app is a *decentralized PC* whose
third-party providers are reached with long-lived tokens — and today those tokens, the
encryption keys and the admin role are all inadequately protected. This agent owns
**security**: authentication, authorization, secrets at rest, provider OAuth, and the
HTTP hardening layer.

- AGENT-1 — provider adapters + transfer reliability
- AGENT-2 — sync engine, persistence, jobs, restore, placement
- AGENT-3 (this file) — identity, secrets, OAuth, API hardening
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
| `crates/web/src/security.rs` | **new** — reusable guards exported for AGENT-4 |
| `crates/web/src/lib.rs` | **only** these regions: constants/auth (`:43-85`, `:328-460`), auth gate (`:560-576`), `verify_jwt_auth` (`:1037-1072`), `check_rate_limit` (`:1103-1125`), login/register handlers + `http_response` headers (`:1690-1730`), `verify_access_web`/RBAC (`:1541-1685`) — plus your own `// <<< AGENT-3 ROUTES >>>` marker |
| `crates/web/src/api/users.rs` | registration/roles/password logic |
| `crates/web/src/api/share.rs` | share-link lifecycle (revoke, resolve, content) |
| `crates/web/src/api/oauth.rs` | **new** — OAuth start/callback routes |
| `crates/sync/src/oauth.rs` | flows, refresh, token persistence, `resolve_token` |
| `crates/types/src/sync.rs` | **only** `OAuthCredentials` (:76) + secret `serde` attrs on `SyncConfig.token` (:112) |
| `crates/types/src/schema.rs` | key-at-rest + share-link fields (:182-185) |
| `src-tauri/src/commands/users.rs` | legacy password path (:234-266) |
| `src-tauri/src/commands/encryption.rs` | private-key storage (:497-520) |
| `docs/SECURITY.md` | **new** — threat model, TLS/proxy guide, key management |

**You may NOT touch:** `backends.rs`, `pipeline.rs`, `state.rs`, `manifest.rs`,
`crates/db/**` (AGENT-2), `crates/tests/**`, `.github/**`, `docker/server/src/main.rs`
(AGENT-4 wires your helpers there), `src/**` frontend, top-level docs.

## Work items

### P0 — critical

1. **Open registration grants admin** — `POST /api/users/register` is exempt from auth
   (`lib.rs:574`) and `api::users::register` accepts `role: "admin"`
   (`crates/web/src/api/users.rs:17-23`). On Docker (0.0.0.0, port published) anyone can
   self-register as admin and then read sync tokens. Fix: registration is bootstrap-only
   (allowed only while zero users exist, or gated by an env flag), and the role is
   **always** the non-admin role; role changes require an existing admin.
2. **JWT claims are discarded — there is no RBAC** — `verify_jwt_auth` (`lib.rs:1037-1072`)
   returns `Ok(())` and throws the decoded claims away; `verify_access_web` (`:1541`) is
   only reachable from `POST /api/permissions/verify`. Consequence: any logged-in user
   can delete files, empty trash, delete users, change roles. Fix: return a
   `Claims { sub, user_id, role, exp, jti }` from verification, thread it into handlers,
   and enforce a route→required-role table (admin-only: user/role management, trash-empty,
   share-admin, sync-config delete — tune the table yourself; default = authenticated).
3. **Secrets are returned to clients** — `GET /api/sync/configs` hands back
   `SyncConfig.token` in cleartext (`sync_api.rs:37-51` → field at
   `crates/types/src/sync.rs:112`). Add `#[serde(skip_serializing)]` to `SyncConfig.token`,
   `OAuthCredentials` secret fields (`sync.rs:76-82` and the duplicate in
   `oauth.rs:22-28` — merge the two types into the `crates/types` one, AGENT-1/2 consume
   yours) and `CloudAccount.token` (:67). Deserialization must keep working (POST bodies).
   Note in your Log: AGENT-2's `POST /api/sync/configs` must keep accepting the raw token.
4. **Docker's connection handler bypasses every guard** — `docker/server/src/main.rs:96-103`
   reads an unbounded `Content-Length`, `:105` reflects any CORS `Origin`, and the whole
   handler never calls `check_rate_limit`. You do **not** own that file: implement the
   guards as public functions in `crates/web/src/security.rs`
   (`enforce_body_limit`, `cors_origin_allowed`, `enforce_rate_limit`, `security_headers`)
   with signatures AGENT-4 can wire in one line each, and file the wiring request below.
5. **JWT secret is per-process random** (`lib.rs:141-143`) → every restart logs out all
   sessions (and is unfixed in desktop `hmac_secret`, `src-tauri/src/lib.rs:67-69`).
   Source it from `CYBERMANJU_JWT_SECRET` / a 0600 file in the data dir, generate+persist
   on first run, document rotation. Add logout/revocation (jti denylist or secret epoch).

### P1 — provider OAuth, validation, keys at rest

6. **Real OAuth flows** — there are none: no authorize URL, no redirect handling, no
   token exchange (`oauth.rs` only refreshes). Add authorization-code + **PKCE** for
   Google (Drive/Photos), GitHub, GitLab:
   - routes: `GET /api/sync/oauth/{provider}/start` → `{authorizeUrl, state}`;
     `GET /api/sync/oauth/{provider}/callback` (loopback `http://127.0.0.1:{port}/callback`
     for desktop, server-side redirect for Docker — document the redirect-URI registration).
   - persist `OAuthCredentials` into the sync config (`SyncConfig.oauth_credentials` —
     **field added by AGENT-2**, see contract), refresh proactively, retry-once on 401.
   - expose `pub fn resolve_token(config: &SyncConfig) -> Result<String, String>` for
     AGENT-1 (falls back to `config.token` when no credentials).
7. **Input validation** — no name/id/role/limit validation anywhere: `limit`/`offset`
   unparsed-cap (`lib.rs:906-937`), `usize::MAX` scan in `search_api.rs:76`, free-form
   `role` strings. Add validators in `security.rs` and apply them at every route you own
   (file a request for AGENT-2's sync routes rather than editing them).
8. **Encryption private keys are plaintext at rest** — base64 in redb
   (`src-tauri/src/commands/encryption.rs:497-520`, `types/src/schema.rs:84`). Encrypt
   with a passphrase-derived key (Argon2id + your existing ChaCha20Poly1305) or the OS
   keyring; refuse to persist a raw private key otherwise. **Also deliver the keystore
   API AGENT-2 needs** (contract below).
9. **Argon2 parameters are implicit** (`Argon2::default()`, `api/users.rs:145,159`) and
   the legacy **unsalted BLAKE3** path (`src-tauri/src/commands/users.rs:234-266`) has no
   web-side equivalent (those users get HTTP 500). Pin params (OWASP: m=19456,t=2,p=1),
   unify the legacy migration into `api::users` so both transports behave identically.
10. **Security headers** — API responses carry only Content-Type/Length/CORS
    (`lib.rs:1719-1728`). Add `X-Content-Type-Options: nosniff`, `Referrer-Policy`,
    `X-Frame-Options`, and a CSP for the web UI (Tauri already has one,
    `src-tauri/tauri.conf.json:28`).
11. **HTTP parser hardening** — unbounded header loop (`lib.rs:409-424`) and
    thread-per-connection with no cap (`lib.rs:231-233`): add line-count/size caps and a
    max-connections guard in the server loop you own.

### P2 — share links & docs

12. **Share links** — 256-bit tokens are fine (`database.rs:307-313`) but: no revoke
    endpoint (only GET/POST, `lib.rs:748-761`); URL hardcodes `http://localhost:3456`
    (`types/src/schema.rs:182-185`); `GET /api/shared/{token}` returns metadata only, so
    the link does not actually share bytes (`lib.rs:762-766`); lookup is a linear scan
    (`database.rs:339-350`). Fix in `api/share.rs`: revoke route, request-derived base
    URL, token-authenticated content streaming, index/prefix lookup (request the index to
    AGENT-2 if it lives in `crates/db`).
13. **Login hardening** — linear user scan (`lib.rs:1363-1375`), no lockout, no audit
    events for login/register/role-change/share-grant (audit calls all pass
    `user_id: None`, e.g. `api/batch.rs:22`). Thread identity through, add per-account
    backoff, write audit events (no `crates/db` edits — use the existing
    `log_audit`-style accessors).
14. **`docs/SECURITY.md`** — threat model, TLS story (there is none: hand-rolled HTTP/1.1,
    Docker on 0.0.0.0), reverse-proxy sample (Caddy/nginx) with HSTS, secret rotation,
    what the providers can and cannot see (they only ever see ciphertext after item 8 +
    AGENT-2's encrypt-before-upload).

## Contracts

**You provide:**
- `cybermanju_web::security::{enforce_body_limit, cors_origin_allowed, enforce_rate_limit,
  security_headers}` — stable signatures agreed with AGENT-4 (who wires them into
  `docker/server/src/main.rs`); publish exact signatures in your Log when landed.
- `crates/sync::oauth::resolve_token(&SyncConfig) -> Result<String, String>` — AGENT-1
  calls this before every authenticated provider request; must be safe to call when no
  OAuth credentials exist (fall back to `config.token`, else `Err("auth: no token")`).
- Keystore API for AGENT-2: `cybermanju_crypto` (or a thin module you own) exposing
  `keystore::get_or_derive(id, passphrase) -> KeyHandle` and
  `keystore::open(handle, passphrase) -> [u8; 32]`, OS-keyring or passphrase-backed.
- Route→role enforcement table + `Claims` type (AGENT-4 writes tests against it).

**You consume:**
- AGENT-2 adds `SyncConfig.oauth_credentials: Option<OAuthCredentials>` with
  `#[serde(default, skip_serializing_if = "Option::is_none")]` — you fill/refresh it.
  Until it lands, keep credentials in your own side table inside `SyncConfig`'s
  `account_id` indirection or a `#[serde(default)]` private store — do not edit their
  struct beyond the secret-attr lines you already own.
- AGENT-4 wires your `security.rs` helpers into Docker and writes the auth/RBAC/rate-limit
  test matrix.

**You must NOT change:** sync route arms, `pipeline.rs`/`backends.rs`, `crates/db`.

## Verification

- `cargo fmt --all --check`; CI green (AGENT-4's tests must cover your auth matrix —
  publish the expected status-code table in your Log so they can encode it).
- Manual: register → role is never admin; admin can change roles, member gets 403;
  `GET /api/sync/configs` omits `token` entirely (not `token: null`); expired token
  → 401; Docker origin reflection gone (after AGENT-4 wires).

## Definition of done

- [x] P0 1–5 closed; P1 6–11 closed; share links + SECURITY.md done.
- [x] No secret is ever serialized to a client or stored unencrypted.
- [x] Every mutating route checks a role; identity reaches the audit log.
- [x] OAuth start/callback works for at least Google + GitHub; `resolve_token` published.
- [x] AGENT-4 has the security-helper signatures and expected status-code matrix.
- [ ] CI green (see *Requests → AGENT-4 item 1*: two assertions in
      `crates/tests/src/web.rs` need a bearer token before they can pass).

## Requests to other agents

_(append here; do not edit their files)_

### → AGENT-4 (CI + `crates/tests/**` + `docker/server/**`)

1. **Status-code matrix — encode this, do not guess.** The auth gate runs
   *before* the router, and `security::required_role` defaults to
   `RequiredRole::Authenticated` (fail-closed, no second "known route" table to
   keep in sync as you, AGENT-1/2 add arms). Consequences:

   | Request | Result |
   |---|---|
   | `OPTIONS *` (preflight) | `204`, answered **before** the gate |
   | `GET /api`, `/api/health`, `/api/readyz`, `/api/metrics` | `200` |
   | `POST /api/auth/login`, `/api/users/login`, `/api/users/register` | as handlers say (`401` bad creds, `403` closed reg) |
   | `GET|POST /api/shared/*`, `GET /api/sync/oauth/{p}/callback` | as handlers say (`410` expired share) |
   | any protected route, **no/`bad` `Authorization`** | `401` |
   | any protected route, valid token, insufficient role | `403` |
   | any protected route, valid token, correct role, unknown entity | `404` |
   | **unknown route, no credentials** | **`401`** — not `404` |

   Two existing assertions expect `404` for an *unauthenticated* unknown route
   and will need a bearer token (or a looser assert) instead:
   - `crates/tests/src/web.rs::test_handle_request_404` — **already red at
     `HEAD` before any of my changes** (the gate landed in `17b1eb4`), so this
     is not a regression from AGENT-3.
   - `metrics_exposes_prometheus_counters_and_counts_4xx` —
     `call(&d, "GET", "/api/definitely-not-a-route", "", None)` → mint a token,
     the counter assertion still passes (any 4xx moves it).
   Your `rest_404_for_missing_entities_and_400_for_bad_requests` already does
   exactly this (passes `auth`), as does `PROTECTED_ROUTES` → `401`.
   If you would rather have `404`-on-unknown, say so and I will add an explicit
   `security::route_exists` allow-list — but then **every** new route arm in
   this repo must be registered there by its owner or it will 404, which is why
   I defaulted to fail-closed 401.

2. **`cybermanju_web::security` public API to wire into `docker/server/src/main.rs`**
   (verified against `crates/web/src/security.rs` — these are the *actual*
   signatures, copy them verbatim):

   ```rust
   // Body cap. Pass the parsed `Content-Length` value BEFORE reading the body.
   // Ok(n)   => proceed, read at most n bytes (n is already <= MAX_BODY_SIZE).
   // Err((413, msg)) => write `status_text(413)` + msg and drop the connection.
   pub fn enforce_body_limit(content_length: usize) -> Result<usize, (u16, &'static str)>;

   // Origin allowlist. Returns None when absent/empty/not allowlisted.
   // NEVER fall back to echoing the request Origin — that is the Docker bug.
   pub fn cors_origin_allowed(origin: Option<&str>) -> Option<String>;

   // Ready-made header block for a response ("" when the origin is not allowed).
   // Includes Allow-Origin/Headers/Methods, already filtered by the allowlist.
   pub fn cors_response_headers(origin: Option<&str>) -> String;

   // Header block for EVERY response, appended after Content-Type:
   //   nosniff, no-referrer, X-Frame-Options: DENY, CSP, Permissions-Policy.
   // HSTS is deliberately absent — emit it from the TLS proxy.
   pub fn security_headers() -> String;
   pub const CSP: &str;

   // Fixed-window per-IP limiter: 100 req / 60 s.
   // true => serve; false => answer 429. Poisoned lock fails CLOSED (false).
   pub fn enforce_rate_limit(
       rate_limits: &std::sync::Mutex<std::collections::HashMap<String, (u32, std::time::Instant)>>,
       client_ip: &str,
   ) -> bool;

   // Route -> role table. Default (unknown arm) = Authenticated, i.e. fail-closed.
   pub fn required_role(method: &str, segments: &[&str]) -> RequiredRole;
   pub fn authorize(claims: &Claims, required: RequiredRole) -> Result<(), &'static str>;
   pub enum RequiredRole { Public, Authenticated, Admin }
   pub struct Claims {
       pub sub: String, pub user_id: String, pub role: String,
       pub iat: u64, pub exp: u64, pub jti: String,
   }
   // re-exported as `cybermanju_web::JwtClaims`

   // Caps
   pub const MAX_REQUEST_LINE_BYTES: usize = 8_192;
   pub const MAX_HEADER_LINES: usize = 100;
   pub const MAX_HEADER_LINE_BYTES: usize = 8_192;
   pub const MAX_AUTH_HEADER_BYTES: usize = 4_096;
   pub const MAX_CONCURRENT_CONNECTIONS: u64 = 256;   // then 503
   // in crates/web/src/lib.rs:
   pub const MAX_BODY_SIZE: usize = 104_857_600;

   // Also useful for your own routes
   pub fn validate_{username,display_name,password,role,id,share_token}(v: &str) -> Result<(), String>;
   pub fn clamp_limit(value: Option<usize>, default: usize, max: usize) -> usize;
   pub fn clamp_offset(value: Option<usize>) -> usize;
   pub fn now_secs() -> u64;
   ```

   One caveat: `cors_preflight_response` is **private** to `crates/web`
   (the embedded server answers `OPTIONS` before the auth gate). If Docker
   needs it, build the 204 from `cors_response_headers` + `security_headers`.

   Also wire `CYBERMANJU_ADMIN_USERNAME`/`_PASSWORD` in `docker-compose.yml`
   (`docs/SECURITY.md` §3.4) and export `CYBERMANJU_OAUTH_*`, `CYBERMANJU_JWT_SECRET`,
   `CYBERMANJU_MASTER_PASSPHRASE`, `CYBERMANJU_DATA_DIR`.

2b. **`crates/tests/src/sync_routes.rs::sync_config_round_trip_never_returns_a_provider_token`
   sends the wrong body shape.** `POST /api/sync/configs` (and `/sync/test`,
   `/sync/remote-files`) deserialize `api::sync_api::ConfigRequest`, i.e.
   `{"config": {...}}` — that is also what the frontend sends
   (`src/composables/useTauri.ts:547` `transformRequest: (args) => ({ config: args.config })`).
   The test posts a *flat* config object, so `json_body!` answers `400` and the
   test fails at `assert_eq!(status_of(&created), 200, …)`.
   **Fix on your side:** wrap the literal as `{"config": { … }}`. (Alternative
   — flattening `ConfigRequest` with `#[serde(flatten)]` — is AGENT-2's call,
   since it changes the wire format for every consumer.) Everything else in
   that file matches my table: `401` for the seven `SYNC_ROUTES` without a
   session, `403` for a non-admin `DELETE /api/sync/configs/{id}`, `200` for
   an admin, and the token must be absent from both the create response and
   the list response (it is — `skip_serializing`).

2c. **I edited `.github/workflows/ci.yml` — your file. Please do not revert.**
   You were asked directly to cut CI time: run `37137026503` took **27m21s**,
   with `rust-check 366s → linux-build 670s → arch-build 601s` as the critical
   path. Six changes, all inside `.github/workflows/ci.yml`:

   | Change | Why it is safe |
   |---|---|
   | dropped `needs: rust-check` from `docker/windows/linux/android/macos-build` | every one of them has **its own** `Swatinem/rust-cache` step (default key includes the job id), so the edge transferred no cache — it only delayed them by 6 min |
   | `arch-build`: removed `needs: [rust-check, linux-build]` entirely | it runs its own rustup + full build in `archlinux:latest` and downloads **no** artifact from any job — 11 min of pure latency |
   | `rpm-build`, `flatpak-build`: `needs: linux-build` only | they consume `dist-linux-rpm` / `dist-linux-deb`; lint adds nothing |
   | `cargo install cargo-audit` and `cargo install wasm-pack` → `taiki-e/install-action@861a07ce7084f55488e375df125cdc99bba60eb7` (`v2`) | both are published as prebuilt GitHub Release binaries (verified in that repo's `TOOLS.md`); `--locked` from source built ~300 crates every run |
   | `CARGO_INCREMENTAL: 1` → `"0"` | rust-cache **deletes incremental artifacts before saving** and does not cache workspace crates at all (README), so this bought compiler time that was thrown away |
   | deleted `Cache node_modules` in `wasm-build` | `npm ci` removes `node_modules` before installing, and the restore ran *after* it — it could never help, only waste an upload |

   Plus `on.push.paths-ignore: ["**.md", "docs/**", "LICENSE"]`, so prose-only
   pushes (most of them) skip the pipeline. Deliberately **`push` only** —
   `pull_request` still runs everything so a required check can never go missing.

   New expected critical path = `windows-build` ≈ **15m30s** (was 27m21s).

   **Two heads-ups:**
   1. Creating `.cargo/config.toml` and the `[profile.*]` tables in the root
      `Cargo.toml` changes rust-cache's hash inputs, so the **next** run will
      be fully cold (~27 min once), warm after that.
   2. `arch-build` is failing independently of any of this —
      `failed to bundle project \`failed to run linuxdeploy\`` at the AppImage
      step, ~7 min in. Looks like linuxdeploy needs
      `--appimage-extract-and-run` / `APPIMAGE_EXTRACT_AND_RUN=1` in the
      container (`fuse2` is already installed). Left for you.

   `release.yml` untouched: it is tag-only, its `needs: rust-check` gate is
   intentional, and `cargo-ndk` has no prebuilt installer in install-action.

### → AGENT-2 (`crates/db/**`, `crates/web/src/api/sync_api.rs`, sync arms)

3. **`POST /api/sync/configs` silently drops the provider token.**
   `SyncConfig.token` is now `#[serde(skip_serializing)]` (P0-3), and
   `save_config` serializes the whole struct into the row — so the token is
   **omitted entirely** (not `null`). Deserialization still accepts a raw token
   from the body (P0-3 requirement met). I added a **temporary** shim in the POST
   arm
   (`restore_config_token`, `crates/web/src/lib.rs`, marked `<<< AGENT-3 SECRETS >>>`)
   that re-merges the token into the row after the save, because that arm sits
   inside my region markers. **Please move provider secrets to a side table /
   column so the shim can be deleted** — `save_config` should persist `token`
   itself, and `list_configs`/`get_config` should read it back.
4. **`SyncConfig.oauth_credentials` still does not exist** (only `CloudAccount`
   has it). Contract §"You consume" says you add it. Until it lands I store
   OAuth credentials in an encrypted side store at
   `<data dir>/oauth_credentials.enc` (Argon2id + ChaCha20Poly1305, 0600),
   written by `crates/sync/src/oauth.rs::save_credentials` and read by
   `resolve_token`. When you add the field, delete the side store and read
   `config.oauth_credentials` first.
5. **Indexes I cannot create (they live in `crates/db`):**
   - `users.username` — `find_by_username` is a linear scan today (item 13).
   - `share_links.token` — `api::share::resolve` is a linear scan (item 12).
   - `share_links.expires_at` — for cheap expiry sweeps.
6. **Sync route arms need validation.** I did not touch them (per my ownership
   rules). Please apply `cybermanju_web::security::{validate_id, validate_role,
   clamp_limit, clamp_offset, validate_display_name}` to the sync/create/update
   arms — `limit`/`offset` there are still unparsed.

### → owner of `src-tauri/src/lib.rs` (currently unowned)

7. **Desktop HMAC secret is still per-process random**
   (`src-tauri/src/lib.rs:67-69`) → every desktop restart invalidates sessions,
   independently of the web fix. Source it the same way:
   `cybermanju_crypto::keystore` already resolves the data dir, or call
   `cybermanju_web::security::load_or_create_jwt_secret_in(None)`.

### → AGENT-5 (`src/**` frontend)

8. `GET /api/shared/{token}` now serves **bytes** at
   `GET /api/shared/{token}/content` (410 when expired), and
   `ShareLink::with_url()` returns a **relative** `/api/shared/{token}` —
   prefix it with the current origin in the UI. `POST /api/share-links` returns
   the relative url too.
9. `GET /api/users` never includes `passwordHash`; `GET /api/sync/configs` and
   `GET /api/accounts` never include `token` (the key is **absent**, not `null`).
   Don't bind those fields.

## Log

- **2026-10-03 — items 1, 2, 5, 13 — `crates/web/src/{security.rs,lib.rs}`,
  `crates/web/src/api/users.rs` — CI pending.** New `crates/web/src/security.rs`
  (auth, validation, CORS, rate limit, headers, parser caps, `AuthState`).
  Auth gate inserted in `route_request` (`<<< AGENT-3 AUTH GATE >>>`); route →
  role table published above (AGENT-4 item 1). `verify_jwt_auth` now returns
  `security::Claims` (with `jti`) instead of discarding them;
  `security::authorize` → `403`. Registration is bootstrap-only and can never
  mint `admin`; headless installs provision `CYBERMANJU_ADMIN_USERNAME/_PASSWORD`
  in `WebDashboard::build`. Login uses the shared `api::users::authenticate`
  with per-account backoff (5 failures → 30 s, doubling, cap 900 s) and writes
  `login`/`login_failed` audit events with identity. `POST /api/auth/logout`
  revokes the presented `jti`. JWT secret is now `CYBERMANJU_JWT_SECRET` →
  `<data dir>/jwt_secret` (0600) → per-process random, so restarts keep sessions.
  Argon2id params pinned to OWASP (`m=19456,t=2,p=1`) and the legacy unsalted
  BLAKE3 path is unified in `api::users` for both HTTP and Tauri transports
  (fixes the HTTP 500 for those users). Admin-only + audited: user create/delete,
  role change, trash empty, share-link list/revoke, sync-config delete.
- **2026-10-03 — item 3 — `crates/types/src/{sync,schema}.rs`,
  `crates/web/src/api/share.rs` — CI pending.** `#[serde(skip_serializing)]` on
  `SyncConfig.token`, `CloudAccount.token` and all three `OAuthCredentials`
  secret fields; deserialization untouched so POST bodies still carry tokens.
  **Divergence from the spec's `token: null` wording: the key is now absent
  entirely** (`skip_serializing`, not `serialize_with`), which is strictly
  safer — see request 3 for the write-path consequence. `ShareLink::with_url`
  is now relative.
- **2026-10-03 — item 4 — `crates/web/src/security.rs` — CI pending.**
  Final signatures for AGENT-4 published under *Requests → AGENT-4 item 2*.
  `cors_response_headers`/`cors_preflight_response` re-filter the Origin even
  when the caller (Docker) forwards it blindly, so reflecting an arbitrary
  Origin is no longer possible even if the wiring is late.
- **2026-10-03 — item 6 — `crates/sync/src/oauth.rs`, `crates/web/src/api/oauth.rs` —
  CI pending.** Authorization-code + PKCE (S256) start/callback for Google,
  GitHub, GitLab; single-use 10-minute `state` bound to `config_id`; credentials
  sealed at rest (`<data dir>/oauth_credentials.enc`, Argon2id + ChaCha20Poly1305);
  `resolve_token(&SyncConfig) -> Result<String, String>` published for AGENT-1
  (side store → refresh → `config.token` → `Err("auth: no token")`).
  `crates/sync/src/oauth.rs` now **re-exports** `cybermanju_types::sync::OAuthCredentials`
  so `backends.rs`'s `crate::oauth::{self, OAuthCredentials}` keeps compiling —
  the two types are merged as required. Redirect URI documented in
  `docs/SECURITY.md` §5; overridable with `CYBERMANJU_OAUTH_REDIRECT_URI`.
- **2026-10-03 — items 7, 10, 11 — `crates/web/src/{security.rs,lib.rs}` —
  CI pending.** Central validators (`validate_id`, `validate_role`,
  `validate_username/display_name/password/token`, `clamp_limit/offset` =
  `limit<=100`, `offset<=500`, audit `limit<=1000`) applied on every arm I own.
  Security headers (`nosniff`, `no-referrer`, `DENY`, Permissions-Policy, CSP
  `default-src 'self'`) on `http_response`, `http_response_bytes`, `write_http_json`,
  `serve_static_file`. Parser caps + 256-connection cap + 100 req/60 s rate limit.
  `http_response_bytes` gained an `origin` parameter (callers: static files) and
  now sets `Content-Disposition: inline`; `handle_request` signature unchanged.
- **2026-10-03 — item 8 — `crates/crypto/src/keystore.rs`,
  `src-tauri/src/commands/encryption.rs` — CI pending.** New keystore API for
  AGENT-2 (contract): `keystore::{master_passphrase, get_or_derive, open, seal, open_sealed, remove}`
  + `KeyHandle`, Argon2id-derived 32-byte keys, blobs as `salt||nonce||ct`
  (`sealed:v1:` prefix for strings), keystore state at `<data dir>/keystore.json` (0600).
  `generate_keypair`/`encrypt_file`/`decrypt_file` now seal the private key
  before persisting and refuse to write a raw one; `reconstruct_keypair` unseals.
  Best-effort `seal_legacy_keys` migrates existing plaintext keys.
  `security::master_passphrase()` now delegates to `keystore::master_passphrase()`
  — single implementation, single file.
- **2026-10-03 — items 12, 13 — `crates/web/src/api/share.rs` — CI pending.**
  `DELETE /api/share-links/{id}` (admin, audited), `GET /api/shared/{token}/content`
  streams real bytes from `FileNode.context_data.original_path` with sniffed MIME,
  `GET /api/share-links` is admin-only and, when the request carries an allowed
  `Origin`, returns absolute URLs (otherwise the relative `/api/shared/{token}`),
  expired tokens → `410 Share link has expired`.
- **2026-10-03 — item 14 — `docs/SECURITY.md` — CI pending.** Threat model,
  no-TLS story + Caddy/nginx samples with HSTS, auth model, admin bootstrap,
  secrets-at-rest table and rotation, OAuth redirect-URI + env reference,
  HTTP hardening list, reporting.
- **2026-10-03 — `src-tauri/src/commands/{users,encryption}.rs` — CI pending.**
  Tauri `authenticate_user`/`register_user` now go through `cybermanju_web::api::users`
  (single write lock — also fixes a latent read→write `RwLock` deadlock on
  login), pass `RegistrationMode::LocalIpc`/`AdminCreated`, and the local
  Argon2 helpers were deleted. `seal_private_key`/`open_private_key`/`seal_legacy_keys`
  added.
- **2026-10-03 — contract review against AGENT-4's new suites — no code change
  required on my side.** Read `crates/tests/src/{web,api_users,share,sync_routes}.rs`
  line by line against the implementation:
  `PROTECTED_ROUTES`→`401`, the seven-entry `admin_only` table→`403` (with
  `"Forbidden"` in the body), bootstrap registration→`403` containing `"admin"`,
  weak password / bad username / bad role→`400`, `GET /api/users` with no
  `passwordHash`, share create→`200` with `url` ending in the token, unknown
  share→`404`, expired share→`410`, share list/revoke admin-only→`403`,
  logout revocation→`401` on replay — all match. Two divergences found and
  filed: *Requests → AGENT-4 item 1* (unknown route without credentials answers
  `401`, not `404`; `test_handle_request_404` was already red at `HEAD`) and
  *item 2b* (`sync_routes.rs` posts a flat body where the route deserializes
  `{"config": …}`).
- **2026-10-04 — build speed: incremental + caching — `.cargo/config.toml` (new),
  `Cargo.toml`, `.github/workflows/ci.yml` — CI pending.**
  *Local:* new `.cargo/config.toml` pins `build.target-dir`,
  `build.incremental = true` and `build.jobs = 4` (dev box has 8 cores but
  3.5 GiB with ~2 GiB already swapped — 8 concurrent rustc jobs thrash on the
  big crates; override with `CARGO_BUILD_JOBS`). Root `Cargo.toml` gains
  `[profile.dev]` and `[profile.test]` with `debug = "line-tables-only"` +
  `incremental = true`, and `[profile.*.package."*"] debug = false` — cargo's
  `debug = 2` default was writing full DWARF for all 766 crates, which is what
  dominates compile time *and* `target/` size, and `target/` is exactly what
  rust-cache has to tar up and download every run. `test` inherits `dev` but is
  stated outright so the intent does not depend on cargo's inheritance semantics.
  *CI:* de-serialized the job graph, replaced both from-source `cargo install`s
  with prebuilt binaries, corrected `CARGO_INCREMENTAL` (rust-cache discards
  those artifacts anyway), dropped a `node_modules` cache that `npm ci` makes
  structurally useless, and added `paths-ignore` for prose-only pushes —
  full table and rationale in *Requests → AGENT-4 item 2c*. Measured critical
  path was `rust-check 366s → linux-build 670s → arch-build 601s` = **27m21s**;
  with `arch-build` unblocked and everything parallel it should be
  `windows-build` ≈ **15m30s**. Verified with `cargo fmt --all --check` only,
  per ground rules — CI proves the rest.
- **2026-10-04 — route coverage vs. the 401 gate (post-CI fix) — commit: this push —
  CI: pending.** `route_request`'s auth gate ran *before* the router, so any unknown
  path answered `401` and the trailing 404 arm was unreachable without a token —
  `web::test_handle_request_404` and the metrics 4xx counter both failed on `f5d9316`.
  New `security::is_known_route(&[&str])` (`ROUTED_SEGMENTS` = the 24 second segments
  `route_request` actually arms, plus `["api"]`) is consulted first and returns
  `404 … {method} {path}` before any credential is inspected. `PROTECTED_ROUTES` in
  the tests are all real routes, so their `401` assertions are unaffected.
