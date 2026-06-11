// Cybermanju Drive — Tauri IPC Composable
// Supports both Tauri desktop (IPC) and Server/Web (REST) modes
//
// In Tauri mode: delegates to @tauri-apps/api/core invoke
// In Web mode: calls the Web Dashboard REST API (port 3456 by default)

import type { FileNode } from '@/types'

// ── Module-level connection state ─────────────────────────────

let _serverUrl = ''
let _authToken = ''

/** Configure the Web Dashboard REST API base URL. */
export function setServerUrl(url: string): void {
  _serverUrl = url.replace(/\/+$/, '')
}

/** Configure the Bearer token for ZimaOS JWT auth. */
export function setAuthToken(token: string): void {
  _authToken = token
}

/** Read the current server URL (for diagnostics). */
export function getServerUrl(): string {
  return _serverUrl
}

// ── Environment Detection ────────────────────────────────────

export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI__' in window
}

export function isWebMode(): boolean {
  return !isTauri()
}

// ── REST helpers ─────────────────────────────────────────────

/** Resolve the base URL for REST calls. */
function getBaseUrl(): string {
  if (_serverUrl) return _serverUrl
  // If the app is served directly from the web dashboard, use same origin
  if (typeof window !== 'undefined' && window.location?.origin) {
    const port = window.location.port
    // Port 3456 is the web dashboard — use same origin
    if (port === '3456') return window.location.origin
  }
  return 'http://localhost:3456'
}

/** Build headers including optional auth. */
function buildHeaders(): Record<string, string> {
  const h: Record<string, string> = {
    'Content-Type': 'application/json',
    'Accept': 'application/json',
  }
  if (_authToken) {
    h['Authorization'] = `Bearer ${_authToken}`
  }
  return h
}

/** Generic REST fetch with proper error handling. */
async function restFetch<T>(method: string, path: string, body?: unknown): Promise<T> {
  const url = `${getBaseUrl()}${path}`
  const init: RequestInit = {
    method,
    headers: buildHeaders(),
  }
  if (body !== undefined) {
    init.body = JSON.stringify(body)
  }

  let res: Response
  try {
    res = await fetch(url, init)
  } catch (err) {
    throw new Error(
      `Network error calling ${method} ${path}: ${err instanceof Error ? err.message : String(err)}`
    )
  }

  if (!res.ok) {
    let message = `HTTP ${res.status} ${res.statusText}`
    try {
      const errBody = await res.json()
      if (errBody?.message) message = errBody.message
      else if (errBody?.error) message = `${res.status}: ${errBody.error}`
    } catch {
      // ignore parse failure
    }
    throw new Error(message)
  }

  // 204 No Content
  if (res.status === 204) return undefined as T

  return res.json() as Promise<T>
}

// ── Response key transformation ──────────────────────────────
// The REST API returns raw redb JSON. Field names may be snake_case
// (Rust default). The Tauri IPC layer uses serde camelCase renaming.
// We convert snake_case → camelCase so responses match TypeScript types.
// The `_key` and `_raw` fields added by list_all_json are stripped.

function toCamelCase(s: string): string {
  return s.replace(/_([a-z])/g, (_, ch: string) => ch.toUpperCase())
}

function transformResponseKeys(value: unknown): unknown {
  if (value === null || value === undefined || typeof value !== 'object') return value
  if (Array.isArray(value)) return value.map(transformResponseKeys)

  const src = value as Record<string, unknown>
  const out: Record<string, unknown> = {}
  for (const [key, val] of Object.entries(src)) {
    // Strip _raw field (unparseable fallback from list_all_json)
    if (key === '_raw') continue
    // Promote _key → id when the object lacks its own id
    if (key === '_key') {
      if (!('id' in src)) {
        out['id'] = val
      }
      continue
    }
    out[toCamelCase(key)] = transformResponseKeys(val)
  }
  return out
}

// ── REST command mapping ────────────────────────────────────
// Maps Tauri IPC command names → Web Dashboard REST endpoints.

interface RestMapping {
  method: string
  buildPath: (args: Record<string, unknown>) => string
  transformRequest?: (args: Record<string, unknown>) => unknown
  transformResponse?: (raw: unknown, args: Record<string, unknown>) => unknown
}

const REST_ROUTES: Record<string, RestMapping> = {
  // ── Files ─────────────────────────────────────────────────
  list_files: {
    method: 'GET',
    buildPath: () => '/api/files',
    transformResponse: (raw, args) => {
      let files = transformResponseKeys(raw) as FileNode[]
      // The REST API returns ALL files — filter by parentPath if provided
      const parentPath = args.parentPath as string | undefined
      if (parentPath) {
        files = files.filter(f => {
          if (f.parentId) return f.parentId === parentPath
          if (f.path) {
            // Match files whose path starts with parentPath and have no deeper separator
            const prefix = parentPath === '/' ? '/' : `${parentPath}/`
            return f.path.startsWith(prefix) && !f.path.slice(prefix.length).includes('/')
          }
          return false
        })
      }
      return files
    },
  },

  get_file: {
    method: 'GET',
    buildPath: (args) => `/api/files/${args.fileId}`,
  },

  delete_file: {
    method: 'DELETE',
    buildPath: (args) => `/api/files/${args.fileId}`,
  },

  search_files: {
    method: 'GET',
    buildPath: (args) => `/api/search?q=${encodeURIComponent(String(args.query ?? ''))}`,
    transformResponse: (raw) => {
      // REST returns raw file objects; map to SearchResult shape
      const items = transformResponseKeys(raw) as Array<Record<string, unknown>>
      return items.map(item => ({
        fileId: item.id ?? '',
        fileName: item.name ?? '',
        score: 1.0,
        snippet: '',
      }))
    },
  },

  get_geo_files: {
    method: 'GET',
    buildPath: () => '/api/geo-files',
    transformResponse: (raw) => {
      const items = transformResponseKeys(raw) as Array<Record<string, unknown>>
      return items
        .filter(f => f.gpsLat != null && f.gpsLon != null)
        .map(f => ({
          id: f.id,
          name: f.name,
          gpsLat: f.gpsLat as number,
          gpsLon: f.gpsLon as number,
        }))
    },
  },

  // ── Accounts ──────────────────────────────────────────────
  list_accounts: {
    method: 'GET',
    buildPath: () => '/api/accounts',
  },

  // ── Collections ───────────────────────────────────────────
  list_collections: {
    method: 'GET',
    buildPath: () => '/api/collections',
  },

  get_collection_items: {
    method: 'GET',
    buildPath: () => '/api/collection-items',
  },

  // ── Face groups ───────────────────────────────────────────
  list_face_groups: {
    method: 'GET',
    buildPath: () => '/api/face-groups',
  },

  // ── Loose groups ──────────────────────────────────────────
  list_loose_groups: {
    method: 'GET',
    buildPath: () => '/api/loose-groups',
  },

  // ── Encryption ────────────────────────────────────────────
  get_encryption_status: {
    method: 'GET',
    buildPath: () => '/api/encryption/status',
    transformResponse: (raw) => {
      const data = transformResponseKeys(raw) as Record<string, unknown>
      return {
        isEncrypted: false,
        algorithm: undefined,
        nistLevel: undefined,
        keyId: undefined,
        encryptedAt: undefined,
        // Include extra info from the REST response
        available: data.available ?? false,
        supportedAlgorithms: data.supportedAlgorithms ?? [],
        engine: data.engine ?? '',
      }
    },
  },

  list_keys: {
    method: 'GET',
    buildPath: () => '/api/encryption/keys',
  },

  // ── User management ───────────────────────────────────────
  list_users: {
    method: 'GET',
    buildPath: () => '/api/users',
  },

  authenticate_user: {
    method: 'POST',
    buildPath: () => '/api/users/login',
    transformRequest: (args) => ({
      username: args.username,
      password: args.password,
    }),
    transformResponse: (raw) => transformResponseKeys(raw),
  },

  register_user: {
    method: 'POST',
    buildPath: () => '/api/users/register',
    transformRequest: (args) => ({
      username: args.username,
      password: args.password,
      displayName: args.displayName,
      role: args.role,
    }),
    transformResponse: (raw) => transformResponseKeys(raw),
  },

  // ── Permissions ───────────────────────────────────────────
  get_file_permissions: {
    method: 'GET',
    buildPath: (args) => `/api/permissions/${args.fileId}`,
  },

  grant_file_permission: {
    method: 'POST',
    buildPath: () => '/api/permissions',
    transformRequest: (args) => ({
      userId: args.userId,
      fileId: args.fileId,
      access: args.access,
    }),
  },

  verify_file_access: {
    method: 'POST',
    buildPath: () => '/api/permissions/verify',
    transformRequest: (args) => ({
      userId: args.userId,
      fileId: args.fileId,
      requiredAccess: args.requiredAccess,
    }),
  },

  // ── Locations ─────────────────────────────────────────────
  list_locations: {
    method: 'GET',
    buildPath: () => '/api/locations',
  },

  // ── Dashboard ─────────────────────────────────────────────
  dashboard_status: {
    method: 'GET',
    buildPath: () => '/api/health',
    transformResponse: (raw) => {
      const data = transformResponseKeys(raw) as Record<string, unknown>
      return {
        running: data.status === 'ok',
        port: 3456,
        url: getBaseUrl(),
        activeConnections: 0,
        service: data.service,
        timestamp: data.timestamp,
      }
    },
  },
}

// Commands that exist in Tauri but have NO REST equivalent (write-heavy / Tauri-only).
const WRITE_ONLY_COMMANDS = new Set([
  'create_folder',
  'rename_file',
  'duplicate_file_context',
  'create_collection',
  'add_to_collection',
  'remove_from_collection',
  'create_account',
  'switch_account',
  'detect_faces',
  'generate_keypair',
  'encrypt_file',
  'decrypt_file',
  'compress_file',
  'decompress_file',
  'parse_file',
  'start_dashboard',
  'stop_dashboard',
  'start_sync',
  'cancel_sync',
  'create_sync_config',
  'delete_sync_config',
  'test_sync_connection',
  'list_remote_files',
  'revoke_file_permission',
  'list_sync_configs',
  'get_sync_progress',
])

/** The core invoke — works in both Tauri and Web modes. */
async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri()) {
    // ── Tauri IPC path ────────────────────────────────────
    return (await import('@tauri-apps/api/core')).then(m => m.invoke<T>(cmd, args))
  }

  // ── Web / REST path ────────────────────────────────────
  const mapping = REST_ROUTES[cmd]
  if (mapping) {
    const path = mapping.buildPath(args ?? {})

    // Allow the mapping to transform the request body
    let body: unknown = undefined
    if (mapping.transformRequest && args) {
      body = mapping.transformRequest(args)
    } else if (mapping.method !== 'GET' && mapping.method !== 'HEAD' && args) {
      body = args
    }

    const raw = await restFetch<unknown>(mapping.method, path, body)

    if (mapping.transformResponse) {
      return (mapping.transformResponse(raw, args ?? {})) as T
    }

    return transformResponseKeys(raw) as T
  }

  // Write-only / unsupported commands in web mode
  if (WRITE_ONLY_COMMANDS.has(cmd)) {
    throw new Error(
      `[Web Mode] Command "${cmd}" requires the Tauri desktop app and is not available through the Web Dashboard REST API.`
    )
  }

  // Unknown command — attempt a best-effort REST call
  console.warn(`[Web Mode] Unknown Tauri command "${cmd}" — no REST mapping exists.`)
  throw new Error(
    `[Web Mode] Command "${cmd}" is not supported. The Web Dashboard REST API does not provide this endpoint.`
  )
}

// ── Composable ──────────────────────────────────────────────

export function useTauri() {
  async function pickFolder(): Promise<string | null> {
    if (isWebMode()) {
      // Native folder picker not available in web mode
      return null
    }
    const { open } = await import('@tauri-apps/plugin-dialog')
    const result = await open({ directory: true, multiple: false })
    return result as string | null
  }

  async function pickFiles(multiple = false): Promise<string[] | null> {
    if (isWebMode()) {
      return null
    }
    const { open } = await import('@tauri-apps/plugin-dialog')
    const result = await open({ directory: false, multiple })
    return result as string[] | null
  }

  async function readDirectory(path: string): Promise<FileNode[]> {
    if (isWebMode()) {
      // Use REST API to list files, then filter by path prefix
      try {
        const allFiles = await invoke<FileNode[]>('list_files', {})
        return allFiles
          .filter(f => {
            if (!f.path) return false
            const dir = f.path.substring(0, f.path.lastIndexOf('/')) || '/'
            return dir === path || (path === '/' && !f.path.includes('/'))
          })
          .map((f, i) => ({
            ...f,
            id: f.id || `web-${i}-${f.name}`,
          }))
      } catch {
        return []
      }
    }
    const { readDir } = await import('@tauri-apps/plugin-fs')
    const entries = await readDir(path)
    return entries.map((entry, i) => ({
      id: `local-${i}-${entry.name}`,
      name: entry.name,
      path: `${path}/${entry.name}`,
      fileType: (entry as unknown as { isDirectory: boolean }).isDirectory ? 'folder' as const : 'file' as const,
      sizeBytes: 0,
      encrypted: false,
      compressionLayers: [],
      createdAt: new Date().toISOString(),
      modifiedAt: new Date().toISOString(),
    }))
  }

  async function readTextFile(path: string): Promise<string> {
    if (isWebMode()) {
      throw new Error('[Web Mode] Direct file reads are not available. Use the REST API file endpoints.')
    }
    const { readFile } = await import('@tauri-apps/plugin-fs')
    return await readFile(path) as unknown as string
  }

  async function writeTextFile(path: string, contents: string): Promise<void> {
    if (isWebMode()) {
      throw new Error('[Web Mode] Direct file writes are not available through the Web Dashboard.')
    }
    const { writeFile } = await import('@tauri-apps/plugin-fs')
    await writeFile(path, new TextEncoder().encode(contents))
  }

  async function createDir(path: string): Promise<void> {
    if (isWebMode()) {
      throw new Error('[Web Mode] Directory creation is not available through the Web Dashboard.')
    }
    const { mkdir } = await import('@tauri-apps/plugin-fs')
    await mkdir(path, { recursive: true })
  }

  async function deletePath(path: string): Promise<void> {
    if (isWebMode()) {
      throw new Error('[Web Mode] File deletion is not available through the Web Dashboard.')
    }
    const { remove } = await import('@tauri-apps/plugin-fs')
    await remove(path)
  }

  async function renamePath(oldPath: string, newPath: string): Promise<void> {
    if (isWebMode()) {
      throw new Error('[Web Mode] File renaming is not available through the Web Dashboard.')
    }
    const { rename } = await import('@tauri-apps/plugin-fs')
    await rename(oldPath, newPath)
  }

  async function copyPath(src: string, dest: string): Promise<void> {
    if (isWebMode()) {
      throw new Error('[Web Mode] File copying is not available through the Web Dashboard.')
    }
    const { copyFile } = await import('@tauri-apps/plugin-fs')
    await copyFile(src, dest)
  }

  async function pathExists(path: string): Promise<boolean> {
    if (isWebMode()) {
      try {
        await restFetch<unknown>('GET', `/api/files`)
        return true
      } catch {
        return false
      }
    }
    const { exists } = await import('@tauri-apps/plugin-fs')
    return await exists(path)
  }

  return {
    invoke,
    pickFolder,
    pickFiles,
    readDirectory,
    readTextFile,
    writeTextFile,
    createDir,
    deletePath,
    renamePath,
    copyPath,
    pathExists,
    isTauri,
    isWebMode,
  }
}