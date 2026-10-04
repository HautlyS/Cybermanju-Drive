# AGENT-1 — Provider Adapters & Reliable Transfer

**Part of the 5-agent production push.** The product vision: a *decentralized PC* whose
storage lives on third-party providers (Local, GitHub, GitLab, Google Drive, Google
Photos, Telegram). This agent owns the **transport layer**: making every provider
adapter correct, honest, resumable, rate-limit-aware and integrity-checked.

- AGENT-1 (this file) — provider adapters + transfer reliability
- AGENT-2 — sync engine, persistence, jobs, restore, chunk manifest
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
| `crates/sync/src/backends.rs` | all 6 backends (1731 lines) |
| `crates/sync/src/retry.rs` | **new** — backoff/retry policy |
| `crates/sync/src/rate_limit.rs` | **new** — per-provider limiter + concurrency caps |
| `crates/sync/src/transfer.rs` | **new** — BLAKE3 integrity verify, size preflight |
| `crates/sync/src/quota.rs` | **new** — quota/usage probing |
| `crates/sync/src/lib.rs` | add `mod` declarations only |
| `crates/sync/Cargo.toml` | append-only (`blake3`, `cybermanju-crypto` if needed) |
| `crates/types/src/sync.rs` | **only** `SyncBackendType` (:5), `RemoteFile` (:150), `StorageBackend` (:158) + new `Capabilities` type |

**You may NOT touch:** `pipeline.rs`, `state.rs`, `oauth.rs`, `crates/db`, `crates/web`,
`src-tauri`, `docker/`, `src/`, `.github/`, `README/ARCHITECTURE/AUDIT`.

## Work items

### P0 — silent data loss / lies (fix first)

1. **GitHub ≥1 MB downloads write 0-byte files and return `Ok`** — `backends.rs:409-429`
   decodes an empty `content` field. Use `Accept: application/vnd.github.raw` (media API)
   and **fail** if the decoded length ≠ expected size.
2. **GitHub re-upload to the same path 422s** — `backends.rs:349-353` omits `sha` on update.
   GET sha first (you already do this in delete, `:434-493`).
3. **Telegram `delete_file`/`list_files` are no-ops returning success** —
   `backends.rs:1546-1561` (AUDIT F9). Return `Err("unsupported: ...")`. Never fake `Ok`.
4. **Telegram round-trip is impossible** — upload stores a `t.me/c/...` URL (`:1473-1477`)
   but download needs a `file_id` (`:1480`). Persist the `file_id` (return it as the
   locator; AGENT-2 stores it).
5. **Telegram `get_file_url` leaks the bot token** — `backends.rs:1597-1600`. Return a
   token-free URL or `Err("unsupported: ...")`.
6. **Google Photos delete always fails / URLs expire** — `:1258-1281` (Library API has no
   delete) → explicit `unsupported:` error; `baseUrl` expires in ~60 min (`:1155-1222`) →
   return the media item id as locator, resolve a fresh URL in `get_file_url`.
7. **Google Drive/Photos/Telegram ignore `remote_path`** — `:909`, `:1155`, `:1418`. Honor
   it (Drive: resolve/create folder chain; Photos/Telegram: encode the path into the
   caption/filename + document it as best-effort).

### P1 — correctness & scale

8. **Pagination everywhere** — GitLab `:770-824` (no `page` loop, `size_bytes: 0`/`modified_at: ""`
   hardcoded at `:817-818`), Drive `nextPageToken` requested but unread `:1035`, Photos
   `pageToken` unread `:1284`, GitHub 1000-entry cap `:495-549`. Add recursion flag for
   directory walks. Fill real size/modified where the API provides them.
9. **Size limits preflight** (`transfer.rs`): Telegram Bot API 50 MB, GitHub Contents 1 MB
   inline / 100 MB release, Drive multipart ~5 MB, Photos ~200 MB → classify as
   `too_large:` *before* the request, and route to the chunked path (see 11).
10. **GitHub draft releases leak** — `:264` creates drafts that are never published or
    garbage-collected. Publish on success or delete on failure.
11. **Resumable/chunked upload** (`transfer.rs`): Drive upload sessions for >5 MB
    (`:928-947` is one multipart shot), GitHub release assets via `Content-Range`
    (`:303-309` is one body). Expose as `upload_file_chunked` on backends that support it.
12. **Retry/backoff/rate limits** (`retry.rs`, `rate_limit.rs`): currently **zero** retry
    handling anywhere. Honor `429`/`Retry-After`/`X-RateLimit-*`/Telegram `retry_after`,
    exponential backoff with jitter, per-provider concurrency cap (today the rayon pool in
    `pipeline.rs:143` fans out N parallel requests at one provider), and one **shared**
    `reqwest` client with connection reuse (today each call builds a client, `:88-95`).
13. **Integrity verification** (`transfer.rs`): BLAKE3 the bytes before upload and after
    download; compare against `Content-Length`/provider checksum. Return
    `integrity: ...` on mismatch. AGENT-2 gates `delete_raw_after_sync` on this.
14. **Quota probing** (`quota.rs`): Drive `about?fields=quotaInfo` (`:1093` currently asks
    for `user` only), GitHub rate-limit headers, GitLab/Telegram best-effort.
15. **Self-hosted GitLab instance URL is overloaded onto `base_path`** — `:592-609`, factory
    `:1694`. Add a dedicated `instance_url` handling (read-only w.r.t. `SyncConfig` — if a
    new field is truly required, file a request to AGENT-2, who owns `SyncConfig`).

### P2 — API surface

16. **Extend the trait non-breakingly** (`crates/types/src/sync.rs:158`) with defaulted
    methods so AGENT-2 is never blocked:

```rust
pub struct Capabilities { pub max_size_bytes: u64, pub supports_delete: bool,
    pub supports_direct_download: bool, pub recursive_list: bool, pub chunked: bool }

pub trait StorageBackend: Send + Sync {
    // ... existing 8 methods unchanged ...
    fn capabilities(&self) -> Capabilities { Capabilities::default() }
    fn stat(&self, _remote_path: &str) -> Result<Option<RemoteFile>, String> {
        Ok(None) // unknown → AGENT-2 treats as "no conflict info"
    }
}
```

17. Remove `#[allow(dead_code)]` from the Google credential paths you touch; call
    `oauth::get_valid_token(...)` (contract below) before every authenticated request.

## Contracts

**Error string prefixes — you define, others map on them.** Every `Err` you return must
start with exactly one of: `auth:`, `rate_limited:`, `not_found:`, `unsupported:`,
`too_large:`, `integrity:`, `network:` (then `: ` then detail).
AGENT-2 maps these to `SyncStatus`/retry policy; AGENT-5 renders them in the UI.

**You provide:** `Capabilities` + defaulted trait methods; `retry::with_retry(policy, op)`;
`rate_limit::acquire(backend_type)`; `transfer::verify_blake3(...)`; `quota::usage(config)`.

**You consume:** `crates/sync/src/oauth.rs::get_valid_token(&mut creds, token_url, buffer)`
(existing signature, `oauth.rs:128`). Wire it in your backends as-is — AGENT-3 guarantees
credentials are actually populated and refreshed; until then it is a harmless no-op for
PAT-based configs.

**You must NOT change:** the 8 existing method signatures (AGENT-2's pipeline and
AGENT-5's UI compile against them), or the `SyncConfig`/`SyncFile` structs (AGENT-2 owns).

## Verification

- `cargo fmt --all --check` after each edit batch.
- CI green on the `rust-check` job (workspace clippy is Agent-4's fix; until it lands,
  ensure at least the crate compiles via fmt + CI).
- For each provider: a short manual test matrix in your Log (upload → list → stat →
  download → byte-compare → delete → list confirms gone) — record real API responses.

## Definition of done

- [ ] No backend method ever returns `Ok` for an operation it did not perform.
- [ ] All 7 P0 items closed; P1 items 8–14 closed; trait extension merged.
- [ ] Every request path goes through retry + rate limiter + shared client.
- [ ] Downloads are byte-verified; oversized files rejected before upload.
- [ ] Error prefixes documented in this file and used consistently.

## Requests to other agents

- **→ AGENT-4 (tests):** `crates/tests/src/backends_contract.rs` asserts the old
  `GitLab ` message prefix at lines **194, 210, 226, 246, 259, 271**
  (`starts_with("GitLab ")` / `starts_with("GitLab delete failed")`). The new error
  contract requires one of the 7 prefixes first, so those six lines will fail CI until
  updated. Everything else in that file still matches (`contains("list failed (401|404|429|500)")`,
  `contains("HTTP 500")` for probes, `contains("Failed to read file")`, `HEAD ` method
  for the upload existence check, GitLab `/-/raw/` and `/-/blob/` URL shapes, `GitLab `
  as a substring still passes). Suggested helper for your file:
  `assert_classified(err: &str)` → `assert!(["auth: ", "rate_limited: ", "not_found: ",
  "unsupported: ", "too_large: ", "integrity: ", "network: "].iter().any(|p| err.starts_with(p)),
  "unclassified error: {err}")`. Please keep the existing `contains(...)` assertions.
- **→ AGENT-2 (optional, P1 item 15):** a dedicated `instance_url` field on `SyncConfig`
  for self-hosted GitLab. Until it exists, `create_backend` resolves the instance base as
  `CYBERMANJU_GITLAB_INSTANCE_URL` env var → `base_path` → `https://gitlab.com`
  (`gitlab_instance_base` in `backends.rs`, also used by `quota.rs`). No change needed if
  the env/`base_path` workaround is acceptable.

## Log

- 2026-10-04 — closed F1–F17 (P0 1–7, P1 8–15, P2 16–17) — commit: uncommitted per ground
  rules (working tree holds all 5 agents' changes) — CI: **not yet run** (no local toolchain;
  `cargo fmt --all` clean).
  *Files:* `backends.rs` rewritten (3157 lines: shared `http_client`/`send_once`/
  `send_classified`/`send_probe` plumbing + 6 backends + factory), `retry.rs`,
  `rate_limit.rs`, `transfer.rs`, `quota.rs`, `lib.rs` (mods/re-exports), `Cargo.toml`
  (+`blake3`), `types/src/sync.rs` (`Capabilities` + defaulted `capabilities`/`stat`/
  `upload_file_chunked`).
  *Verification done locally:* `cargo fmt --all` exits 0; scripted contract sweep over
  `backends.rs` — all 115 error-message sites take a `retry::` prefix constant, zero
  bare `Err("...")`, zero `unwrap()`/`TODO`/`#[allow(dead_code)]`, no leftover removed
  helpers; cross-file refs (`quota.rs` → `send_classified`/`http_client`/`urlencoding`/
  `gitlab_instance_base`, `pipeline.rs` → `create_backend`) resolve; base64/json! call
  styles match CI-green HEAD code.
  *Not executed (cannot be faked):* real-API test matrix (upload → list → stat →
  download → byte-compare → delete) and workspace clippy/test — deferred to CI.
  *Expected CI state:* my crates green except AGENT-4's 6 prefix assertions above
  (filed as request); `Cargo.lock` regeneration is AGENT-4's DoD, untouched here.
  *Design notes:* every locator is something `upload_file` itself returns (GitHub
  release-tag URLs, Drive file URLs, Telegram `file_id`, Photos media-item id), so
  pipeline re-uploads accept their own output; Telegram/Photos/Drive/Local honor
  `remote_path` (folder chain / description / caption); shared GitHub release tag is
  `cybermanju-sync-{blake3(remote)[..16]}` with publish-on-success/delete-on-failure;
  Telegram errors strip URLs (`without_url`) so the bot token never lands in logs.
- 2026-10-04 — error-prefix contract (post-CI fix) — commit: this push — CI: pending.
  The `retry::{AUTH,…}` constants were **bare tokens** in the Log's contract claim but
  shipped with a trailing `:` while every message builder already renders
  `format!("{}: …", …)`, so constant-built errors read `network:: …`, `classify` still
  matched, and AGENT-4's `starts_with` asserts did not. Constants are now bare
  (`"auth"`, `"network"`, …); `http_error` renders through the new `prefix_head()`,
  which accepts a token with or without its colon and emits `auth: ` (empty head for
  unclassified), so the hand-written `"rate_limited:"` literals callers pass keep
  reading correctly. `lib.rs` re-exports `ErrorClass` alongside `classify_error`/
  `RetryPolicy` so contract tests can assert classes instead of string prefixes.
