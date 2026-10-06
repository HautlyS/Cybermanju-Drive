<template>
  <div class="agent-panel">
    <div class="panel-header">
      <div class="header-left">
        <span class="icon-agent"><AppIcon name="solar:bot-bold" /></span>
        <h2 class="panel-title">AI AGENT</h2>
        <span class="text-muted transport-tag">{{ transportLabel }}</span>
      </div>
      <button class="bw-btn small" @click="showSetup = !showSetup">{{ showSetup ? 'HIDE SETUP' : 'SETUP' }}</button>
    </div>

    <!-- Provider + config setup -->
    <div v-if="showSetup" class="section wizard">
      <h3 class="section-title"><AppIcon name="solar:add-bold" :size="13" /> PROVIDER PRESETS ({{ allPresets.length }})</h3>
      <div class="preset-grid">
        <button
          v-for="p in allPresets"
          :key="p.id"
          class="preset-card"
          :class="{ on: form.providerId === p.id }"
          @click="pickPreset(p)"
          :title="`${p.baseUrl} · ${p.defaultModel}`"
        >
          <span class="preset-name">{{ p.label }}</span>
          <span class="preset-meta text-muted">{{ p.family }} · {{ p.defaultModel }}</span>
          <span v-if="p.keyless" class="preset-free">NO KEY</span>
        </button>
      </div>

      <label class="w-label">CONFIG NAME <input v-model="form.name" class="bw-input" placeholder="MY AGENT" /></label>
      <div class="w-row">
        <label class="w-label">MODEL
          <span class="model-row">
            <input v-model="form.model" class="bw-input" placeholder="MODEL ID" list="agent-models" />
            <button class="bw-btn small" :disabled="modelBusy || !form.providerId" @click="refreshModels" title="REFRESH MODEL LIST FROM PROVIDER" aria-label="REFRESH MODEL LIST"><AppIcon name="solar:refresh-bold" :size="13" /></button>
          </span>
          <datalist id="agent-models">
            <option v-for="m in models" :key="m" :value="m" />
          </datalist>
        </label>
        <label class="w-label">KIND
          <select v-model="form.agentKind" class="bw-input">
            <option value="build">BUILD (FULL ACCESS)</option>
            <option value="plan">PLAN (READ-ONLY)</option>
          </select>
        </label>
      </div>
      <label class="w-label">ENDPOINT OVERRIDE (OPTIONAL)
        <input v-model="form.baseUrlOverride" class="bw-input" :placeholder="presetBase" />
      </label>
      <div v-if="isCustom" class="w-row">
        <label class="w-label">DIALECT
          <select v-model="form.dialectOverride" class="bw-input">
            <option value="openAi">OPENAI-COMPATIBLE</option>
            <option value="anthropic">ANTHROPIC</option>
          </select>
        </label>
        <label class="w-label">AUTH
          <select v-model="form.authSchemeOverride" class="bw-input">
            <option value="bearer">BEARER</option>
            <option value="header">HEADER</option>
            <option value="query">QUERY (?key=)</option>
            <option value="none">NONE</option>
          </select>
        </label>
        <label v-if="form.authSchemeOverride === 'header' || form.authSchemeOverride === 'query'" class="w-label">AUTH NAME
          <input v-model="form.authNameOverride" class="bw-input" placeholder="x-api-key" />
        </label>
      </div>
      <div class="w-row">
        <label class="w-label">WORKING DIR (VOLUME-RELATIVE, EMPTY = ROOT)
          <input v-model="form.workingDir" class="bw-input" placeholder="/" />
        </label>
        <label class="w-label">MAX TURNS
          <input v-model.number="form.maxTurns" class="bw-input" type="number" min="1" max="50" />
        </label>
      </div>
      <div class="w-row">
        <label class="w-label">PERMISSIONS
          <select v-model="permPreset" class="bw-input">
            <option value="strict">STRICT (ASK EVERYTHING)</option>
            <option value="balanced">BALANCED (READS AUTO, MUTATIONS ASK)</option>
            <option value="yolo">YOLO (AUTO, DENY NEVER BYPASSED)</option>
          </select>
        </label>
        <label class="w-check"><input type="checkbox" v-model="form.autoApprove" /> AUTO-APPROVE ASKS</label>
      </div>
      <label class="w-label">{{ wasmMode ? 'API KEY (MEMORY ONLY — CLEARED ON RELOAD)' : 'API KEY (SEALED server-side, NEVER SHOWN BACK)' }}
        <span class="model-row">
          <input v-model="keyInput" type="password" class="bw-input" placeholder="PASTE KEY" autocomplete="off" />
          <button class="bw-btn small" :disabled="!savedConfigId || !keyInput || keyBusy" @click="saveKey">SEAL KEY</button>
        </span>
      </label>
      <div class="w-actions">
        <button class="bw-btn small primary" :disabled="busy || !canSave" @click="saveConfig">SAVE CONFIG</button>
      </div>
      <div v-if="setupMsg" class="w-msg">{{ setupMsg }}</div>
      <p class="text-muted hint">CUSTOM PROVIDER: pick the CUSTOM preset, set endpoint + dialect + auth. MODEL LIST refresh needs a saved key (Anthropic has no list API — enter manually).</p>
    </div>

    <div class="section">
      <h3 class="section-title"><AppIcon name="solar:settings-minimalistic-bold" :size="13" /> AGENT CONFIGS ({{ configs.length }})</h3>
      <div class="config-list">
        <div v-for="cfg in configs" :key="cfg.id" class="config-card" :class="{ on: chatConfigId === cfg.id }" @click="chatConfigId = cfg.id">
          <div class="cfg-header">
            <span class="cfg-name">{{ cfg.name }}</span>
            <span class="cfg-type text-muted">{{ cfg.providerId }}/{{ cfg.model }}</span>
            <span class="cfg-status" :class="{ on: cfg.hasKey || isKeyless(cfg) }">{{ cfg.hasKey || isKeyless(cfg) ? 'READY' : 'NO KEY' }}</span>
            <span class="cfg-type text-muted">{{ cfg.agentKind.toUpperCase() }}</span>
          </div>
          <div class="cfg-actions">
            <button class="bw-btn xs danger" @click.stop="removeCfg(cfg.id)">DEL</button>
          </div>
        </div>
      </div>
      <div v-if="!configs.length" class="empty text-muted">No configs — open SETUP, pick a preset, save.</div>
    </div>

    <div v-if="chatConfig" class="section">
      <h3 class="section-title"><AppIcon name="solar:plug-circle-bold" :size="13" /> SERVERS FOR {{ chatConfig.name.toUpperCase() }} ({{ mcpEntries.length }})</h3>
      <div class="config-list">
        <div v-for="m in mcpEntries" :key="m.name" class="config-card">
          <div class="cfg-header">
            <span class="cfg-name">{{ m.name }}</span>
            <span class="cfg-type text-muted">{{ m.cfg.transport }}{{ m.cfg.enabled ? '' : ' · OFF' }}</span>
          </div>
          <div class="cfg-meta text-muted">
            <span v-if="m.cfg.transport === 'stdio'">{{ m.cfg.command }} {{ (m.cfg.args || []).join(' ') }}</span>
            <span v-else>{{ m.cfg.url }}</span>
          </div>
          <div class="cfg-actions">
            <button class="bw-btn xs danger" @click="removeMcp(m.name)">DETACH</button>
          </div>
        </div>
      </div>
      <div class="w-row">
        <input v-model="mcpForm.name" class="bw-input" placeholder="server-name" spellcheck="false" />
        <select v-model="mcpForm.transport" class="bw-input">
          <option value="stdio">STDIO (LOCAL CMD)</option>
          <option value="http">HTTP (STREAMABLE)</option>
        </select>
      </div>
      <div class="w-row">
        <input
          v-if="mcpForm.transport === 'stdio'"
          v-model="mcpForm.command"
          class="bw-input"
          placeholder="command on PATH (e.g. npx)"
          spellcheck="false"
        />
        <input
          v-if="mcpForm.transport === 'stdio'"
          v-model="mcpForm.args"
          class="bw-input"
          placeholder="args, space-separated"
          spellcheck="false"
        />
        <input
          v-if="mcpForm.transport === 'http'"
          v-model="mcpForm.url"
          class="bw-input"
          placeholder="https://…/mcp"
          spellcheck="false"
        />
        <button class="bw-btn small" :disabled="mcpBusy || !mcpForm.name.trim()" @click="addMcp">ATTACH</button>
        <button class="bw-btn small" :disabled="mcpBusy || !chatConfigId" @click="refreshMcpTools">LIST TOOLS</button>
      </div>
      <div v-if="mcpTools.length" class="remote-list">
        <div v-for="t in mcpTools" :key="t.name" class="remote-row">
          <span>{{ t.name }}</span><span class="text-muted">{{ (t.description || '').slice(0, 80) }}</span>
        </div>
      </div>
      <div v-if="mcpMsg" class="w-msg">{{ mcpMsg }}</div>
      <p class="text-muted hint">ATTACH/DETACH NEEDS ADMIN (STDIO SPAWNS PROCESSES). TOOLS APPEAR AS <span class="mono">mcp__server__tool</span> AND FOLLOW THE SAME ASK/DENY RULES.</p>
    </div>

    <div class="section">
      <h3 class="section-title"><AppIcon name="solar:chat-square-bold" :size="13" /> SESSIONS ({{ sessions.length }})</h3>
      <div class="w-row">
        <select v-model="chatConfigId" class="bw-input">
          <option value="">SELECT CONFIG</option>
          <option v-for="c in configs" :key="c.id" :value="c.id">{{ c.name }} ({{ c.model }})</option>
        </select>
        <button class="bw-btn small" :disabled="!chatConfigId" @click="newSession">+ NEW</button>
        <button class="bw-btn small" :disabled="!chatConfigId || jobActive" @click="initRepo" title="Analyze the repo and write AGENTS.md with a detached run">
          INIT REPO
        </button>
        <button class="bw-btn small" @click="importClick">IMPORT</button>
        <input ref="importEl" type="file" accept="application/json" hidden @change="importFile" />
      </div>
      <div class="session-list">
        <div
          v-for="s in sessions"
          :key="s.id"
          class="session-card"
          :class="{ on: viewing?.id === s.id }"
          @click="loadSession(s.id)"
        >
          <span class="session-title">{{ s.title }}</span>
          <span class="text-muted session-meta">{{ s.messages.length }} msgs</span>
          <button class="bw-btn xs" @click.stop="exportSession(s.id)" title="EXPORT SESSION" aria-label="EXPORT SESSION"><AppIcon name="solar:download-bold" :size="13" /></button>
          <button class="bw-btn xs danger" @click.stop="removeSession(s.id)" aria-label="CLOSE"><AppIcon name="solar:close-bold" :size="13" /></button>
        </div>
      </div>
    </div>

    <div v-if="viewing" class="section thread">
      <h3 class="section-title"><AppIcon name="solar:chat-square-bold" :size="13" /> {{ viewing.title }}</h3>
      <div class="w-actions thread-actions">
        <button class="bw-btn xs" :disabled="!viewing.messages.length || jobActive" @click="compactThread" title="Summarize into a fresh session (old kept)">
          COMPACT
        </button>
      </div>
      <div class="usage text-muted" v-if="threadUsage">TOKENS IN {{ threadUsage.inputTokens }} / OUT {{ threadUsage.outputTokens }}</div>
      <div class="messages">
        <div v-for="(m, i) in viewing.messages" :key="i" class="msg" :class="`role-${m.role}`">
          <div class="msg-role text-muted">{{ roleLabel(m) }}</div>
          <div v-if="m.content" class="msg-body">{{ m.content }}</div>
          <div v-if="m.toolName || m.toolInput" class="tool-block">
            <span class="tool-name"><AppIcon name="solar:toolbox-bold" :size="12" /> {{ m.toolName ?? toolNameOf(m) }}</span>
            <pre class="tool-input">{{ prettyInput(m) }}</pre>
          </div>
        </div>
        <div v-if="!viewing.messages.length" class="empty text-muted">No messages yet — ask below.</div>
      </div>

      <div v-if="pendingApproval" class="approval">
        <div class="approval-title"><AppIcon :name="pendingApproval.question ? 'solar:question-circle-bold' : 'solar:shield-check-bold'" :size="13" /> {{ pendingApproval.question ? 'NEEDS YOUR ANSWER' : 'AGENT WAITS' }}</div>
        <div class="approval-text">{{ pendingApproval.question || pendingApproval.summary }}</div>
        <div v-if="pendingApproval.question" class="w-row">
          <input v-model="answerInput" class="bw-input" placeholder="TYPE ANSWER…" @keyup.enter="answerApproval(true)" />
        </div>
        <div class="w-actions">
          <button class="bw-btn small primary" @click="answerApproval(true)">ALLOW</button>
          <button class="bw-btn small" @click="answerApproval(true, true)" title="Allow this tool for the rest of the config (stored as an explicit rule)">
            ALLOW ALWAYS
          </button>
          <button class="bw-btn small danger" @click="answerApproval(false)">DENY</button>
        </div>
      </div>

      <div class="prompt-row">
        <textarea
          v-model="promptInput"
          class="bw-input prompt-box"
          placeholder="ASK THE AGENT… (Ctrl+Enter to send)"
          rows="3"
          @keydown.ctrl.enter="sendPrompt"
          @keydown.meta.enter="sendPrompt"
        />
      </div>
      <div class="w-actions">
        <button class="bw-btn small primary" :disabled="!canSend" @click="sendPrompt">SEND</button>
        <button v-if="jobActive" class="bw-btn small danger" @click="abortJob">ABORT</button>
      </div>
      <div v-if="jobLine" class="w-msg">{{ jobLine }}</div>
      <div v-if="jobError" class="w-msg err" :title="jobHint">{{ jobError }}</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import AppIcon from '@/components/AppIcon.vue'
import { computed, reactive, ref, watch, onMounted } from 'vue'
import { useAppStore } from '@/stores/app'
import { isStaticHost } from '@/composables/useTauri'
import {
  useAgent,
  runLocalAgent,
  listLocalConfigs,
  saveLocalConfig,
  deleteLocalConfig,
  listLocalSessions,
  saveLocalSession,
  deleteLocalSession,
  abortLocalRun,
} from '@/composables/useAgent'
import { agentPermissionPreset, describeSyncError } from '@/types'
import type { AgentConfig, AgentJob, AgentSession, ProviderPreset } from '@/types'

const store = useAppStore()
const agent = useAgent()

const wasmMode = computed(() => {
  try {
    return isStaticHost()
  } catch {
    return false
  }
})
const transportLabel = computed(() => (wasmMode.value ? 'WASM LOCAL' : 'DESKTOP / REST'))

// ── data sources (server store vs browser-local) ──
const localPresets = ref<ProviderPreset[]>([])
const localConfigs = ref<AgentConfig[]>([])
const localSessions = ref<AgentSession[]>([])
const localViewing = ref<AgentSession | null>(null)
const localJob = ref<AgentJob | null>(null)
/** Provider keys live in memory only — never localStorage. */
const localKeys = ref<Record<string, string>>({})

const providers = computed(() => (wasmMode.value ? localPresets.value : store.agentProviders))
const configs = computed(() => (wasmMode.value ? localConfigs.value : store.agentConfigs))
const sessions = computed(() => (wasmMode.value ? localSessions.value : store.agentSessions))
const viewing = computed(() => (wasmMode.value ? localViewing.value : serverViewing.value))
const serverViewing = ref<AgentSession | null>(null)
const activeJob = computed(() => (wasmMode.value ? localJob.value : store.activeAgentJob))

const showSetup = ref(false)
const busy = ref(false)
const keyBusy = ref(false)
const modelBusy = ref(false)
const setupMsg = ref('')
const keyInput = ref('')
const savedConfigId = ref('')
const models = ref<string[]>([])
const permPreset = ref<'strict' | 'balanced' | 'yolo'>('balanced')

const form = reactive({
  providerId: 'openrouter',
  name: '',
  model: '',
  baseUrlOverride: '',
  dialectOverride: 'openAi' as 'openAi' | 'anthropic',
  authSchemeOverride: 'bearer' as 'bearer' | 'header' | 'query' | 'none',
  authNameOverride: '',
  workingDir: '',
  agentKind: 'build' as 'build' | 'plan',
  autoApprove: false,
  maxTurns: 25,
})

const chatConfigId = ref('')
const promptInput = ref('')
const answerInput = ref('')
const importEl = ref<HTMLInputElement | null>(null)

const chatConfig = computed(() => configs.value.find(c => c.id === chatConfigId.value) ?? null)

// ─── MCP attach/detach/discover (selected config) ───
const mcpTools = ref<Array<{ server: string; name: string; description: string }>>([])
const mcpBusy = ref(false)
const mcpMsg = ref('')
const mcpForm = reactive({ name: '', transport: 'stdio', command: '', args: '', url: '' })

const mcpEntries = computed(() => {
  const cfg = chatConfig.value
  if (!cfg) return []
  return Object.entries(cfg.mcpServers ?? {}).map(([name, server]) => ({ name, cfg: server }))
})

async function addMcp() {
  if (!chatConfigId.value || !mcpForm.name.trim()) return
  mcpBusy.value = true
  mcpMsg.value = ''
  try {
    const server = {
      transport: mcpForm.transport,
      command: mcpForm.transport === 'stdio' ? mcpForm.command.trim() || undefined : undefined,
      args: mcpForm.transport === 'stdio' ? mcpForm.args.split(/\s+/).filter(Boolean) : [],
      env: {},
      url: mcpForm.transport === 'http' ? mcpForm.url.trim() || undefined : undefined,
      headers: [],
      enabled: true,
    }
    const updated = await store.mcpAddServer(chatConfigId.value, mcpForm.name.trim(), server)
    if (updated) {
      mcpForm.name = ''
      mcpForm.command = ''
      mcpForm.args = ''
      mcpForm.url = ''
      mcpMsg.value = `Attached — ${updated.mcpServers ? Object.keys(updated.mcpServers).length : 0} server(s).`
    }
  } finally {
    mcpBusy.value = false
  }
}

async function removeMcp(name: string) {
  if (!chatConfigId.value) return
  mcpBusy.value = true
  try {
    await store.mcpRemoveServer(chatConfigId.value, name)
    mcpTools.value = mcpTools.value.filter(t => t.server !== name)
  } finally {
    mcpBusy.value = false
  }
}

async function refreshMcpTools() {
  if (!chatConfigId.value) return
  mcpBusy.value = true
  mcpMsg.value = ''
  try {
    const tools = await store.mcpListTools(chatConfigId.value)
    if (tools) {
      mcpTools.value = tools
      mcpMsg.value = tools.length ? `${tools.length} tools discovered.` : 'Connected — no tools exposed.'
    }
  } finally {
    mcpBusy.value = false
  }
}

async function compactThread() {
  if (!viewing.value || !viewing.value.messages.length || jobActive.value) return
  const compacted = await store.compactAgentSession(viewing.value.configId, viewing.value.id)
  if (compacted) setViewing(compacted)
}

/** Mode-aware viewer setter (server viewing lives in a ref, local in state). */
function setViewing(s: AgentSession | null) {
  if (wasmMode.value) localViewing.value = s
  else serverViewing.value = s
}

const preset = computed(() => providers.value.find(p => p.id === form.providerId) ?? null)
const presetBase = computed(() => preset.value?.baseUrl ?? 'https://…')
const isCustom = computed(() => form.providerId === 'custom')
const canSave = computed(() => form.name.trim() !== '' && form.model.trim() !== '' && form.providerId !== '')

// ─── browser-local mode (static host): same shapes, localStorage rows ──

function newLocalId(prefix: string): string {
  try {
    return `${prefix}-${crypto.randomUUID().slice(0, 8)}`
  } catch {
    return `${prefix}-${Date.now().toString(36)}`
  }
}

function localNow(): string {
  return new Date().toISOString()
}

function refreshLocal() {
  localConfigs.value = listLocalConfigs().map(c => ({
    ...c,
    hasKey: !!localKeys.value[c.id] || c.hasKey,
  }))
  localSessions.value = listLocalSessions()
}

function localPresetFor(config: AgentConfig): ProviderPreset | null {
  return (
    localPresets.value.find(p => p.id === config.providerId) ??
    providers.value.find(p => p.id === config.providerId) ??
    null
  )
}

function buildLocalHeaders(
  preset: ProviderPreset | null,
  key: string,
): Array<[string, string]> {
  const headers: Array<[string, string]> = [...(preset?.extraHeaders ?? [])]
  const auth = preset?.auth ?? 'bearer'
  if (auth === 'bearer' && key) headers.push(['Authorization', `Bearer ${key}`])
  else if (auth === 'header') headers.push([preset?.authName ?? 'x-api-key', key])
  return headers
}

function localChatUrl(baseUrl: string, dialect: string): string {
  const base = baseUrl.replace(/\/$/, '')
  return dialect === 'anthropic' ? `${base}/v1/messages` : `${base}/chat/completions`
}

function localSystemPrompt(config: AgentConfig): string {
  const root = config.workingDir ? `/${config.workingDir}` : '/'
  return (
    `You are Cybermanju, an AI coding agent running fully in the browser over a local file volume.\n` +
    `Working root: ${root}\n` +
    `Agent mode: ${config.agentKind} (plan = read-only, never edit).\n` +
    `SANDBOX: browser file volume — read/list/grep/glob/write/edit only. There is NO bash, ` +
    `NO subagents, NO MCP servers here; those tools answer unsupported:, so never call them.\n` +
    `TOOLS — paths: leading / = volume root, else working-dir-relative.\n` +
    `- read {path}: always read a file before editing it.\n` +
    `- list {path?}: one directory level; orient at / first.\n` +
    `- grep {pattern, path?, limit?}: regex over contents (invalid regex searches literally).\n` +
    `- glob {pattern, path?}: find files (* stays in one segment, ** crosses).\n` +
    `- edit {path, old_block, new_block}: replace ONE exact block; missing → not_found:, ` +
    `ambiguous → conflict:, then re-read and send a larger block.\n` +
    `- write {path, content}: full-file create/overwrite; prefer edit for small changes.\n` +
    `WORKFLOW: orient (list/glob) → read → act → verify. Small verified steps; ` +
    `never invent file contents. Denials are information — work around them, never ` +
    `retry identically. Report errors with their machine prefix. Answer concisely; ` +
    `lead with what changed (file:line).`
  )
}

const customPreset: ProviderPreset = {
  id: 'custom',
  label: 'Custom endpoint',
  family: 'custom',
  baseUrl: '',
  defaultModel: '',
  dialect: 'openAi',
  auth: 'bearer',
  authName: null,
  keyEnv: '',
  keyless: false,
  extraHeaders: [],
}
const allPresets = computed(() => [...providers.value, ...(providers.value.some(p => p.id === 'custom') ? [] : [customPreset])])

function pickPreset(p: ProviderPreset) {
  form.providerId = p.id
  if (!form.model || !form.name) {
    form.model = p.defaultModel
    if (!form.name) form.name = p.label
  } else {
    form.model = p.defaultModel
  }
  form.baseUrlOverride = ''
  models.value = []
  setupMsg.value = p.keyless ? 'Keyless provider — save, no key needed.' : `Needs ${p.keyEnv || 'an API key'} — save config, then SEAL KEY.`
}

function isKeyless(cfg: AgentConfig) {
  return providers.value.find(p => p.id === cfg.providerId)?.keyless ?? false
}

function localConfigFromForm(): AgentConfig {
  const now = localNow()
  return {
    id: savedConfigId.value && wasmMode.value ? savedConfigId.value : newLocalId('cfg'),
    name: form.name.trim(),
    providerId: form.providerId,
    model: form.model.trim(),
    baseUrlOverride: form.baseUrlOverride.trim() || undefined,
    dialectOverride: isCustom.value ? form.dialectOverride : undefined,
    authSchemeOverride: isCustom.value ? form.authSchemeOverride : undefined,
    authNameOverride: isCustom.value && form.authNameOverride.trim() ? form.authNameOverride.trim() : undefined,
    workingDir: form.workingDir.trim(),
    agentKind: form.agentKind,
    permission: agentPermissionPreset(permPreset.value),
    autoApprove: form.autoApprove,
    maxTurns: Math.min(50, Math.max(1, form.maxTurns || 25)),
    hasKey: false,
    createdAt: now,
    updatedAt: now,
  }
}

async function saveConfig() {
  busy.value = true
  setupMsg.value = ''
  try {
    if (wasmMode.value) {
      const cfg = localConfigFromForm()
      const prev = listLocalConfigs().find(c => c.id === cfg.id)
      saveLocalConfig({ ...cfg, createdAt: prev?.createdAt ?? cfg.createdAt })
      refreshLocal()
      savedConfigId.value = cfg.id
      chatConfigId.value = cfg.id
      setupMsg.value = 'Saved locally — paste the key below (kept in memory only).'
      return
    }
    const saved = await store.saveAgentConfig({
      name: form.name.trim(),
      providerId: form.providerId,
      model: form.model.trim(),
      baseUrlOverride: form.baseUrlOverride.trim() || undefined,
      dialectOverride: isCustom.value ? form.dialectOverride : undefined,
      authSchemeOverride: isCustom.value ? form.authSchemeOverride : undefined,
      authNameOverride: isCustom.value && form.authNameOverride.trim() ? form.authNameOverride.trim() : undefined,
      workingDir: form.workingDir.trim(),
      agentKind: form.agentKind,
      permission: agentPermissionPreset(permPreset.value),
      autoApprove: form.autoApprove,
      maxTurns: Math.min(50, Math.max(1, form.maxTurns || 25)),
    })
    if (saved) {
      savedConfigId.value = saved.id
      chatConfigId.value = saved.id
      setupMsg.value = isKeyless(saved)
        ? 'Saved — keyless provider, ready to chat.'
        : 'Saved — now SEAL KEY above, then chat.'
    }
  } finally {
    busy.value = false
  }
}

async function saveKey() {
  if (!savedConfigId.value || !keyInput.value) return
  if (wasmMode.value) {
    localKeys.value[savedConfigId.value] = keyInput.value
    keyInput.value = ''
    refreshLocal()
    setupMsg.value = 'Key held in memory for this page only — never stored.'
    return
  }
  keyBusy.value = true
  try {
    if (await store.saveAgentKey(savedConfigId.value, keyInput.value)) {
      keyInput.value = ''
      setupMsg.value = 'Key sealed — never shown back.'
    }
  } finally {
    keyBusy.value = false
  }
}

async function removeCfg(id: string) {
  if (wasmMode.value) {
    deleteLocalConfig(id)
    delete localKeys.value[id]
    refreshLocal()
  } else {
    await store.deleteAgentConfig(id)
  }
  if (chatConfigId.value === id) chatConfigId.value = ''
}

async function refreshModels() {
  if (wasmMode.value) {
    const cfg = localConfigs.value.find(c => c.id === (savedConfigId.value || chatConfigId.value))
    const preset = cfg ? localPresetFor(cfg) : null
    const base = (cfg?.baseUrlOverride || preset?.baseUrl || '').replace(/\/$/, '')
    if (!base) {
      setupMsg.value = 'Save the config first, then refresh.'
      return
    }
    if ((preset?.dialect ?? 'openAi') === 'anthropic') {
      setupMsg.value = 'Anthropic has no list API — enter the model id manually.'
      return
    }
    modelBusy.value = true
    try {
      const headers: Record<string, string> = {}
      const key = localKeys.value[cfg?.id ?? ''] ?? ''
      const auth = preset?.auth ?? 'bearer'
      if (auth === 'bearer' && key) headers['Authorization'] = `Bearer ${key}`
      else if (auth === 'header') headers[preset?.authName ?? 'x-api-key'] = key
      for (const [k, v] of preset?.extraHeaders ?? []) headers[k] = v
      const url = preset?.auth === 'query'
        ? `${base}/models?${encodeURIComponent(preset?.authName ?? 'key')}=${encodeURIComponent(key)}`
        : `${base}/models`
      const res = await fetch(url, { headers })
      if (!res.ok) throw new Error(`HTTP ${res.status}`)
      const body = (await res.json()) as { data?: Array<{ id?: string }> }
      const list = (body.data ?? []).map(m => m.id ?? '').filter(Boolean)
      models.value = [...new Set(list)].sort()
      setupMsg.value = `${models.value.length} models listed.`
    } catch (e) {
      setupMsg.value = `Refresh failed: ${e instanceof Error ? e.message : String(e)}`
    } finally {
      modelBusy.value = false
    }
    return
  }
  if (!savedConfigId.value) {
    setupMsg.value = 'Save the config first, then refresh.'
    return
  }
  modelBusy.value = true
  try {
    const list = await store.refreshAgentModels(savedConfigId.value)
    if (list) {
      models.value = list
      setupMsg.value = `${list.length} models listed.`
    }
  } finally {
    modelBusy.value = false
  }
}

async function newSession() {
  if (!chatConfigId.value) return
  if (wasmMode.value) {
    const cfg = localConfigs.value.find(c => c.id === chatConfigId.value)
    if (!cfg) return
    const now = localNow()
    const s: AgentSession = {
      id: newLocalId('ses'),
      title: 'Untitled session',
      configId: cfg.id,
      providerId: cfg.providerId,
      model: cfg.model,
      agentKind: cfg.agentKind,
      workingDir: cfg.workingDir,
      messages: [],
      usage: { inputTokens: 0, outputTokens: 0 },
      createdAt: now,
      updatedAt: now,
    }
    saveLocalSession(s)
    refreshLocal()
    setViewing(s)
    return
  }
  const s = await (async () => {
    try {
      const { invoke } = await import('@/composables/useTauri')
      const created = await invoke<AgentSession>('create_agent_session', { configId: chatConfigId.value })
      await store.fetchAgentSessions()
      return created
    } catch (e) {
      store.notifyError('Failed to create session', e)
      return null
    }
  })()
  if (s) setViewing(s)
}

async function loadSession(id: string) {
  if (wasmMode.value) {
    setViewing(listLocalSessions().find(s => s.id === id) ?? null)
    return
  }
  setViewing(await store.loadAgentSession(id))
}

async function removeSession(id: string) {
  if (wasmMode.value) {
    deleteLocalSession(id)
    refreshLocal()
  } else {
    await store.deleteAgentSession(id)
  }
  if (viewing.value?.id === id) setViewing(null)
}

async function exportSession(id: string) {
  const s = wasmMode.value
    ? listLocalSessions().find(s => s.id === id) ?? null
    : await store.loadAgentSession(id)
  if (!s) return
  const blob = new Blob([JSON.stringify(s, null, 2)], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `agent-session-${id}.json`
  a.click()
  URL.revokeObjectURL(url)
}

function importClick() {
  importEl.value?.click()
}

async function importFile(e: Event) {
  const file = (e.target as HTMLInputElement).files?.[0]
  if (!file) return
  try {
    const text = await file.text()
    const parsed = JSON.parse(text) as AgentSession
    if (!Array.isArray(parsed.messages)) throw new Error('not an agent session transcript')
    if (wasmMode.value) {
      const now = localNow()
      saveLocalSession({
        ...parsed,
        id: newLocalId('ses'),
        createdAt: parsed.createdAt || now,
        updatedAt: now,
      })
      refreshLocal()
    } else {
      const { invoke } = await import('@/composables/useTauri')
      await invoke('import_agent_session', { session: parsed })
      await store.fetchAgentSessions()
    }
  } catch (err) {
    store.notifyError('Import failed', err)
  } finally {
    (e.target as HTMLInputElement).value = ''
  }
}

const pendingApproval = computed(() => {
  if (wasmMode.value) {
    const p = agent.pendingApproval.value
    if (!p) return null
    return { tool: p.tool, input: p.input, summary: p.summary, question: p.question ?? null }
  }
  const job = activeJob.value
  if (!job || job.status !== 'waiting_approval' || !job.pending) return null
  if (viewing.value && job.sessionId !== viewing.value.id) return null
  return job.pending
})

const jobActive = computed(() => {
  if (wasmMode.value) return agent.running.value
  const s = activeJob.value?.status
  return s === 'running' || s === 'waiting_approval'
})

const jobLine = computed(() => {
  if (wasmMode.value) {
    if (!localJob.value) return ''
    return `LOCAL JOB · ${localJob.value.status.toUpperCase()} · TURN ${localJob.value.turnsUsed}/${localJob.value.maxTurns}`
  }
  const job = activeJob.value
  if (!job) return ''
  return `JOB ${job.jobId.slice(0, 18)}… · ${job.status.toUpperCase()} · TURN ${job.turnsUsed}/${job.maxTurns}`
})

const jobError = computed(() => {
  if (wasmMode.value) return localJob.value?.error ?? ''
  return activeJob.value?.error ?? ''
})
const jobHint = computed(() => {
  if (!jobError.value) return ''
  const d = describeSyncError(jobError.value)
  return `${d.prefix}: ${d.hint}`
})

const threadUsage = computed(() => {
  const v = viewing.value
  if (!v || (v.usage.inputTokens === 0 && v.usage.outputTokens === 0)) return null
  return v.usage
})

const canSend = computed(() => promptInput.value.trim() !== '' && chatConfigId.value !== '' && !jobActive.value)

async function sendPrompt() {
  if (!canSend.value) return
  if (wasmMode.value) {
    await sendPromptLocal()
    return
  }
  const prompt = promptInput.value.trim()
  promptInput.value = ''
  // Ensure a session exists for this config; the job returns transcript via polling.
  let sessionId = viewing.value && viewing.value.configId === chatConfigId.value ? viewing.value.id : undefined
  if (!sessionId) {
    const { invoke } = await import('@/composables/useTauri')
    try {
      const created = await invoke<AgentSession>('create_agent_session', { configId: chatConfigId.value })
      await store.fetchAgentSessions()
      setViewing(created)
      sessionId = created.id
    } catch (e) {
      store.notifyError('Failed to create session', e)
      return
    }
  }
  await store.startAgentRun(chatConfigId.value, prompt, sessionId)
}

/** Browser-local run: same thread UI, loop in useAgent, volume tools. */
async function sendPromptLocal() {
  const prompt = promptInput.value.trim()
  if (!prompt || !chatConfigId.value) return
  const cfg = localConfigs.value.find(c => c.id === chatConfigId.value)
  if (!cfg) {
    store.notifyError('No local config selected', 'save one in SETUP first')
    return
  }
  const preset = localPresetFor(cfg)
  const base = (cfg.baseUrlOverride || preset?.baseUrl || '').replace(/\/$/, '')
  if (!base) {
    store.notifyError('No endpoint', 'set an endpoint override or pick a preset with one')
    return
  }
  const key = localKeys.value[cfg.id] ?? ''
  if (!key && !(preset?.keyless ?? false)) {
    store.notifyError('No API key', 'paste the key in SETUP (kept in memory only)')
    return
  }
  const dialect = (cfg.dialectOverride ?? preset?.dialect ?? 'openAi') as 'openAi' | 'anthropic'
  const auth = (cfg.authSchemeOverride ?? preset?.auth ?? 'bearer') as 'bearer' | 'header' | 'query' | 'none'
  const headers: Array<[string, string]> = [...(preset?.extraHeaders ?? [])]
  if (auth === 'bearer' && key) headers.push(['Authorization', `Bearer ${key}`])
  else if (auth === 'header') headers.push([cfg.authNameOverride || preset?.authName || 'x-api-key', key])
  const url =
    auth === 'query'
      ? `${base}${dialect === 'anthropic' ? '/v1/messages' : '/chat/completions'}?${encodeURIComponent(cfg.authNameOverride || preset?.authName || 'key')}=${encodeURIComponent(key)}`
      : dialect === 'anthropic'
        ? `${base}/v1/messages`
        : `${base}/chat/completions`

  let session = viewing.value && viewing.value.configId === cfg.id ? viewing.value : null
  if (!session) {
    const now = localNow()
    session = {
      id: newLocalId('ses'),
      title: prompt.split(/\s+/).slice(0, 8).join(' ') || 'Untitled session',
      configId: cfg.id,
      providerId: cfg.providerId,
      model: cfg.model,
      agentKind: cfg.agentKind,
      workingDir: cfg.workingDir,
      messages: [],
      usage: { inputTokens: 0, outputTokens: 0 },
      createdAt: now,
      updatedAt: now,
    }
    saveLocalSession(session)
    refreshLocal()
    setViewing(session)
  }
  promptInput.value = ''
  session.messages.push({ role: 'user', content: prompt })
  session.updatedAt = localNow()
  saveLocalSession(session)
  refreshLocal()
  setViewing({ ...session })

  localJob.value = {
    jobId: newLocalId('job'),
    sessionId: session.id,
    configId: cfg.id,
    status: 'running',
    turnsUsed: 0,
    maxTurns: cfg.maxTurns,
    usage: { ...session.usage },
  }
  const outcome = await runLocalAgent(
    {
      baseUrl: base,
      dialect,
      model: cfg.model,
      headers,
      system: localSystemPrompt(cfg),
      maxTurns: cfg.maxTurns,
      permission: cfg.permission,
      autoApprove: cfg.autoApprove,
      agentKind: cfg.agentKind,
    },
    session.messages,
    session.usage,
    () => {
      session!.updatedAt = localNow()
      saveLocalSession(session!)
      if (localJob.value) {
        localJob.value = {
          ...localJob.value,
          status: 'running',
          usage: { ...session!.usage },
        }
      }
    },
  )
  const finished: AgentJob = {
    ...(localJob.value ?? {
      jobId: newLocalId('job'),
      sessionId: session.id,
      configId: cfg.id,
      maxTurns: cfg.maxTurns,
      usage: { ...session.usage },
    }),
    status: outcome.stopped === 'done' || outcome.stopped === 'limit' ? 'done' : outcome.stopped === 'aborted' ? 'cancelled' : 'error',
    usage: { ...session.usage },
    result: outcome.stopped === 'limit' ? 'turn budget exhausted — transcript saved' : undefined,
    error: outcome.error,
  }
  session.updatedAt = localNow()
  saveLocalSession(session)
  refreshLocal()
  setViewing({ ...session })
  localJob.value = finished
}

async function abortJob() {
  if (wasmMode.value) {
    abortLocalRun()
    if (localJob.value) localJob.value = { ...localJob.value, status: 'cancelled' }
    return
  }
  const job = activeJob.value
  if (job) await store.abortAgentJob(job.jobId)
}

async function answerApproval(approved: boolean, remember = false) {
  if (wasmMode.value) {
    const pending = agent.pendingApproval.value
    agent.pendingApproval.value = null
    pending?.resolve(approved, approved ? answerInput.value || undefined : undefined)
    answerInput.value = ''
    return
  }
  const job = activeJob.value
  if (!job) return
  await store.approveAgentJob(job.jobId, approved, approved ? answerInput.value || undefined : undefined, remember)
  answerInput.value = ''
  if (remember && approved) void store.fetchAgentConfigs()
}

async function initRepo() {
  if (!chatConfigId.value || jobActive.value) return
  if (wasmMode.value) {
    store.notifyError('Repo-init needs a worker', 'use a desktop/Docker config — the browser loop cannot run detached jobs')
    return
  }
  await store.initAgentRun(chatConfigId.value)
}

function roleLabel(m: { role: string; toolName?: string | null }) {
  if (m.role === 'user') return 'YOU'
  if (m.role === 'tool') return `TOOL: ${m.toolName ?? ''}`
  if (m.role === 'assistant_tool') return 'AGENT TOOL'
  return 'AGENT'
}

function toolNameOf(m: { toolInput?: unknown }) {
  try {
    const arr = (m.toolInput ?? []) as Array<{ name?: string }>
    return arr.map(c => c.name ?? '?').join(', ')
  } catch {
    return ''
  }
}

function prettyInput(m: { toolName?: string | null; toolInput?: unknown }) {
  try {
    if (m.toolName) return JSON.stringify(m.toolInput ?? {}, null, 1).slice(0, 800)
    const arr = (m.toolInput ?? []) as Array<{ name?: string; input?: unknown }>
    return arr.map(c => `${c.name ?? '?'} ${JSON.stringify(c.input ?? {}).slice(0, 300)}`).join('\n')
  } catch {
    return ''
  }
}

watch(
  () => activeJob.value?.status,
  (status) => {
    if ((status === 'done' || status === 'error' || status === 'cancelled') && viewing.value) {
      void store.loadAgentSession(viewing.value.id).then(s => {
        if (s) setViewing(s)
      })
      void store.fetchAgentSessions()
    }
  },
)

onMounted(async () => {
  if (wasmMode.value) {
    try {
      const presets = (await agent.wasmAgentCatalog()) as ProviderPreset[]
      if (presets.length) localPresets.value = presets
    } catch {
      localPresets.value = []
    }
    refreshLocal()
    if (!form.model) {
      const medium = localPresets.value.find(p => p.id === 'openrouter')
      if (medium) pickPreset(medium)
    }
    return
  }
  await Promise.allSettled([store.fetchAgentProviders(), store.fetchAgentConfigs(), store.fetchAgentSessions()])
  if (!form.model) {
    const medium = providers.value.find(p => p.id === 'openrouter')
    if (medium) pickPreset(medium)
  }
})
</script>

<style scoped>
.agent-panel {
  width: 100%;
  height: 100%;
  background: #000;
  overflow-y: auto;
  padding: 16px;
  font-family: 'Courier New', monospace;
  color: #FFFFFF;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-bottom: 10px;
  border-bottom: 2px solid #FFFFFF;
  margin-bottom: 16px;
}

.header-left { display: flex; align-items: center; gap: 8px; }
.icon-agent { font-size: 16px; }
.panel-title { font-size: 14px; font-weight: 800; letter-spacing: 1px; margin: 0; }
.transport-tag { font-size: 9px; }

.section { margin-bottom: 16px; }

.section-title {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 1px;
  color: rgba(255,255,255,0.6);
  margin: 0 0 8px;
}

.wizard { border: 2px dashed #FFFFFF; padding: 10px; }
.preset-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 6px;
  margin-bottom: 10px;
}
.preset-card {
  background: #000;
  border: 2px solid rgba(255,255,255,0.4);
  color: #FFF;
  padding: 6px 8px;
  text-align: left;
  cursor: pointer;
  font-family: inherit;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.preset-card.on { border-color: #FFF; background: rgba(255,255,255,0.08); }
.preset-name { font-size: 11px; font-weight: 700; }
.preset-meta { font-size: 9px; }
.preset-free { font-size: 8px; color: #8f8; }

.w-label { display: flex; flex-direction: column; gap: 4px; font-size: 10px; margin-bottom: 8px; flex: 1; }
.bw-input { background: #000; color: #FFF; border: 1px solid #FFF; padding: 6px 8px; font-size: 11px; font-family: inherit; }
.w-row { display: flex; gap: 8px; margin-bottom: 8px; flex-wrap: wrap; align-items: flex-end; }
.w-row .w-label { min-width: 140px; }
.w-check { font-size: 10px; display: flex; gap: 4px; align-items: center; padding-bottom: 8px; }
.w-actions { display: flex; gap: 8px; flex-wrap: wrap; }
.w-msg { font-size: 10px; margin-top: 6px; color: rgba(255,255,255,0.75); }
.w-msg.err { color: #F99; }
.hint { font-size: 9px; }
.model-row { display: flex; gap: 6px; }
.model-row .bw-input { flex: 1; }

.bw-btn { background: #000; color: #FFF; border: 2px solid #FFF; padding: 6px 10px; font-size: 10px; font-weight: 700; cursor: pointer; font-family: inherit; }
.bw-btn.small { font-size: 10px; }
.bw-btn.xs { font-size: 9px; padding: 3px 6px; border-width: 1px; }
.bw-btn.primary { background: #FFF; color: #000; }
.bw-btn.danger { border-color: #F66; color: #F66; }
.bw-btn:disabled { opacity: 0.4; cursor: default; }

.config-list, .session-list { display: flex; flex-direction: column; gap: 6px; }
.config-card, .session-card { border: 2px solid #FFFFFF; padding: 8px 10px; cursor: pointer; }
.session-card { display: flex; align-items: center; gap: 8px; }
.config-card.on, .session-card.on { background: rgba(255,255,255,0.08); }
.cfg-header { display: flex; align-items: center; gap: 8px; margin-bottom: 4px; }
.cfg-name { font-size: 12px; font-weight: 700; flex: 1; }
.cfg-type { font-size: 9px; }
.cfg-status { font-size: 9px; font-weight: 700; border: 1px solid #FFFFFF; padding: 0 4px; }
.cfg-status.on { background: #FFFFFF; color: #000; }
.cfg-actions { display: flex; gap: 6px; margin-top: 6px; }
.session-title { font-size: 12px; flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.session-meta { font-size: 9px; }
.empty { font-size: 10px; }

.thread { border-top: 2px solid #FFFFFF; padding-top: 12px; }
.thread-actions { margin-bottom: 8px; }
.mono { font-family: inherit; border: 1px solid rgba(255,255,255,0.4); padding: 0 4px; }
.remote-list { margin-top: 6px; }
.remote-row { display: flex; justify-content: space-between; gap: 8px; font-size: 10px; border-bottom: 1px solid #222; padding: 2px 0; }
.usage { font-size: 9px; margin-bottom: 6px; }
.messages { display: flex; flex-direction: column; gap: 8px; margin-bottom: 10px; max-height: 420px; overflow-y: auto; }
.msg { border: 1px solid rgba(255,255,255,0.25); padding: 6px 8px; }
.msg-role { font-size: 9px; font-weight: 700; margin-bottom: 4px; }
.msg-body { font-size: 11px; white-space: pre-wrap; word-break: break-word; }
.role-user { border-color: rgba(255,255,255,0.6); }
.role-assistant_tool { border-style: dashed; }
.role-tool { background: rgba(255,255,255,0.04); }
.tool-block { margin-top: 4px; }
.tool-name { font-size: 10px; font-weight: 700; }
.tool-input { font-size: 9px; color: rgba(255,255,255,0.7); white-space: pre-wrap; margin: 4px 0 0; }

.approval { border: 2px solid #FFB800; padding: 8px; margin-bottom: 10px; }
.approval-title { font-size: 10px; font-weight: 700; color: #FFB800; margin-bottom: 4px; }
.approval-text { font-size: 11px; margin-bottom: 8px; word-break: break-word; }

.prompt-box { width: 100%; min-height: 64px; resize: vertical; margin-bottom: 8px; }
.text-muted { color: rgba(255,255,255,0.5) !important; }
</style>
