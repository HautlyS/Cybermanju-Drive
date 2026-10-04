# Cybermanju OS — What Is Missing

**Vision:** a decentralized OS over third-party providers. Each provider holds a
`.cybermanju` virtual disk of **adjustable, choosable size**. Every disk is merged into
**one** logical volume. Everything is **encrypted and compressed**. The more providers you
connect, the **more space and the more processing** you have — plus a real system terminal.

**Today's reality:** `crates/sync` is a *file-level sync engine* with per-file striped
placement (4 MiB BLAKE3 chunks round-robined across enabled configs, replication parity).
That is a good substrate — it is not yet a disk, not a volume, and not an OS.

This file is the authoritative gap list. It is split across three briefs:

| Brief | Scope |
|---|---|
| [`AGENT-6.md`](./AGENT-6.md) | **Disk & Volume substrate** — the `.cybermanju` object, sizing, merging, mounting, admission control |
| [`AGENT-7.md`](./AGENT-7.md) | **Durability, repair & multi-writer** — scrub, repair, erasure coding, catalog replication, health, GC, leases |
| [`AGENT-8.md`](./AGENT-8.md) | **OS layer** — `cybsh` system terminal, task model (`ps`/`top`), compute fan-out, syscall boundary, UI |

Owner legend: **[6]** AGENT-6 · **[7]** AGENT-7 · **[8]** AGENT-8 · **[PRE]** pre-wired by the
supervisor before launch (nobody else touches it).

---

## 0. Baseline (measured before launch)

- [ ] `ORT_CACHE_DIR=$HOME/.cache/ort cargo check --workspace --all-targets` — exit status
      *Known blocker:* `crates/faces` has `default = ["onnx-face"]` → `ort` with
      `download-binaries`; `ort-sys`'s `cache_dir()` (`src/internal/dirs.rs:196`) returns
      `None` on Termux and panics with *"could not determine cache directory"*. Fix is the
      `ORT_CACHE_DIR` export used by every gate and every agent invocation in this repo.
      Fallback if the ONNX download/link fails on `aarch64-linux-android`: build
      `cybermanju-faces` with `--no-default-features` for the local gate only (CI/Ubuntu
      keeps `onnx-face`).
- [ ] `cargo test --workspace --no-fail-fast` — exit status
- [ ] `npm run typecheck` — exit status
- [ ] `npm run lint` — exit status

> Every baseline failure is assigned to a brief below rather than left for an agent to
> rediscover.

---

## A. The `.cybermanju` disk object — does not exist **[6]**

- [ ] **A1** No container format: no `.cybermanju` extension, no superblock/header, no
      versioning. The only on-disk magic today is `CYBE1` (`crates/sync/src/pipeline.rs:32`)
      wrapping a *per-file sidecar* artifact — not a disk.
- [ ] **A2** **No adjustable size.** `SyncConfig` has no `capacity_bytes`. `quota.rs`
      queries provider quota at call time and is never persisted, never used for allocation.
- [ ] **A3** No lifecycle: `create --size` / `attach` / `detach` / `resize` (grow **and**
      shrink) / `check` (fsck) / `destroy` per provider.
- [ ] **A4** Disk metadata lives only in local redb (`sync_files.manifest_ref`,
      `crates/db/src/database.rs:42`). The disk is **not self-describing or portable**:
      lose the laptop and the striped pool is orphaned.

## B. Merge into one volume — no address space **[6]**

- [ ] **B1** Striping is per-file and sync-triggered. There is **no virtual block device**:
      no LBA → (provider, offset) map, no logical capacity figure, no mount, no `df`.
- [ ] **B2** No mount path: zero FUSE/NBD code in the repo, and **`/dev/fuse` is
      unavailable on Termux** (`Permission denied`) → FUSE must be `cfg`-gated with a
      portable fallback (HTTP block API) that works on Android/Docker/desktop alike.
- [ ] **B3** No volume manager: attaching a provider changes nothing that is tracked; no
      allocator, no cross-pool free-space accounting, no rebalance on attach/detach.
- [ ] **B4** No refcount/GC on content-addressed chunks — orphaned chunks accumulate forever
      (`chunk_remote_path` is content-addressed, but nothing ever sweeps it).

## C. Durability over the pool — absent **[7]**

- [ ] **C1** No background **scrubber**: `verify_striped` runs only immediately after an
      upload (`crates/sync/src/pipeline.rs:809`). Nothing re-verifies later.
- [ ] **C2** No **repair/rebuild** when a provider dies, quota-exceeds or disconnects.
- [ ] **C3** **Reed–Solomon is explicitly deferred** (`crates/sync/src/manifest.rs:12`) —
      replication-only `parity`, default 1.
- [ ] **C4** Superblock/manifests are never replicated to providers; there is no
      `rebuild-from-remote` path, so the catalog is a single point of failure.
- [ ] **C5** No provider **health scoring**, no quarantine/demotion, no **eviction policy**,
      no **rebalance** when capacity changes.
- [ ] **C6** No chunk **GC** (see B4) and no safety guarantee that GC never deletes
      referenced data.

## D. "More providers = more space" — not delivered **[6]**

- [ ] **D1** No **admission control**: the pipeline never checks remaining disk size before
      writing, so a write can blow straight past a provider's declared capacity.
- [ ] **D2** No **capacity aggregation UI**: `SyncPanel.vue` (127 lines) shows file counts
      only — no total/used/free, no per-disk rows.
- [ ] **D3** Round-robin placement only — no **spanned** allocation (fill provider A's
      high-water mark before spilling to B), so a big provider and a small one contribute
      equally instead of the big one absorbing more.

## E. "More providers = more processing" — zero **[8]**

- [ ] **E1** No compute layer at all: no distributed job scheduler, no provider-side
      execution, no fan-out of the work the app already does (compression, re-index,
      thumbnails, face embeddings).
- [ ] **E2** `Capabilities` (`crates/types/src/sync.rs:296`) measures **storage** limits
      only; nothing scores a provider's compute value, so "more providers → more
      processing" is unrepresentable in the model.

## F. "OS" layer — cosmetic today **[8]**

- [ ] **F1** No **system terminal**. The repo has `DesktopShell.vue`/`Dock.vue` — a window
      manager skin over a file manager. No shell, no commands, no REPL.
- [ ] **F2** No **process/task model**: no `ps`/`top` over sync jobs, scrub, repair or
      compute; no way to `kill` a runaway job from the UI.
- [ ] **F3** No **syscall boundary**: components call sync/web internals directly. There is
      no typed surface (`open/read/write/seek/close/stat/unlink/readdir/mount/df`) for the
      shell or future apps to sit on.
- [ ] **F4** **Single-writer only**: `ConflictPolicy` is last-writer-wins; no leases, no
      locking, no version vectors. Two devices sharing one pool will clobber each other.
- [ ] **F5** No peer-to-peer layer: providers are dumb HTTP APIs driven from one node
      (out of scope for this run — see "Explicitly deferred").

## G. Foundation not yet closed **[PRE + assigned]**

- [ ] **G1** `AUDIT.md` F1–F22, phases 1–7 were still in flight at launch (Phase 1
      "IN PROGRESS"). Baseline gate results in §0 decide which are still live; any that
      are get assigned in a brief rather than ignored.
- [ ] **G2** Docs drift (`AUDIT.md` F14) — README/ARCHITECTURE must describe disks,
      volumes, the terminal and the compute layer once they exist. **[8]** owns docs.

---

## Pre-wired before launch **[PRE]**

Concurrency safety: three agents write to this worktree simultaneously, so every *shared*
file is written once by the supervisor and is thereafter read-only to the agents.

- [ ] `MISSING.md`, `AGENT-6.md`, `AGENT-7.md`, `AGENT-8.md` exist
- [ ] Root `Cargo.toml` lists `crates/disk`, `crates/erasure`, `crates/os`
- [ ] The three crates compile at t=0 (stubs)
- [ ] `crates/db/src/database.rs` declares **all** new tables (`disks`, `volumes`,
      `block_map`, `scrub_runs`, `repairs`, `chunk_refs`, `leases`, `compute_tasks`,
      `shell_history`) + `get_*_table()` accessors — agents only *use* them
- [ ] `crates/web/src/lib.rs` has three `Option<String>` dispatch hooks; `api/mod.rs`
      registers `disk_api` / `repair_api` / `os_api`
- [ ] `crates/web/src/security.rs` `ROUTED_SEGMENTS` includes `disk volume repair scrub os`
- [ ] `crates/tests/src/{disk,repair,os}.rs` stubs registered in `lib.rs`
- [ ] `.gitignore` covers `logs/`
- [ ] `scripts/os-supervisor.sh`, `scripts/os-acceptance.sh` exist

**Rule:** nobody edits `lib.rs`, `security.rs`, `api/mod.rs`, `database.rs`, root
`Cargo.toml` or `crates/tests/src/lib.rs` after launch. Need a route segment or a table?
Add it to your brief's *Requests to the supervisor* section.

---

## Explicitly deferred (not in this run)

- **Peer-to-peer node discovery / device-to-device sync (F5)** — needs a network identity
  and NAT traversal story that does not exist yet. Tracked, not attempted.
- **FUSE on Termux** — impossible without `/dev/fuse`; the HTTP block API is the portable
  path, FUSE ships for Linux/macOS desktop builds only.
- **Provider-side arbitrary code execution** — compute fan-out uses a constrained task ABI
  over the existing workload, not arbitrary remote code.

---

## Acceptance = 100%

`scripts/os-acceptance.sh` passes all three tiers **and** the supervisor gate
(`fmt`, `clippy -D warnings`, `cargo test --workspace`, `typecheck`, `lint`) is green
**and** every checkbox in the three briefs is ticked.
