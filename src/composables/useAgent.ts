// Cybermanju — browser-local agent loop (static-host / Pages transport).
//
// The WASM dispatcher exposes single-turn primitives (`agent_catalog`,
// `agent_prompt`); this composable drives the multi-turn loop entirely in
// the browser: transcript, tool execution against the volume dispatcher,
// ask-approval parking, usage accounting, and localStorage transcripts.
// Provider keys live in memory only — they are never written to storage.
//
// Mirrors the native permission semantics in `crates/agent/src/config.rs`
// (last-match-wins wildcards, plan denies mutations, deny beats auto).
// Anything the sandbox cannot do answers `unsupported:` — never a fake Ok.

import { ref } from 'vue'
import { wasmAgentCatalog, wasmAgentPrompt } from './useWasmBackend'
import { wasmOsDispatch } from './useWasmBackend'
import type {
  AgentConfig,
  AgentJob,
  AgentSession,
  ChatMessage,
  PermissionRuleset,
  ProviderPreset,
  TokenUsage,
  ToolCall,
} from '@/types'

const SESSIONS_KEY = 'cybermanju.agent.sessions.v1'
const CONFIGS_KEY = 'cybermanju.agent.configs.v1'

/** Shell-style wildcard: `*` spans any run, `?` exactly one char. */
export function matchWildcard(pattern: string, input: string): boolean {
  const px = pattern
  const memo = new Map<string, boolean>()
  function go(pi: number, ii: number): boolean {
    const key = `${pi}:${ii}`
    const hit = memo.get(key)
    if (hit !== undefined) return hit
    let out: boolean
    if (pi >= px.length) {
      out = ii >= input.length
    } else if (px[pi] === '*') {
      let p = pi
      while (p < px.length && px[p] === '*') p++
      out = false
      for (let skip = ii; skip <= input.length; skip++) {
        if (go(p, skip)) {
          out = true
          break
        }
      }
    } else if (px[pi] === '?') {
      out = ii < input.length && go(pi + 1, ii + 1)
    } else {
      out = ii < input.length && input[ii] === px[pi] && go(pi + 1, ii + 1)
    }
    memo.set(key, out)
    return out
  }
  return go(0, 0)
}

/** Glob matcher mirroring Rust `config::match_glob`: `*`/`?` stay inside
 *  one segment, `**` crosses separators. */
export function matchGlob(pattern: string, path: string): boolean {
  const trim = (s: string) => s.replace(/\\/g, '/').split('/').filter(Boolean)
  const px = trim(pattern.length ? pattern : '**')
  const ix = trim(path)
  function go(pi: number, ii: number): boolean {
    if (pi >= px.length) return ii >= ix.length
    if (px[pi] === '**') {
      let p = pi + 1
      while (p < px.length && px[p] === '**') p++
      for (let skip = ii; skip <= ix.length; skip++) {
        if (go(p, skip)) return true
      }
      return false
    }
    if (ii >= ix.length) return false
    return matchWildcard(px[pi], ix[ii]) && go(pi + 1, ii + 1)
  }
  return go(0, 0)
}

/** Regex when it compiles, literal substring when it does not. */
export function compileGrep(pattern: string): { test: (line: string) => boolean; regex: boolean } {
  try {
    const re = new RegExp(pattern)
    return { test: line => re.test(line), regex: true }
  } catch {
    return { test: line => line.includes(pattern), regex: false }
  }
}

function matchInput(tool: string, input: Record<string, unknown>): string {
  const arg = salientArg(input)
  return arg ? `${tool} ${arg}` : tool
}

function salientArg(input: Record<string, unknown>): string {
  return (
    (input.command as string) ??
    (input.pattern as string) ??
    (input.path as string) ??
    (input.glob as string) ??
    (input.query as string) ??
    (input.url as string) ??
    ''
  )
}

export type LocalDecision = { kind: 'allow' } | { kind: 'ask'; summary: string } | { kind: 'deny'; reason: string }

/** Mirror of Rust `config::decide` — keep semantics identical. */
export function decideLocalTool(
  rules: PermissionRuleset,
  agentKind: 'build' | 'plan',
  tool: string,
  input: Record<string, unknown>,
): LocalDecision {
  if (agentKind === 'plan' && (tool === 'edit' || tool === 'write' || tool === 'bash')) {
    return { kind: 'deny', reason: `deny: plan agent may not run \`${tool}\`` }
  }
  const rule = rules.rules[tool]
  let action = rules.default
  if (typeof rule === 'string') {
    action = rule
  } else if (Array.isArray(rule)) {
    const target = matchInput(tool, input)
    const arg = salientArg(input)
    for (const [pattern, act] of rule) {
      if (
        matchWildcard(pattern, target) ||
        matchWildcard(pattern, tool) ||
        (arg !== '' && matchWildcard(pattern, arg))
      ) {
        action = act
      }
    }
  }
  if (action === 'allow') return { kind: 'allow' }
  if (action === 'deny') return { kind: 'deny', reason: `deny: \`${tool}\` is denied by the permission ruleset` }
  return { kind: 'ask', summary: `Approve \`${tool}\`?` }
}

// ─── volume tools (WASM dispatcher) ─────────────────────────────

async function wasmRead(path: string): Promise<string> {
  const res = (await wasmOsDispatch('exec', { line: `cat "${path}"` })) as {
    ok: boolean
    output: string
  }
  if (!res.ok) throw new Error(res.output || `not_found: ${path}`)
  return res.output
}

async function wasmWrite(path: string, content: string): Promise<string> {
  const res = (await wasmOsDispatch('write', { path, content })) as {
    ok: boolean
    output: string
  }
  if (!res.ok) throw new Error(res.output || 'write failed')
  return res.output
}

async function wasmList(path: string): Promise<string> {
  const res = (await wasmOsDispatch('ls', { path })) as { ok: boolean; output: string } | string[]
  if (Array.isArray(res)) return res.join('\n')
  if (!res.ok) throw new Error(res.output || `not_found: ${path}`)
  return res.output
}

/** Mirror of Rust `edit::apply_edit` (exact-once anchored replacement). */
function applyEditLocal(current: string, oldBlock: string, newBlock: string): string {
  if (!oldBlock) throw new Error('invalid: old_block is empty')
  const hits = current.split(oldBlock).length - 1
  if (hits === 0) throw new Error('not_found: old_block does not occur in the file — re-read and retry')
  if (hits > 1) throw new Error(`conflict: old_block occurs ${hits} times — resend a larger, unique block`)
  return current.replace(oldBlock, newBlock)
}

async function execLocalTool(
  call: { name: string; input: Record<string, unknown> },
  cwd: string,
): Promise<string> {
  const join = (p: string) => {
    const raw = String(p || '')
    if (raw.startsWith('/')) return raw
    return cwd === '/' ? `/${raw}` : `${cwd}/${raw}`
  };
  switch (call.name) {
    case 'read': {
      return wasmRead(join(String(call.input.path ?? '')))
    }
    case 'list': {
      const p = String(call.input.path ?? '')
      return wasmList(p ? join(p) : cwd)
    }
    case 'write': {
      const path = join(String(call.input.path ?? ''))
      const content = String(call.input.content ?? '')
      if (content.length > 1024 * 1024) throw new Error('too_large: content exceeds the 1 MiB browser write cap')
      await wasmWrite(path, content)
      return `wrote ${path} (${content.length} bytes)`
    }
    case 'edit': {
      const path = join(String(call.input.path ?? ''))
      const current = await wasmRead(path)
      const updated = applyEditLocal(
        current,
        String(call.input.old_block ?? ''),
        String(call.input.new_block ?? ''),
      )
      await wasmWrite(path, updated)
      return `edited ${path}`
    }
    case 'grep': {
      const rawPattern = String(call.input.pattern ?? '')
      if (!rawPattern) throw new Error('invalid: pattern is required')
      const base = String(call.input.path ?? '')
      const limit = Math.min(50, Math.max(1, Number(call.input.limit ?? 50)))
      const { test } = compileGrep(rawPattern)
      const start = base ? join(base) : cwd
      const listing = await wasmList(start)
      const names = listing.split('\n').map(s => s.trim()).filter(s => s && !s.endsWith('/'))
      const matches: string[] = []
      for (const name of names.slice(0, 200)) {
        const full = start === '/' ? `/${name}` : `${start}/${name}`
        try {
          const text = await wasmRead(full)
          text.split('\n').forEach((line, i) => {
            if (matches.length < limit && test(line)) {
              matches.push(`${full}:${i + 1}: ${line.trim().slice(0, 240)}`)
            }
          })
        } catch {
          // Unreadable entries are skipped, never fatal.
        }
        if (matches.length >= limit) break
      }
      return matches.length ? matches.join('\n') : `no matches for \`${rawPattern}\``
    }
    case 'glob': {
      const pattern = String(call.input.pattern ?? '**') || '**'
      const base = String(call.input.path ?? '')
      const limit = Math.min(200, Math.max(1, Number(call.input.limit ?? 200)))
      const start = base ? join(base) : cwd
      const hits: string[] = []
      const stack = [start]
      let seen = 0
      while (stack.length && hits.length < limit && seen < 2000) {
        const dir = stack.pop()!
        let names: string[]
        try {
          names = (await wasmList(dir)).split('\n').map(s => s.trim()).filter(s => s && !s.startsWith('.'))
        } catch {
          continue
        }
        for (const name of names) {
          if (hits.length >= limit || seen >= 2000) break
          if (name.endsWith('/')) {
            stack.push(dir === '/' ? `/${name.slice(0, -1)}` : `${dir}/${name.slice(0, -1)}`)
            continue
          }
          seen++
          const full = dir === '/' ? `/${name}` : `${dir}/${name}`
          const rel = full === start
            ? name
            : full.startsWith(`${start}/`)
              ? full.slice(start.length + 1)
              : full.replace(/^\//, '')
          if (matchGlob(pattern, rel)) hits.push(full)
        }
      }
      hits.sort()
      return hits.length ? hits.join('\n') : `no files match \`${pattern}\``
    }
    case 'bash':
      throw new Error('unsupported: `bash` needs the desktop app or Docker server — no shell in the browser sandbox')
    case 'task':
      throw new Error('unsupported: subagents are not available in the browser loop yet — break the goal into steps')
    case 'question':
      throw new Error('unsupported: routed through approvals, never executed directly')
    default:
      throw new Error(`unsupported: unknown tool '${call.name}'`)
  }
}

// ─── loop state ─────────────────────────────────────────────────

export interface LocalRunOpts {
  baseUrl: string
  dialect: 'openAi' | 'anthropic'
  model: string
  headers: Array<[string, string]>
  system: string
  maxTurns: number
  permission: PermissionRuleset
  autoApprove: boolean
  agentKind: 'build' | 'plan'
}

export interface ApprovalRequest {
  tool: string
  input: Record<string, unknown>
  summary: string
  question?: string | null
  resolve: (approved: boolean, answer?: string) => void
}

const running = ref(false)
const pendingApproval = ref<ApprovalRequest | null>(null)
let abortRequested = false

/** Ask the running browser loop to stop at the next turn boundary. */
export function abortLocalRun(): void {
  abortRequested = true
  if (pendingApproval.value) {
    const parked = pendingApproval.value
    pendingApproval.value = null
    parked.resolve(false)
  }
}

function waitApproval(req: Omit<ApprovalRequest, 'resolve'>): Promise<{ approved: boolean; answer?: string }> {
  return new Promise(resolve => {
    pendingApproval.value = {
      ...req,
      resolve: (approved, answer) => {
        pendingApproval.value = null
        resolve({ approved, answer })
      },
    }
  })
}

function assistantToolWire(calls: ToolCall[]): unknown[] {
  return calls.map(c => ({ id: c.id, name: c.name, input: c.input }))
}

/** Drive one prompt to completion in the browser. Mutates `messages`. */
export async function runLocalAgent(
  opts: LocalRunOpts,
  messages: ChatMessage[],
  usage: TokenUsage,
  onUpdate?: () => void,
): Promise<{ stopped: 'done' | 'limit' | 'aborted' | 'error'; error?: string }> {
  running.value = true
  abortRequested = false
  try {
    for (let turn = 0; turn < Math.min(50, Math.max(1, opts.maxTurns)); turn++) {
      if (abortRequested) return { stopped: 'aborted' as const }
      const res = await wasmAgentPrompt({
        url: opts.dialect === 'anthropic' ? `${opts.baseUrl.replace(/\/$/, '')}/v1/messages` : `${opts.baseUrl.replace(/\/$/, '')}/chat/completions`,
        dialect: opts.dialect,
        model: opts.model,
        headers: opts.headers,
        system: opts.system,
        messages: messages as unknown as Array<Record<string, unknown>>,
        tools: true,
      })
      if (!res.ok) {
        return { stopped: 'error', error: res.error ?? 'network: provider call failed' }
      }
      const turnData = res.turn!
      usage.inputTokens += turnData.usage.inputTokens ?? 0
      usage.outputTokens += turnData.usage.outputTokens ?? 0
      if (!turnData.tool_calls.length) {
        messages.push({ role: 'assistant', content: turnData.content })
        onUpdate?.()
        return { stopped: 'done' }
      }
      messages.push({
        role: 'assistant_tool',
        content: turnData.content,
        toolInput: assistantToolWire(turnData.tool_calls),
      } as ChatMessage)
      onUpdate?.()
      for (const call of turnData.tool_calls) {
        const input = (call.input ?? {}) as Record<string, unknown>
        if (call.name === 'question') {
          const q = String(input.question ?? 'The agent has a question.')
          const ans = await waitApproval({ tool: 'question', input, summary: q, question: q })
          messages.push({
            role: 'tool',
            content: ans.approved ? `user answered: ${ans.answer || 'approved without comment'}` : 'declined: user declined to answer',
            toolCallId: call.id,
            toolName: call.name,
          })
          onUpdate?.()
          continue
        }
        const decision = decideLocalTool(opts.permission, opts.agentKind, call.name, input)
        if (decision.kind === 'deny') {
          messages.push({
            role: 'tool',
            content: `${decision.reason} — adjust the permission ruleset to allow it`,
            toolCallId: call.id,
            toolName: call.name,
          })
          onUpdate?.()
          continue
        }
        if (decision.kind === 'ask' && !opts.autoApprove) {
          const ans = await waitApproval({ tool: call.name, input, summary: decision.summary, question: null })
          if (!ans.approved) {
            messages.push({
              role: 'tool',
              content: `denied: user rejected \`${call.name}\` — work around it or explain`,
              toolCallId: call.id,
              toolName: call.name,
            })
            onUpdate?.()
            continue
          }
        }
        try {
          const output = await execLocalTool(call, '/')
          messages.push({ role: 'tool', content: output, toolCallId: call.id, toolName: call.name })
        } catch (e) {
          const detail = e instanceof Error ? e.message : String(e)
          messages.push({ role: 'tool', content: `error: ${detail}`, toolCallId: call.id, toolName: call.name })
        }
        onUpdate?.()
      }
    }
    return { stopped: 'limit' }
  } finally {
    running.value = false
  }
}

// ─── local configs + sessions (localStorage; keys never persisted) ──

function readJson<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key)
    if (!raw) return fallback
    return JSON.parse(raw) as T
  } catch {
    return fallback
  }
}

function writeJson(key: string, value: unknown): void {
  try {
    localStorage.setItem(key, JSON.stringify(value))
  } catch {
    // Quota or private mode — sessions simply don't persist.
  }
}

export function listLocalConfigs(): AgentConfig[] {
  return readJson<AgentConfig[]>(CONFIGS_KEY, [])
}

export function saveLocalConfig(config: AgentConfig): void {
  const all = listLocalConfigs().filter(c => c.id !== config.id)
  all.push({ ...config, hasKey: false })
  writeJson(CONFIGS_KEY, all)
}

export function deleteLocalConfig(id: string): void {
  writeJson(
    CONFIGS_KEY,
    listLocalConfigs().filter(c => c.id !== id),
  )
}

export function listLocalSessions(): AgentSession[] {
  const all = readJson<AgentSession[]>(SESSIONS_KEY, [])
  return all.sort((a, b) => (b.updatedAt || '').localeCompare(a.updatedAt || '')).slice(0, 100)
}

export function saveLocalSession(session: AgentSession): void {
  const all = readJson<AgentSession[]>(SESSIONS_KEY, [])
  const i = all.findIndex(s => s.id === session.id)
  if (i >= 0) all[i] = session
  else all.unshift(session)
  writeJson(SESSIONS_KEY, all.slice(0, 100))
}

export function deleteLocalSession(id: string): void {
  writeJson(
    SESSIONS_KEY,
    readJson<AgentSession[]>(SESSIONS_KEY, []).filter(s => s.id !== id),
  )
}

export function useAgent() {
  return {
    running,
    pendingApproval,
    wasmAgentCatalog,
    runLocalAgent,
    listLocalConfigs,
    saveLocalConfig,
    deleteLocalConfig,
    listLocalSessions,
    saveLocalSession,
    deleteLocalSession,
    matchWildcard,
    decideLocalTool,
  }
}

export type LocalAgentJob = {
  jobId: string
  sessionId: string
  configId: string
  status: string
  turnsUsed: number
  maxTurns: number
  usage: TokenUsage
  result?: string | null
  error?: string | null
  pending?: { tool: string; input: Record<string, unknown>; summary: string; question?: string | null } | null
};
