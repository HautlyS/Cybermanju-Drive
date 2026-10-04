# AGENT-6 — Disk & Volume Substrate (the `.cybermanju`)

**Part of the 3-agent Cybermanju OS push.** The product vision: a *decentralized OS* whose
storage lives on third-party providers, where each provider holds a `.cybermanju` virtual
disk of **adjustable size**, all of them merged into **one** logical volume that is
encrypted and compressed — the more providers you connect, the more space you get. This
brief owns the **disk and volume layer**: making that object real, sized, mountable and
capacity-aware.

- **AGENT-6 (this file)** — `.cybermanju` disks, allocator, volume merge, mount, `df`, admission control
- AGENT-7 — durability, scrub, repair, erasure coding, catalog replication, multi-writer
- AGENT-8 — `cybsh` terminal, task model, compute fan-out, syscall boundary, UI

Read [`MISSING.md`](./MISSING.md) first — this brief closes **A, B, D** there.

## Ground rules

1. **Three agents share this worktree right now.** Only touch the files in *Files you
   own*. Shared files were pre-wired by the supervisor and are **read-only** — if you need
   a route segment or a table, add a line under *Requests to the supervisor* instead.
2. **Never edit:** `crates/web/src/lib.rs`, `crates/web/src/security.rs`,
   `crates/web/src/api/mod.rs`, `crates/db/src/database.rs`, root `Cargo.toml`,
   `crates/tests/src/lib.rs`, `package.json`, `.github/**`, `src/**` (AGENT-8 owns the
   frontend), `crates/os/**` (AGENT-8), `crates/erasure/**` + `scrub/repair/gc/health/lease`
   (AGENT-7).
3. **Verify locally** (this repo now builds on this machine — see *Verification*).
   Export `ORT_CACHE_DIR="$HOME/.cache/ort"` before any cargo command, or `ort-sys` panics.
4. **Do not run `git` commands.** The supervisor commits and pushes after each green gate.
5. Append a dated line to **Log** after each unit of work.

## Files you own

| Path | Notes |
|---|---|
| `crates/disk/**` | **new** crate — superblock, allocator, volume, mount, block API |
| `crates/db/src/database.rs` *usage only* | call `Database::get_disks_table()` etc. — never edit the file |
| `crates/sync/src/pipeline.rs` | **admission-control hook only** — smallest possible edits inside `// <<< AGENT-6 … >>>` markers |
| `crates/sync/src/quota.rs` | surface disk-aware usage (AGENT-1 file — surgical, marked edits) |
| `src-tauri/src/commands/disk.rs` | **new** — Tauri command wrappers (register them by adding to *Requests*) |
| `crates/web/src/api/disk_api.rs` | **pre-created stub** — fill in `route()`, it is already hooked up |
| `crates/tests/src/disk.rs` | **pre-created stub** — your test module |
| `AGENT-6.md` | this file |

## Work items

### P0 — the disk object must exist

- [ ] **1. `.cybermanju` superblock** (`crates/disk/src/superblock.rs`)
      Header: magic `CYBMJU1`, format `version`, volume UUID, provider/config id,
      `capacity_bytes`, `created_at`/`updated_at`, key handle, block-map BLAKE3 checksum,
      footer pointer to the block map. Serialize to bytes, **keystore-seal** it (reuse
      `cybermanju_crypto::keystore`, same `CYBE1`-style magic convention), and make the
      round-trip (`encode → seal → open → decode → verify checksum`) a test.
- [ ] **2. Disk lifecycle** (`crates/disk/src/disk.rs`)
      `create(config_id, size_bytes, passphrase)` · `attach` · `detach` ·
      `resize(new_size)` (grow **and** shrink-with-refusal-if-data-would-not-fit) ·
      `check()` (fsck: superblock checksum, block-map vs. reality, orphans) · `destroy`.
      Persist each disk as a row in the `disks` table (already declared).
- [ ] **3. Choosable size is enforced, not decorative**
      `size_bytes` is what the user picked; `create` rejects `0` and rejects a size larger
      than the provider's reported free/quota (`cybermanju_sync::quota::usage`). `df`
      reports this number as capacity.
- [ ] **4. Block allocator** (`crates/disk/src/allocator.rs`)
      LBA space with a free/used bitmap, BLAKE3 content-addressing for dedup, and a
      **refcount** per chunk hash so AGENT-7's GC can distinguish referenced from orphaned.
      Unit tests: no double-allocation, free-list round-trip, refcount monotonicity.

### P0 — merge into one volume

- [ ] **5. Volume manager** (`crates/disk/src/volume.rs`)
      One logical volume composed of N attached disks. `df()` returns
      `{ total_bytes, used_bytes, free_bytes, disks: [{id, provider, size, used, free,
      health}] }` — this is the "more providers → more space" number, and it must **grow
      when a disk is attached** (test asserts exactly that).
- [ ] **6. Spanned allocation (D3)** — placement fills a disk to its high-water mark
      before spilling to the next, honouring each disk's `capacity_bytes`. Round-robin
      stays available as a policy flag but **spanned is the default** for volume writes.
- [ ] **7. Admission control (D1)** — add a check at the top of the write path in
      `crates/sync/src/pipeline.rs` (inside a marked `// <<< AGENT-6 ADMISSION >>>` block):
      refuse with `Err("disk full: …")` when the volume has no free bytes for the payload.
      Test: fill a 1 MiB fake disk, assert the next write fails with `disk full` **and
      that no partial upload is left behind**.

### P1 — mount, so it is a disk and not a sync job

- [ ] **8. HTTP block API** — `GET/PUT /api/volume/block/{lba}` and
      `GET /api/volume/df` in `crates/web/src/api/disk_api.rs` (already routed). This is
      the **portable** path: Termux/Android has no `/dev/fuse`, Docker and the web build
      use this. Range support, BLAKE3 verification on read, auth-gated like everything else.
- [ ] **9. FUSE mount, cfg-gated** — `cybermanju mount <path>` via `fuser` behind
      `#[cfg(all(unix, not(target_os = "android")))]` **plus** a runtime `/dev/fuse` probe
      that degrades to a clear "FUSE unavailable, use the HTTP block API" message instead
      of a panic. Never break the build on Termux.
- [ ] **10. Tauri commands** — `create_disk`, `attach_disk`, `detach_disk`, `resize_disk`,
      `list_disks`, `volume_df`, `check_disk` in `src-tauri/src/commands/disk.rs`; add the
      `mod disk;` + `invoke_handler` lines under *Requests to the supervisor* (you may not
      edit `lib.rs`).

### P2 — polish

- [ ] **11. `df`/`du` in the shell vocabulary** — expose `crates/disk` through
      AGENT-8's syscall boundary (`open/stat/readdir/df`); coordinate via *Requests*, do
      not edit `crates/os/**`.
- [ ] **12. Docs** — append a "Disks & Volumes" section to your *Log*; AGENT-8 owns
      README/ARCHITECTURE.

## Contracts (do not change unilaterally)

- Superblock magic `CYBMJU1`; artifact magic stays `CYBE1` (AGENT-1/2 own it).
- Tables: `disks`, `volumes`, `block_map` — already declared in
  `crates/db/src/database.rs`; use `Database::get_disks_table()` etc.
- `disk_api::route(db, method, path_segments, body, origin) -> Option<String>` — return
  `None` for anything that is not yours (the stub already does).
- JSON keys are **camelCase** (serde `rename_all`), matching every existing route.
- Errors are `Err("reason: detail")` with a machine-readable prefix (`disk_full:`,
  `integrity:`, `unsupported:`) — matches AGENT-1's error-prefix contract.

## Verification

```bash
export ORT_CACHE_DIR="$HOME/.cache/ort"
cargo fmt --all -- --check
cargo clippy -p cybermanju-disk --all-targets -- -D warnings
cargo test  -p cybermanju-disk
cargo test  -p cybermanju-tests disk
```

Run the workspace gate only when you are done — it is expensive on this machine:
`cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace --no-fail-fast`

## Definition of done

- [ ] Every checkbox above is `[x]`
- [ ] `cargo test -p cybermanju-disk` green, `cargo clippy -p cybermanju-disk -D warnings` clean
- [ ] `scripts/os-acceptance.sh` **Tier 0** passes (create two sized disks → attach →
      `df` reports the merged total → data spans both disks → byte-identical read-back →
      artifacts sealed `CYBE1` and compressed → all driven through `cybsh`)
- [ ] No edit outside *Files you own* (checked by `git status` in the supervisor gate)
- [ ] **Log** section below has a dated entry per unit of work

## Requests to the supervisor

<!-- Add numbered requests here; the supervisor applies them between gates. -->
<!-- e.g. "R6-1: add `mod disk;` + these 7 invoke_handler lines to src-tauri/src/lib.rs" -->

## Log

- _(append dated entries: `YYYY-MM-DD — item N — gate status`)_
