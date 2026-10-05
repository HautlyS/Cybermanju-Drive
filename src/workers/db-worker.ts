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
import init, { db_dispatch, db_open } from 'cybermanju-drive-wasm'

interface DbRequest {
  id: number
  op: string
  args?: Record<string, unknown>
}

let opened: Promise<{ backend: string }> | null = null

function ensureOpen(): Promise<{ backend: string }> {
  if (!opened) {
    opened = (async () => {
      await init()
      const raw = await db_open()
      const env = JSON.parse(raw as string) as { ok: boolean; data?: { backend?: string }; error?: string }
      if (!env.ok) throw new Error(env.error || 'db_open failed')
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
