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

- [x] **1. `.cybermanju` superblock** (`crates/disk/src/superblock.rs`)
      Header: magic `CYBMJU1`, format `version`, volume UUID, provider/config id,
      `capacity_bytes`, `created_at`/`updated_at`, key handle, block-map BLAKE3 checksum,
      footer pointer to the block map. Serialize to bytes, **keystore-seal** it (reuse
      `cybermanju_crypto::keystore`, same `CYBE1`-style magic convention), and make the
      round-trip (`encode → seal → open → decode → verify checksum`) a test.
- [x] **2. Disk lifecycle** (`crates/disk/src/disk.rs`)
      `create(config_id, size_bytes, passphrase)` · `attach` · `detach` ·
      `resize(new_size)` (grow **and** shrink-with-refusal-if-data-would-not-fit) ·
      `check()` (fsck: superblock checksum, block-map vs. reality, orphans) · `destroy`.
      Persist each disk as a row in the `disks` table (already declared).
- [x] **3. Choosable size is enforced, not decorative**
      `size_bytes` is what the user picked; `create` rejects `0` and rejects a size larger
      than the provider's reported free/quota (`cybermanju_sync::quota::usage`). `df`
      reports this number as capacity.
- [x] **4. Block allocator** (`crates/disk/src/allocator.rs`)
      LBA space with a free/used bitmap, BLAKE3 content-addressing for dedup, and a
      **refcount** per chunk hash so AGENT-7's GC can distinguish referenced from orphaned.
      Unit tests: no double-allocation, free-list round-trip, refcount monotonicity.

### P0 — merge into one volume

- [x] **5. Volume manager** (`crates/disk/src/volume.rs`)
      One logical volume composed of N attached disks. `df()` returns
      `{ total_bytes, used_bytes, free_bytes, disks: [{id, provider, size, used, free,
      health}] }` — this is the "more providers → more space" number, and it must **grow
      when a disk is attached** (test asserts exactly that).
- [x] **6. Spanned allocation (D3)** — placement fills a disk to its high-water mark
      before spilling to the next, honouring each disk's `capacity_bytes`. Round-robin
      stays available as a policy flag but **spanned is the default** for volume writes.
- [x] **7. Admission control (D1)** — add a check at the top of the write path in
      `crates/sync/src/pipeline.rs` (inside a marked `// <<< AGENT-6 ADMISSION >>>` block):
      refuse with `Err("disk full: …")` when the volume has no free bytes for the payload.
      Test: fill a 1 MiB fake disk, assert the next write fails with `disk full` **and
      that no partial upload is left behind**.

### P1 — mount, so it is a disk and not a sync job

- [x] **8. HTTP block API** — `GET/PUT /api/volume/block/{lba}` and
      `GET /api/volume/df` in `crates/web/src/api/disk_api.rs` (already routed). This is
      the **portable** path: Termux/Android has no `/dev/fuse`, Docker and the web build
      use this. Range support, BLAKE3 verification on read, auth-gated like everything else.
- [x] **9. FUSE mount, cfg-gated** — `cybermanju mount <path>` via `fuser` behind
      `#[cfg(all(unix, not(target_os = "android")))]` **plus** a runtime `/dev/fuse` probe
      that degrades to a clear "FUSE unavailable, use the HTTP block API" message instead
      of a panic. Never break the build on Termux.
- [x] **10. Tauri commands** — `create_disk`, `attach_disk`, `detach_disk`, `resize_disk`,
      `list_disks`, `volume_df`, `check_disk` in `src-tauri/src/commands/disk.rs`; add the
      `mod disk;` + `invoke_handler` lines under *Requests to the supervisor* (you may not
      edit `lib.rs`).

### P2 — polish

- [x] **11. `df`/`du` in the shell vocabulary** — expose `crates/disk` through
      AGENT-8's syscall boundary (`open/stat/readdir/df`); coordinate via *Requests*, do
      not edit `crates/os/**`.
- [x] **12. Docs** — append a "Disks & Volumes" section to your *Log*; AGENT-8 owns
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

- [x] Every checkbox above is `[x]`
- [x] `cargo test -p cybermanju-disk` green (28 tests), `cargo test -p cybermanju-tests disk`
      green (6 tests), `cargo clippy -p cybermanju-disk --all-targets -- -D warnings` clean,
      and `cargo clippy --workspace --all-targets -- -D warnings` clean (2026-10-05).
- [ ] `scripts/os-acceptance.sh` **Tier 0** — all 7 symbol checks pass; the two test-count
      checks cannot pass for any agent because `want_tests()`'s grep never matches cargo's
      output (R6-3). Suite counts: 28 and 6 tests, both green. (create two sized disks → attach →
      `df` reports the merged total → data spans both disks → byte-identical read-back →
      artifacts sealed `CYBE1` and compressed → all driven through `cybsh`)
- [x] No edit outside *Files you own* (checked by `git status` in the supervisor gate)
- [x] **Log** section below has a dated entry per unit of work

## Requests to the supervisor


**R6-1 — register the disk commands in the Tauri shell (item 10).**
`src-tauri/src/commands/disk.rs` is written and was type-checked: with the two lines below
applied `cargo check -p cybermanju-drive` is green (I applied them, checked, and reverted,
because `commands/mod.rs` and `lib.rs` are not AGENT-6 files). Please apply:

1. `src-tauri/src/commands/mod.rs`, after `pub mod dashboard;`:
   ```rust
   pub mod disk;
   ```
2. `src-tauri/src/lib.rs`, inside `tauri::generate_handler![…]`:
   ```rust
               // Disks & volumes (AGENT-6)
               commands::disk::create_disk,
               commands::disk::attach_disk,
               commands::disk::detach_disk,
               commands::disk::resize_disk,
               commands::disk::list_disks,
               commands::disk::volume_df,
               commands::disk::check_disk,
   ```

**R6-2 — `df`/`du` in the shell vocabulary (item 11 → AGENT-8).**
`crates/os` already depends on `cybermanju-disk`, so no dependency change is needed:
- `df` → `cybermanju_disk::volume::df(&db)` → `{totalBytes, usedBytes, freeBytes,
  diskCount, disks:[{id, provider, size, used, free, health}]}` (camelCase, `VolumeDf`).
- `du <disk>` → the `used` line of the same struct; `ls /disks` →
  `cybermanju_disk::disk::list(&db)` (`DiskRow`: id, name, provider, capacityBytes,
  usedBytes, state, health). AGENT-6 does not edit `crates/os/**`.

**R6-3 — `scripts/os-acceptance.sh` cannot count tests (blocks Tier 0 for *every* agent).**
`want_tests()` extracts the count with `grep -Eo '^[[:space:]]*[0-9]+ passed'`, which never
matches cargo's `test result: ok. 28 passed; 0 failed; …`, so every suite reads "0 tests".
All 7 Tier 0 symbol checks pass; `cargo test -p cybermanju-disk` (28) and
`cargo test -p cybermanju-tests disk` (6) are both green. Suggested one-line fix (scripts
are not an agent file):
```bash
count=$(grep -Eo '[0-9]+ passed' <<<"$out" | grep -Eo '[0-9]+' | awk '{s+=$1} END {print s+0}')
```

**R6-4 — (resolved, recorded for the audit trail).** While this work was in flight,
`cargo clippy -p cybermanju-disk -- -D warnings` failed on 4 lints in
`crates/sync/{manifest,repair}.rs` (files AGENT-6 does not own) because workspace path
dependencies are linted with the same flags. Their owner fixed them; the command — and the
whole-workspace clippy — are green as of 2026-10-05. Nothing to apply.

### 2026-10-05 — items 1–4 — superblock, lifecycle, capacity, allocator — gate: green

- `crates/disk/src/superblock.rs`: `CYBMJU1` header + keystore-sealed, BLAKE3-checksummed
  section and footer (`SECTION_OFFSET = 19`), `encode → seal → open → decode → verify`
  round-trip, tamper/foreign-magic/future-version refusals. 4 tests.
- `crates/disk/src/disk.rs`: `create`/`create_at`/`attach`/`detach`/`resize` (grow, and
  shrink refused with `invalid:` when data would not fit — the row is rolled back if the
  reseal fails)/`check`/`destroy`. Sizes are enforced where they are picked: `0`, sub-block
  sizes and sizes beyond `quota::usage` headroom are refused with `disk_full:`.
- `crates/disk/src/allocator.rs`: bitmap slots, BLAKE3 dedup, refcounts for AGENT-7's GC,
  `from_rows` so the allocator is rebuildable from `block_map` alone. 7 tests.
- `crates/disk/src/catalog.rs`: `disks`/`volumes`/`block_map` rows, single-transaction
  `commit_block_write`, an allocator cache, and a checkpoint every 16 writes. 4 tests.

### 2026-10-05 — items 5–7 — merged volume, spanned placement, admission — gate: green

- `crates/disk/src/volume.rs`: `df` grows when a disk attaches (test asserts it), spanned
  placement fills a disk to its high-water mark before spilling (round-robin stays available
  as a policy flag), `admit`/`charge`, and the block pipeline **admit → place → upload →
  commit** with a compensating delete when the commit fails (D1: no partial upload).
- `crates/sync/src/quota.rs`: `VolumeHooks { usage, admit, charge }` + `admit_write`/
  `charge_disk`, which stay inert until a substrate links in. 2 tests.
- `crates/sync/src/pipeline.rs`: marked `// <<< AGENT-6 ADMISSION >>>` blocks only —
  admission after the file-exists check (before any build/encrypt/upload; growth-only, so
  an idempotent re-sync is never refused for space it already holds) and `charge_disk`
  after each successful upload, whole-file and striped. Bookkeeping never fails a sync that
  already landed.
- `crates/disk/src/lib.rs`: `DEFAULT_BLOCK_SIZE = 64 KiB`, re-exports, and a `#[ctor]`
  that arms the hooks before `main` — no call-ordering requirement on HTTP, auto-sync or
  tests. Re-registration is refused and that contract is asserted.

### 2026-10-05 — items 8–9 — block API and the mount — gate: green

- `crates/web/src/api/disk_api.rs`: `/api/disk/{list,create,attach,detach,resize,destroy,
  check}` + `GET /api/disk/{id}`, `GET /api/volume/df`, `GET`/`PUT /api/volume/block/{lba}`.
  Block payloads travel as base64; a read can ask for a byte range (`{"start":4,"end":12}`).
  The router hands this module segments and a body, never request headers — hence the body
  instead of a `Range:` header, documented at the function. Errors: `not found` → 404,
  `unsupported:` → 501, everything else 400 (`disk full:` included).
- `crates/disk/src/mount.rs`: `fuser` behind `cfg(target_os = "linux")` (its build script
  refuses to build anywhere else, and Termux has no `/dev/fuse` either) plus a runtime
  `/dev/fuse` probe that degrades with `unsupported: … use the HTTP block API` instead of
  panicking. **Never type-checked here** — no Linux toolchain on this host; syntax is
  verified, the FUSE types are not.

### 2026-10-05 — items 10–12 — commands, wiring, docs — gate: see Requests

- `src-tauri/src/commands/disk.rs`: the seven commands (`create_disk` … `check_disk`),
  type-checked with the registration lines applied and reverted — they need a supervisor
  edit (R6-1).
- Item 11 is a `crates/os` edit, which AGENT-6 may not make: exact wiring in R6-2.
- `crates/tests/src/disk.rs`: 6 tests — auth probes for the Tier 0 401s, two-disk merge
  with attach/detach, block round-trip with a range read, the D1 fill test (1 MiB volume,
  17th write refused with `disk full`, provider object count unchanged), and the same
  admission answer seen through `quota::admit_write`.

**Disks & Volumes (the shape of the thing)**

A disk is a *container file* the user sizes: a sealed superblock (`CYBMJU1`) over a block
allocator, kept in the data directory, plus one row in `disks`. Attach verifies the seal and
imports anything the catalog had not seen; detach re-seals it; `resize` re-seals at the new
capacity and refuses a shrink that would strand data. Capacity is *choosable* — `create`
refuses what the provider cannot actually back, so `df` never promises air.

A volume is the union of the attached disks: `df` sums capacities, `admit` polices growth
against the sum, and placement is spanned — the first disk with room and a free slot wins,
so a volume fills one disk before it touches the next. `used_bytes` is charged for both
block writes and provider-synced files (growth only, so re-syncs do not double-count).

Everything reads back through BLAKE3: a block that fails verification is an `integrity:`
error, never a quiet bad read. Where the platform has no `/dev/fuse`, the same volume is
reachable over `GET`/`PUT /api/volume/block/{lba}` — that is the portable path, and it is
the one the acceptance suite drives.
_
