// Virtual module stub for `cybermanju-drive-wasm` in desktop builds.
// The real wasm backend is only compiled into the static/WASM bundle
// (vite.config.wasm.ts). On desktop the OS layer goes through Tauri IPC /
// same-origin REST, so the lazy import must resolve to something — this
// stub throws only if actually loaded, which never happens on desktop
// (isStaticHost() is false there).
const stub = `
export function os_dispatch() {
  throw new Error('cybermanju-drive-wasm is not available in the desktop build')
}
export async function db_open() {
  throw new Error('cybermanju-drive-wasm is not available in the desktop build')
}
export function db_dispatch() {
  throw new Error('cybermanju-drive-wasm is not available in the desktop build')
}
export default function init() {
  return Promise.resolve()
}
`

export default {
  name: 'cybermanju-drive-wasm-stub',
  resolveId(id: string) {
    if (id === 'cybermanju-drive-wasm') return '\0cybermanju-drive-wasm-stub'
    return null
  },
  load(id: string) {
    if (id === '\0cybermanju-drive-wasm-stub') return stub
    return null
  },
}
