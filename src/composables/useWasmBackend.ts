// Cybermanju Drive — WASM Backend Bridge
//
// The GitHub Pages build ships no dashboard behind it, so REST calls to
// `http://localhost:3456` are doomed to `ERR_CONNECTION_REFUSED`. Instead,
// when the app runs from a non-3456 origin (i.e. the WASM / Pages pack),
// the OS-layer commands are served by the `cybermanju-drive-wasm` crate:
// `os_dispatch(cmd, args_json)` answers the terminal, task table, volume df
// and workers against a virtual volume kept in localStorage.
//
// Data APIs (files, accounts, collections, …) have NO wasm implementation —
// they are database-backed and stay REST-only. Those callers get a single
// clear "dashboard not reachable" error instead of per-request fetch spam.

interface WasmBackend {
  os_dispatch: (cmd: string, argsJson: string) => string
}

let wasmModule: WasmBackend | null = null
let wasmLoad: Promise<WasmBackend> | null = null

/**
 * Load the wasm-pack bundle (`--target web` output). Vite sees the
 * virtual module via the resolved alias in `vite.config.wasm.ts`; when the
 * bundle isn't present (plain dev server, dashboard origin) the load fails
 * and we degrade back to REST.
 */
async function loadWasm(): Promise<typeof wasmModule> {
  if (wasmModule) return wasmModule
  if (!wasmLoad) {
    wasmLoad = (async () => {
      const mod = (await import('cybermanju-drive-wasm')) as unknown as WasmBackend & {
        default: () => Promise<void>
      }
      await mod.default()
      wasmModule = mod
      return mod
    })()
  }
  return wasmLoad
}

/** True once the wasm backend has been loaded (or loading has started). */
export function wasmBackendActive(): boolean {
  return wasmModule !== null
}

/** Dispatch one os/* command through the wasm crate. Throws on transport errors. */
export async function wasmOsDispatch(cmd: string, args: Record<string, unknown> = {}): Promise<unknown> {
  const mod = await loadWasm()
  if (!mod) throw new Error('wasm backend unavailable')
  const payload = JSON.stringify({ args: argsToArgList(cmd, args) })
  const raw = mod.os_dispatch(cmd, payload)
  try {
    return JSON.parse(raw)
  } catch {
    return raw
  }
}

/**
 * Map the os/* REST contract onto the dispatcher's arg-list contract.
 * The wasm dispatcher takes positional string args; the frontend passes
 * named ones.
 */
function argsToArgList(cmd: string, args: Record<string, unknown>): string[] {
  const list: string[] = []
  switch (cmd) {
    case 'exec': list.push(String(args.line ?? '')); break
    case 'complete': list.push(String(args.prefix ?? '')); break
    case 'stat':
    case 'ls':
    case 'du':
    case 'cat':
      if (args.path) list.push(String(args.path))
      break
    case 'compute':
      if (args.job) list.push(String(args.job))
      if (args.path) list.push(String(args.path))
      break
    case 'search':
      if (args.query) list.push(String(args.query))
      break
    default:
      break
  }
  return list
}

/**
 * Search the wasm volume with BM25-lite — the only data-shape API the wasm
 * pack implements, exposed as a source for `search_files` on Pages.
 */
export async function wasmSearchFiles(query: string): Promise<Array<{ path: string; score: number }>> {
  const mod = await loadWasm()
  if (!mod) return []
  const raw = mod.os_dispatch('search', JSON.stringify({ args: [query] }))
  try {
    const parsed = JSON.parse(raw) as { ok?: boolean; output?: string }
    if (parsed.ok === false || !parsed.output) return []
    return parsed.output
      .split('\n')
      .map(line => line.trim())
      .filter(Boolean)
      .map(path => ({ path, score: 1.0 }))
  } catch {
    return []
  }
}
