# AGENT-8 — OS Layer: System Terminal, Tasks, Compute & UI

**Part of the 3-agent Cybermanju OS push.** The product vision: a *decentralized OS* over
third-party providers with `.cybermanju` disks merged into one encrypted volume, and **the
more providers you connect, the more processing you have**. AGENT-6 and AGENT-7 build the
storage; this brief makes it an **operating system** you can actually drive: a real
**system terminal**, a process table, compute fan-out that scales with the provider pool,
a typed syscall boundary, and the UI that shows it all.

- AGENT-6 — `.cybermanju` disks, allocator, volume merge, mount, `df`
- AGENT-7 — durability, scrub, repair, RS, catalog replication, health, GC, leases
- **AGENT-8 (this file)** — `cybsh`, `ps`/`top`, compute fan-out, syscall boundary, UI, docs

Read [`MISSING.md`](./MISSING.md) first — this brief closes **E, F** (and G2, docs).

## Ground rules

1. **Three agents share this worktree right now.** Only touch the files in *Files you
   own*. Shared files were pre-wired by the supervisor and are **read-only** — request
   anything else under *Requests to the supervisor*.
2. **Never edit:** `crates/web/src/lib.rs`, `crates/web/src/security.rs`,
   `crates/web/src/api/mod.rs`, `crates/db/src/database.rs`, root `Cargo.toml`,
   `crates/tests/src/lib.rs`, `.github/**`, `crates/sync/src/{pipeline,manifest,scrub,
   repair,gc,health,lease}.rs`, `crates/disk/**`, `crates/erasure/**`.
   `package.json`: **dependencies only**, append-only (scripts are the supervisor's).
3. **Verify locally.** Export `ORT_CACHE_DIR="$HOME/.cache/ort"` before any cargo command.
4. **Do not run `git` commands.** The supervisor commits and pushes after each green gate.
5. Append a dated line to **Log** after each unit of work.

## Files you own

| Path | Notes |
|---|---|
| `crates/os/**` | **new** crate — shell, task table, compute scheduler, syscall boundary |
| `src/**` | all Vue components, Pinia store, composables, types (40 components) |
| `crates/web/src/api/os_api.rs` | **pre-created stub** — fill in `route()`, already hooked up |
| `crates/drive-wasm/**` | WASM command dispatcher parity |
| `README.md`, `ARCHITECTURE.md`, `AUDIT.md`, `worklog.md` | docs (G1/G2) |
| `crates/tests/src/os.rs` | **pre-created stub** — your test module |
| `AGENT-8.md` | this file |

## Work items

### P0 — the system terminal (F1)

- [x] **1. `cybsh` interpreter** (`crates/os/src/shell.rs`)
      A real, server-side shell — not a fake textarea. Input → tokenizer (quotes, `|` pipes,
      `&&`/`;`, `--json` flags) → command dispatch → formatted output. Commands:
      - `help`, `history`, `clear`, `version`
      - **files:** `ls`, `cd`, `pwd`, `cat`, `cp`, `mv`, `rm`, `mkdir`, `touch`, `stat`, `du`
      - **volume:** `df`, `mount`, `umount`, `disk create|attach|detach|resize|list|check`
      - **providers:** `providers`, `quota`, `sync start|status|cancel`
      - **durability:** `scrub`, `repair`, `gc`, `lease status`
      - **tasks:** `ps`, `top`, `kill <id>`
      - **compute:** `jobs`, `compute run <task>`, `workers`
      - **crypto/search:** `keygen`, `encrypt`, `decrypt`, `search <query>`
      Unknown command → helpful error + did-you-mean; every command has `--json`.
      Persistence for `history` in the `shell_history` table (already declared).
- [x] **2. `TerminalPanel.vue`** — keyboard-driven, ANSI colours, ↑/↓ history,
      **tab-completion from the live command table**, multi-line paste, clickable output,
      openable from the dock and a global hotkey. Same component in the desktop shell and
      the web build (transport-agnostic: it calls `invoke()`).
- [x] **3. REST + WASM parity** — `POST /api/os/exec {cmd, args}` in `os_api.rs` (already
      routed) and the matching dispatcher entry in `crates/drive-wasm`, so the GitHub Pages
      build has the same terminal (AUDIT F1/D4). `useTauri.ts` gains the `os/*` routes.

### P0 — syscall boundary (F3)

- [x] **4. `crates/os/src/api.rs`** — typed surface `open · read · write · seek · close ·
      stat · unlink · readdir · mkdir · mount · df`, implemented **on top of AGENT-6's
      disk/volume API and AGENT-7's catalog** (call their crates; never re-implement
      storage). The shell, the REST routes and future apps all go through this — nothing
      else calls `cybermanju_sync` directly for I/O.
- [x] **5. Wire it up** — `os_api::route` serves `/api/os/*` (exec, stat, ls, df, ps, top,
      jobs, workers); auth-gated by default. Nothing to edit in `lib.rs`/`security.rs` —
      they are already hooked and `os` is already a routed segment.

### P0 — tasks & compute (F2, E1, E2)

- [x] **6. Task table** (`crates/os/src/task.rs`) — every long-running activity (sync run,
      scrub, repair, GC, compute job) is a task with `id, kind, state, progress, started_at,
      bytes, provider`. `ps` lists them, `top` shows live stats, `kill` cancels through the
      existing cancellation handles. Persist to `compute_tasks` (already declared).
- [x] **7. Compute fan-out** (`crates/os/src/compute.rs`) — a scheduler that splits a job
      across N workers: local `rayon` slots **plus** per-provider slots. Workload = the
      expensive things this app already does: batch compression, search re-index, thumbnail
      generation, face embedding. **Attaching a provider must increase parallelism** —
      that is the acceptance test (Tier 2).
- [x] **8. `Capabilities.compute` (E2)** — add a `compute: u32` (concurrent slots a
      provider offers) to `crates/types/src/sync.rs`'s `Capabilities`. **This file is
      shared with AGENT-1** — make a *surgical, marked* edit
      (`// <<< AGENT-8 COMPUTE >>>`) touching only that struct + its `Default`, and note it
      under *Requests*.

### P1 — UI (D2, F1 surface)

- [x] **9. Disk/Volume manager page** — per-provider disk cards with a **size slider**
      (the "adjustable, choosable size"), create/attach/detach/resize buttons, one merged
      **`df` bar** (total/used/free) that grows when a provider is added, per-disk health.
      New `src/components/DiskManagerPage.vue`, wired into the dock/window manager and
      `StorageDashboard.vue`.
- [x] **10. Process/top page** — `src/components/ProcessPanel.vue`: task table with state,
      progress, provider, `kill`; live refresh; fed by `/api/os/ps` and `/api/os/top`.
- [x] **11. Terminal in the shell** — dock entry + hotkey (`` Ctrl+` ``), window title
      `cybsh`, resize handling, and a `StatusBar` indicator when a job is running.
- [x] **12. Store + routes** — `src/stores/app.ts` actions and `REST_ROUTES` entries for
      every new command; Settings shows the active transport.

### P2 — docs & polish

- [ ] **13. Docs (G1/G2)** — in progress, see Log 2026-10-05 — README + ARCHITECTURE sections for disks, volumes, the
      terminal, tasks and compute; reconcile `AUDIT.md` status so docs stop drifting.
- [x] **14. WASM/GH Pages** — `crates/drive-wasm` implements `os/*` against `localStorage`
      (with BM25-lite search) so the Pages build is never an empty shell.
- [x] **15. Accessibility/perf** — terminal keeps 60 fps scrolling on a few thousand lines;
      panel is lazy-loaded.

## Contracts (do not change unilaterally)

- `os_api::route(db, method, path_segments, body, origin) -> Option<String>` — `None` for
  anything that is not yours (the stub already does).
- New REST segments must be **already listed** in `security.rs::ROUTED_SEGMENTS`
  (`os` is pre-added; if you need `tasks` or `jobs` as a *second-level* segment, ask —
  `os/…` nesting needs no new entry).
- All JSON is camelCase; all errors are `"prefix: detail"`.
- Frontend must keep `npm run typecheck` and `npm run lint` green.
- The terminal must work in **all three transports** (tauri / rest / wasm).

## Verification

```bash
export ORT_CACHE_DIR="$HOME/.cache/ort"
cargo fmt --all -- --check
cargo clippy -p cybermanju-os --all-targets -- -D warnings
cargo test  -p cybermanju-os
cargo test  -p cybermanju-tests os
npm run typecheck
npm run lint
```

## Definition of done

- [ ] Every checkbox above is `[x]`
- [ ] `cybsh` executes every listed command with real effects; unknown-command handling and
      tab-completion tested
- [ ] `scripts/os-acceptance.sh` **Tier 2** passes (`ps`/`top` list live tasks · a compute
      job's parallelism **grows when provider B is attached** · `/api/os/*` returns 401
      without a token · typecheck + workspace tests green)
- [ ] No edit outside *Files you own* (except surgical marked edits noted under *Requests*)
- [ ] **Log** section has a dated entry per unit of work

## Requests to the supervisor

- R8-1 (resolved as documented refusal, 2026-10-05): `cybsh sync start` cannot run a
  pipeline worker from `&Database`. Production path is `POST /api/sync/start → 202 {jobId}`
  (Sync panel wired to it), shell polls `sync status`. Message updated in
  `crates/os/src/shell.rs` to say exactly that. No `lib.rs` change needed.
- R8-2 (no new segment needed): durability uses `repair/scrub/lease` (pre-routed) and
  `os/*` nesting; `security.rs::ROUTED_SEGMENTS` already lists all families.

## Log

- _(append dated entries: `YYYY-MM-DD — item N — gate status`)_
- 2026-10-05 — items 1–12, 14–15 implemented; 13 in progress — static review (no toolchain).
  `shell.rs:2057` (tokenizer, pipes, `&&/||/;`, `--json`, did-you-mean, history table),
  `api.rs:861` (Kernel syscall boundary), `task.rs:751` (`ps/top/kill`), `compute.rs:704`
  (local rayon + provider slots via `Capabilities.compute`), `os_api.rs:161` (exec/stat/ls/du/df/ps/top/workers/jobs, 401-gated),
  `drive-wasm/src/os.rs:677` (localStorage volume + BM25-lite, honest `unsupported:` for server ops),
  `TerminalPanel.vue/DiskManagerPage.vue/ProcessPanel.vue` + dock/hotkey wiring,
  `REST_ROUTES` + `REST_FIRST` + full Pinia surface (`execShellLine/completeShellLine/fetchOs*/killOsTask/runComputeJob` + disks + durability + sync jobs/restore/usage).
  This pass added the missing durability/sync-job REST mappings, `SyncConfig` placement/conflict/encrypt fields,
  `describeSyncError` prefix hints, full `SyncPanel.vue` wizard (create/test/start/cancel/quota/restore/remote-delete/browse + OAuth + striped placement),
  honest `sync start` refusal text, `import_from_url` bytes-to-disk (F19), and faces no-fabrication (F7).
  Left for CI: `cargo test -p cybermanju-os`, `-p cybermanju-tests os`, `npm run typecheck/lint`, `scripts/os-acceptance.sh 2`.
