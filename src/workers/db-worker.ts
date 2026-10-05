// Cybermanju Drive — database Web Worker (static/WASM build only)
//
// redb's `StorageBackend` is fully synchronous, and the only synchronous
// file primitive in a browser is `FileSystemSyncAccessHandle`, which exists
// **only inside Dedicated Workers**. So the database runs here: this worker
// loads the wasm bundle, opens the real `cybermanju.db` in OPFS (in-memory
// fallback), and answers `{id, op, args}` messages with `{id, ok, data?,
// error?}`.
//
// The main thread never touches redb directly — `wasmDbDispatch()` in
// `useWasmBackend.ts` is the only caller, and `invoke()` is already async
// everywhere, so no Atomics/spinlock ferry is needed (those only exist for
// callers that insist on *synchronous* calls from the main thread).
import init, { db_dispatch, db_open, db_restore, db_snapshot } from 'cybermanju-drive-wasm'

interface DbRequest {
  id: number
  op: string
  args?: Record<string, unknown>
}

const DB_FILE = 'cybermanju.db'
const BACKUP_FILE = 'cybermanju.backup.db'
const SNAPSHOT_MS = 5 * 60 * 1000

let opened: Promise<{ backend: string }> | null = null
let dirty = false
let snapshotTimer = 0

// Read-only ops never dirty the database.
function isReadOnly(op: string): boolean {
  return (
    op === '_status' ||
    op === '_snapshot' ||
    op === '_restore' ||
    op.endsWith('.list') ||
    op.endsWith('.get') ||
    op === 'volume.df' ||
    op === 'disks.check' ||
    op === 'sync.secret'
  )
}

async function opfsRoot(): Promise<any> {
  const nav = navigator as unknown as { storage: { getDirectory: () => Promise<any> } }
  return nav.storage.getDirectory()
}

async function readBackupBytes(): Promise<Uint8Array | null> {
  try {
    const root = await opfsRoot()
    const fh = await root.getFileHandle(BACKUP_FILE)
    const file = await fh.getFile()
    const buf: ArrayBuffer = await file.arrayBuffer()
    if (buf.byteLength === 0) return null
    return new Uint8Array(buf)
  } catch {
    return null
  }
}

async function mainSize(): Promise<number | null> {
  try {
    const root = await opfsRoot()
    const fh = await root.getFileHandle(DB_FILE)
    const file = await fh.getFile()
    return file.size as number
  } catch {
    return null
  }
}

async function writeBackup(bytes: Uint8Array): Promise<number> {
  const root = await opfsRoot()
  const fh = await root.getFileHandle(BACKUP_FILE, { create: true })
  const writable = await fh.createWritable()
  await writable.write(bytes)
  await writable.close()
  return bytes.byteLength
}

async function snapshotNow(): Promise<number> {
  const view = db_snapshot() as unknown as Uint8Array
  const n = await writeBackup(view)
  dirty = false
  return n
}

function armSnapshotTimer() {
  if (snapshotTimer) return
  snapshotTimer = self.setInterval(() => {
    if (!dirty || !opened) return
    snapshotNow().catch(() => {
      // Best-effort: the live database is unaffected by backup failure.
    })
  }, SNAPSHOT_MS)
}

function ensureOpen(): Promise<{ backend: string }> {
  if (!opened) {
    opened = (async () => {
      await init()
      // Crash recovery: a missing/empty main file with a valid backup
      // means the previous session died mid-write — restore first.
      try {
        const [size, backup] = await Promise.all([mainSize(), readBackupBytes()])
        if (backup && backup.byteLength > 0 && (size === null || size === 0)) {
          const root = await opfsRoot()
          const fh = await root.getFileHandle(DB_FILE, { create: true })
          const writable = await fh.createWritable()
          await writable.write(backup)
          await writable.close()
        }
      } catch {
        // Recovery is best-effort; a fresh database still opens below.
      }
      const raw = await db_open()
      const env = JSON.parse(raw as string) as { ok: boolean; data?: { backend?: string }; error?: string }
      if (!env.ok) throw new Error(env.error || 'db_open failed')
      armSnapshotTimer()
      return { backend: String(env.data?.backend ?? 'unknown') }
    })()
    // A failed open must be retryable, not sticky.
    opened.catch(() => {
      opened = null
    })
  }
  return opened
}

self.onmessage = async (ev: MessageEvent<DbRequest>) => {
  const { id, op, args } = ev.data ?? ({} as DbRequest)
  const post = (msg: Record<string, unknown>) => {
    self.postMessage({ id, ...msg })
  }
  try {
    const info = await ensureOpen()
    if (op === '_status') {
      post({ ok: true, data: { backend: info.backend, file: 'cybermanju.db' } })
      return
    }
    if (op === '_snapshot') {
      const n = await snapshotNow()
      post({ ok: true, data: { bytes: n, file: BACKUP_FILE } })
      return
    }
    if (op === '_restore') {
      const backup = await readBackupBytes()
      if (!backup) {
        post({ ok: false, error: 'not_found: no backup image (cybermanju.backup.db)' })
        return
      }
      const out = db_restore(backup) as string
      const env = JSON.parse(out) as { ok: boolean; data?: unknown; error?: string }
      if (env.ok) {
        dirty = false
        post({ ok: true, data: env.data ?? null })
      } else {
        post({ ok: false, error: String(env.error ?? 'restore failed') })
      }
      return
    }
    if (!isReadOnly(op)) dirty = true
    const out = db_dispatch(
      op,
      JSON.stringify({ ...(args ?? {}), now: new Date().toISOString() }),
    ) as string
    const env = JSON.parse(out) as { ok: boolean; data?: unknown; error?: string }
    if (env.ok) post({ ok: true, data: env.data ?? null })
    else post({ ok: false, error: String(env.error ?? 'unknown db error') })
  } catch (e) {
    post({ ok: false, error: e instanceof Error ? e.message : String(e) })
  }
}
