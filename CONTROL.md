# CONTROL — `.cybermanju` file, OAuth-only accounts, browser crypto

Living task tracker. Updated as each task lands. Status legend:
`[ ]` todo · `[~]` in progress · `[x]` done · `[-]` dropped/deferred

**Constraints (locked in):**
- No local Rust toolchain. All `cargo`/`wasm-pack` build + check runs in **GitHub CI** on manual push
  (`.github/workflows/ci.yml` → `rust-check`, `wasm-build` → `cargo check --target wasm32`,
  `wasm-pack build`, `npm run build:wasm:frontend`, Pages deploy).
- Local verification loop = `npm run typecheck` + `npx vitest run`.
- Accounts login: **removed entirely** — OAuth only (Supabase broker).
- **MEGA dropped** (not a sync backend, not a Supabase Auth provider).

**Baseline before work started:** `npm run typecheck` ✅ · `npx vitest run` 8/8 ✅

---

## Phase 1 — `.cybermanju` as a real file on the user's disk

- [x] 1.1 `src/utils/container.ts` — container codec: `CYBMJ01` header, lz4-compress,
      ChaCha20-Poly1305 encrypt (pkg exports), key from passphrase via iterated
      PBKDF2-HMAC-SHA512 (100k, `hmac_sha512`), legacy raw-redb pass-through.
      Pluggable primitives for tests. + `isEncryptedContainer()` header probe.
- [x] 1.2 `tests/frontend/container.test.ts` — round-trip, legacy image, wrong passphrase,
      no-passphrase, compression-skip. (10 tests, green)
- [x] 1.3 `src/utils/idb.ts` — IndexedDB store for the `FileSystemFileHandle`.
- [x] 1.4 `src/workers/db-worker.ts` — `_attach {handle, bytes, passphrase}` → decode →
      `db_restore`, `_save` → `db_snapshot` → encode → `createWritable`, `_export`,
      `_detach`, status folded into `_status.disk`, debounced autosave (2 s),
      `pagehide` flush.
- [x] 1.5 `src/composables/useWasmBackend.ts` — `wasmAttachDisk/wasmSaveDisk/wasmDiskStatus/
      wasmExportDisk/wasmDetachDisk` + explicit "needs the worker" errors in the
      main-thread fallback.
- [x] 1.6 `src/composables/useCybermanjuFile.ts` — open/create picker, re-attach from IDB with
      permission re-request, passphrase prompt for encrypted files, save now,
      export/import fallback (Firefox/Safari have no FSA API), boot restore.

## Phase 2 — config + secrets + file content live inside the file

- [ ] 2.1 Rust `crates/db/src/database.rs` — `kv` table + `get_kv_table()`.
- [ ] 2.2 Rust `crates/drive-wasm/src/db.rs` — open `kv` in `open_all_tables`;
      ops `kv.get` / `kv.set` / `kv.delete` / `kv.list`; `files.create`; `files.patch`.
- [ ] 2.3 `src/composables/useVault.ts` — kv-backed secrets with localStorage fallback
      (old `pkg/` has no `kv.*` until CI rebuilds → must degrade silently).
- [ ] 2.4 `useSupabase.ts` — hydrate config from vault at boot; write through to vault.
- [ ] 2.5 Volume mirror: shell files (`os_write`) mirrored into `kv` under `volume:*` and
      replayed at boot, so terminal file contents also persist in `.cybermanju`.
- [ ] 2.6 File content: `read_file_content` / `write_file_content` / `upload_file` static routes
      backed by `kv content:<fileId>` (today they throw "needs the dashboard" in WASM mode).

## Phase 3 — Accounts window: OAuth pickers + disk, no password

- [ ] 3.1 Delete `src/components/LoginPopup.vue` + `App.vue` import/render.
- [ ] 3.2 Remove `showLoginPopup` from `stores/app.ts` (+ 401 auto-popup) and from
      `CommandPalette.vue` / `TopMenuBar.vue` actions.
- [ ] 3.3 `useSupabase.ts` — `startSupabaseSignIn(provider)` + identity (session → `currentUser`).
- [ ] 3.4 `AccountManagerPanel.vue` — remove `[SESSION] APP LOGIN` and `[LOCAL] DEVICE ACCOUNTS`;
      add `[IDENTITY]` OAuth pickers (Google / GitHub / GitLab, sign out) and
      `[DISK] THIS MACHINE` (open / create / save / export / import + status).
- [ ] 3.5 Keep `[PROVIDERS]` OAuth connect + `.cybermanju` size slider per provider (moved up).

## Phase 4 — encryption + compression actually work in the WASM build

- [ ] 4.1 `src/composables/useWasmCrypto.ts` — keypair gen (X25519 / ML-DSA-65 / ChaCha20 key),
      file encrypt/decrypt, compress/decompress over pkg exports, marker encoding.
- [ ] 4.2 `src/composables/useTauri.ts` — static-host routes for `generate_keypair`, `list_keys`,
      `get_encryption_status`, `encrypt_file`, `decrypt_file`, `compress_file`, `decompress_file`.
- [ ] 4.3 `EncryptionPanel.vue` / `CompressionPanel.vue` — replace `webLocked = isWebMode()`
      with a real capability check, delete the "NEEDS THE TAURI APP / FOR REFERENCE" copy.
- [ ] 4.4 Add `brotli` to `COMPRESSION_INFO` (desktop `compress_file` already accepts it).

## Verification

- [ ] 5.1 `npm run typecheck` green.
- [ ] 5.2 `npx vitest run` green (old 8 + new suites).
- [ ] 5.3 `npm run build:wasm:frontend` (dist-wasm bundle builds with committed `pkg/`).
- [ ] 5.4 CI green on push: rust-check, clippy, wasm32 check, wasm-pack, dist-wasm, Pages.

---

## Notes / decisions

- **Persistence without Rust:** `db_restore(bytes)` (`db.rs:897`) loads a whole redb image and
  `db_snapshot()` (`db.rs:863`) returns one (256 MiB cap) — so open/save of a user-picked file is
  pure TypeScript + File System Access API. `FileSystemFileHandle` is structured-cloneable →
  posted to `db-worker`, where redb already runs (OPFS sync handle).
- **Container encryption is JS-side**, using pkg exports (`chacha20_*`, `hkdf_derive`, `blake3_hash`,
  `compress_lz4`) — no new Rust crypto.
- **`kv.*` is the one hard Rust requirement** (secrets/content cannot reach redb otherwise).
  Until CI builds it, `useVault` falls back to localStorage so every phase stays testable.
- **Honest scope:** FSA API is Chromium-only (export/import fallback elsewhere); provider *network
  sync* still needs the desktop app / Docker / dashboard server — that message stays.
- **Browser crypto honesty:** the wasm pkg has X25519 + ML-DSA-65 + ChaCha20-Poly1305 but no
  ML-KEM (Kyber). Keygen for `kyber*`/`hybrid`/`frodokem*` produces X25519 material and file
  encryption is ChaCha20-Poly1305 with a keypair-derived key; the panel says so.
