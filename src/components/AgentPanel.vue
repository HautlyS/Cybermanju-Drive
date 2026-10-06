<template>
  <div class="agent-panel">
    <div class="panel-header">
      <div class="header-left">
        <span class="icon-agent"><AppIcon name="solar:bot-bold" /></span>
        <h2 class="panel-title">AI AGENT</h2>
        <span class="text-muted transport-tag">{{ transportLabel }}</span>
      </div>
      <UiButton size="sm" :icon="showSetup ? 'solar:close-bold' : 'solar:settings-bold'" @click="showSetup = !showSetup">
        {{ showSetup ? 'HIDE SETUP' : 'SETUP' }}
      </UiButton>
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

      <div class="w-field">
        <span class="w-label text-muted">CONFIG NAME</span>
        <UiInput v-model="form.name" placeholder="MY AGENT" aria-label="Config name" />
      </div>
      <div class="w-row">
        <div class="w-field">
          <span class="w-label text-muted">MODEL</span>
          <div class="model-row">
            <UiInput
              v-model="form.model"
              placeholder="MODEL ID"
              list="agent-models"
              aria-label="Model id"
            />
            <UiButton
              size="sm"
              icon="solar:refresh-bold"
              icon-only
              :disabled="modelBusy || !form.providerId"
              :loading="modelBusy"
              title="REFRESH MODEL LIST FROM PROVIDER"
              aria-label="REFRESH MODEL LIST"
              @click="refreshModels"
            />
          </div>
          <datalist id="agent-models">
            <option v-for="m in models" :key="m" :value="m" />
          </datalist>
        </div>
        <div class="w-field">
          <span class="w-label text-muted">KIND</span>
          <UiSelect
            :model-value="form.agentKind"
            :options="[
              { label: 'BUILD (FULL ACCESS)', value: 'build' },
              { label: 'PLAN (READ-ONLY)', value: 'plan' },
            ]"
            @update:model-value="form.agentKind = $event as 'build' | 'plan'"
          />
        </div>
      </div>
      <div class="w-field">
        <span class="w-label text-muted">ENDPOINT OVERRIDE (OPTIONAL)</span>
        <UiInput v-model="form.baseUrlOverride" :placeholder="presetBase" aria-label="Endpoint override" />
      </div>
      <div v-if="isCustom" class="w-row">
        <div class="w-field">
          <span class="w-label text-muted">DIALECT</span>
          <UiSelect
            :model-value="form.dialectOverride"
            :options="[
              { label: 'OPENAI-COMPATIBLE', value: 'openAi' },
              { label: 'ANTHROPIC', value: 'anthropic' },
            ]"
            @update:model-value="form.dialectOverride = $event as 'openAi' | 'anthropic'"
          />
        </div>
        <div class="w-field">
          <span class="w-label text-muted">AUTH</span>
          <UiSelect
            :model-value="form.authSchemeOverride"
            :options="[
              { label: 'BEARER', value: 'bearer' },
              { label: 'HEADER', value: 'header' },
              { label: 'QUERY (?key=)', value: 'query' },
              { label: 'NONE', value: 'none' },
            ]"
            @update:model-value="form.authSchemeOverride = $event as 'bearer' | 'header' | 'query' | 'none'"
          />
        </div>
        <div v-if="form.authSchemeOverride === 'header' || form.authSchemeOverride === 'query'" class="w-field">
          <span class="w-label text-muted">AUTH NAME</span>
          <UiInput v-model="form.authNameOverride" placeholder="x-api-key" aria-label="Auth header name" />
        </div>
      </div>
      <div class="w-row">
        <div class="w-field">
          <span class="w-label text-muted">WORKING DIR (VOLUME-RELATIVE, EMPTY = ROOT)</span>
          <UiInput v-model="form.workingDir" placeholder="/" aria-label="Working directory" />
        </div>
        <div class="w-field">
          <span class="w-label text-muted">MAX TURNS</span>
          <UiInput
            :model-value="String(form.maxTurns)"
            type="number"
            :min="1"
            :max="50"
            aria-label="Max turns"
            @update:model-value="form.maxTurns = Number($event)"
          />
        </div>
      </div>
      <div class="w-row">
        <div class="w-field">
          <span class="w-label text-muted">PERMISSIONS</span>
          <UiSelect
            :model-value="permPreset"
            :options="[
              { label: 'STRICT (ASK EVERYTHING)', value: 'strict' },
              { label: 'BALANCED (READS AUTO, MUTATIONS ASK)', value: 'balanced' },
              { label: 'YOLO (AUTO, DENY NEVER BYPASSED)', value: 'yolo' },
            ]"
            @update:model-value="permPreset = $event as 'strict' | 'balanced' | 'yolo'"
          />
        </div>
        <div class="w-field check-field">
          <UiCheckbox v-model="form.autoApprove" label="AUTO-APPROVE ASKS" />
        </div>
      </div>
      <div class="w-field">
        <span class="w-label text-muted">{{ wasmMode ? 'API KEY (MEMORY ONLY — CLEARED ON RELOAD)' : 'API KEY (SEALED server-side, NEVER SHOWN BACK)' }}</span>
        <div class="model-row">
          <UiInput
            v-model="keyInput"
            type="password"
            placeholder="PASTE KEY"
            aria-label="API key"
            autocomplete="off"
          />
          <UiButton
            size="sm"
            :disabled="!savedConfigId || !keyInput || keyBusy"
            :loading="keyBusy"
            @click="saveKey"
          >SEAL KEY</UiButton>
        </div>
      </div>
      <div class="w-actions">
        <UiButton size="sm" variant="primary" :disabled="busy || !canSave" :loading="busy" @click="saveConfig">SAVE CONFIG</UiButton>
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
            <UiBadge :tone="cfg.hasKey || isKeyless(cfg) ? 'accent' : 'neutral'" size="sm">
              {{ cfg.hasKey || isKeyless(cfg) ? 'READY' : 'NO KEY' }}
            </UiBadge>
            <span class="cfg-type text-muted">{{ cfg.agentKind.toUpperCase() }}</span>
          </div>
          <div class="cfg-actions">
            <UiButton size="xs" variant="danger" @click.stop="removeCfg(cfg.id)">DEL</UiButton>
          </div>
        </div>
      </div>
      <UiEmpty
        v-if="!configs.length"
        size="sm"
        icon="solar:bot-bold"
        title="No agent configs"
        description="Open SETUP, pick a preset, save a config."
      />
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
            <UiButton size="xs" variant="danger" @click="removeMcp(m.name)">DETACH</UiButton>
          </div>
        </div>
      </div>
      <div class="w-row">
        <div class="w-field grow">
          <UiInput v-model="mcpForm.name" placeholder="server-name" aria-label="MCP server name" spellcheck="false" />
        </div>
        <div class="w-field grow">
          <UiSelect
            :model-value="mcpForm.transport"
            :options="[
              { label: 'STDIO (LOCAL CMD)', value: 'stdio' },
              { label: 'HTTP (STREAMABLE)', value: 'http' },
            ]"
            @update:model-value="mcpForm.transport = $event"
          />
        </div>
      </div>
      <div class="w-row">
        <div v-if="mcpForm.transport === 'stdio'" class="w-field grow">
          <UiInput v-model="mcpForm.command" placeholder="command on PATH (e.g. npx)" aria-label="MCP command" spellcheck="false" />
        </div>
        <div v-if="mcpForm.transport === 'stdio'" class="w-field grow">
          <UiInput v-model="mcpForm.args" placeholder="args, space-separated" aria-label="MCP args" spellcheck="false" />
        </div>
        <div v-if="mcpForm.transport === 'http'" class="w-field grow">
          <UiInput v-model="mcpForm.url" placeholder="https://…/mcp" aria-label="MCP URL" spellcheck="false" />
        </div>
        <UiButton size="sm" :disabled="mcpBusy || !mcpForm.name.trim()" :loading="mcpBusy" @click="addMcp">ATTACH</UiButton>
        <UiButton size="sm" :disabled="mcpBusy || !chatConfigId" @click="refreshMcpTools">LIST TOOLS</UiButton>
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
        <div class="w-field grow">
          <UiSelect
            :model-value="chatConfigId"
            :options="sessionConfigOptions"
            @update:model-value="chatConfigId = $event"
          />
        </div>
        <UiButton size="sm" icon="solar:add-bold" :disabled="!chatConfigId" @click="newSession">NEW</UiButton>
        <UiButton
          size="sm"
          :disabled="!chatConfigId || jobActive"
          title="Analyze the repo and write AGENTS.md with a detached run"
          @click="initRepo"
        >INIT REPO</UiButton>
        <UiButton size="sm" icon="solar:download-bold" @click="importClick">IMPORT</UiButton>
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
          <UiButton
            size="xs"
            icon="solar:download-bold"
            icon-only
            title="EXPORT SESSION"
            aria-label="EXPORT SESSION"
            @click.stop="exportSession(s.id)"
          />
          <UiButton
            size="xs"
            variant="danger"
            icon="solar:close-bold"
            icon-only
            title="CLOSE SESSION"
            aria-label="CLOSE"
            @click.stop="removeSession(s.id)"
          />
        </div>
      </div>
    </div>

    <div v-if="viewing" class="section thread">
      <h3 class="section-title"><AppIcon name="solar:chat-square-bold" :size="13" /> {{ viewing.title }}</h3>
      <div class="w-actions thread-actions">
        <UiButton
          size="xs"
          :disabled="!viewing.messages.length || jobActive"
          title="Summarize into a fresh session (old kept)"
          @click="compactThread"
        >COMPACT</UiButton>
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
          <div class="w-field grow">
            <UiInput
              v-model="answerInput"
              placeholder="TYPE ANSWER…"
              aria-label="Approval answer"
              @enter="answerApproval(true)"
            />
          </div>
        </div>
        <div class="w-actions">
          <UiButton size="sm" variant="primary" @click="answerApproval(true)">ALLOW</UiButton>
          <UiButton
            size="sm"
            title="Allow this tool for the rest of the config (stored as an explicit rule)"
            @click="answerApproval(true, true)"
          >ALLOW ALWAYS</UiButton>
          <UiButton size="sm" variant="danger" @click="answerApproval(false)">DENY</UiButton>
        </div>
      </div>

      <div class="prompt-row">
        <textarea
          v-model="promptInput"
          class="prompt-box"
          placeholder="ASK THE AGENT… (Ctrl+Enter to send)"
          rows="3"
          @keydown.ctrl.enter="sendPrompt"
          @keydown.meta.enter="sendPrompt"
        />
      </div>
      <div class="w-actions">
        <UiButton size="sm" variant="primary" :disabled="!canSend" @click="sendPrompt">SEND</UiButton>
        <UiButton v-if="jobActive" size="sm" variant="danger" @click="abortJob">ABORT</UiButton>
      </div>
      <div v-if="jobLine" class="w-msg">{{ jobLine }}</div>
      <div v-if="jobError" class="w-msg err" :title="jobHint">{{ jobError }}</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import AppIcon from '@/components/AppIcon.vue'
import UiBadge from '@/components/ui/UiBadge.vue'
import UiButton from '@/components/ui/UiButton.vue'
import UiCheckbox from '@/components/ui/UiCheckbox.vue'
import UiEmpty from '@/components/ui/UiEmpty.vue'
import UiInput from '@/components/ui/UiInput.vue'
import UiSelect from '@/components/ui/UiSelect.vue'
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

const sessionConfigOptions = computed(() => [
  { label: 'SELECT CONFIG', value: '' },
  ...configs.value.map(c => ({ label: `${c.name} (${c.model})`, value: c.id })),
])

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
  background: var(--ui-surface);
  overflow-y: auto;
  padding: 16px;
  font-family: var(--ui-font);
  color: var(--ui-text);
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-bottom: 10px;
  border-bottom: 1px solid var(--ui-border);
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
  color: color-mix(in srgb, var(--ui-text) 60%, transparent);
  margin: 0 0 8px;
}

.wizard {
  border: 1px dashed color-mix(in srgb, var(--ui-border-strong) 80%, transparent);
  border-radius: var(--ui-radius-md);
  padding: 12px;
  background: color-mix(in srgb, var(--ui-glass) 70%, transparent);
  backdrop-filter: blur(calc(var(--ui-blur) * 0.6));
  -webkit-backdrop-filter: blur(calc(var(--ui-blur) * 0.6));
}

.preset-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 6px;
  margin-bottom: 10px;
}

.preset-card {
  background: color-mix(in srgb, var(--ui-surface) 55%, transparent);
  border: 1px solid var(--ui-hairline);
  border-radius: var(--ui-radius-sm);
  color: var(--ui-text);
  padding: 6px 8px;
  text-align: left;
  cursor: pointer;
  font-family: inherit;
  display: flex;
  flex-direction: column;
  gap: 2px;
  transition:
    border-color var(--ui-dur-fast) var(--ui-ease-out),
    background-color var(--ui-dur-fast) var(--ui-ease-out),
    box-shadow var(--ui-dur) var(--ui-ease-out);
}

.preset-card:hover {
  border-color: var(--ui-border-hover);
  background: var(--ui-glass-2);
}

.preset-card:focus-visible {
  outline: 2px solid color-mix(in srgb, var(--ui-accent) 75%, transparent);
  outline-offset: 2px;
}

.preset-card.on {
  border-color: color-mix(in srgb, var(--ui-accent) 55%, transparent);
  background: var(--ui-accent-softer);
  box-shadow: var(--ui-glow-soft);
}

.preset-name { font-size: 11px; font-weight: 700; }
.preset-meta { font-size: 9px; }
.preset-free { font-size: 8px; color: var(--ui-accent); }

.w-field { display: flex; flex-direction: column; gap: 4px; margin-bottom: 8px; min-width: 0; flex: 1; }
.w-field.grow { flex: 1; min-width: 140px; }
.w-field.check-field { justify-content: flex-end; padding-bottom: 6px; flex: 0 0 auto; }
.w-label { font-size: 10px; font-weight: 700; letter-spacing: 0.06em; }
.w-row { display: flex; gap: 8px; margin-bottom: 8px; flex-wrap: wrap; align-items: flex-end; }
.w-actions { display: flex; gap: 8px; flex-wrap: wrap; }
.w-msg { font-size: 10px; margin-top: 6px; color: color-mix(in srgb, var(--ui-text) 70%, transparent); }
.w-msg.err { color: var(--ui-danger); }
.hint { font-size: 9px; }
.model-row { display: flex; gap: 6px; }
.model-row .ui-field { flex: 1; }

.config-list, .session-list { display: flex; flex-direction: column; gap: 6px; }
.config-card, .session-card {
  border: 1px solid var(--ui-border);
  border-radius: var(--ui-radius-md);
  padding: 8px 10px;
  cursor: pointer;
  background: color-mix(in srgb, var(--ui-glass) 55%, transparent);
  transition: border-color var(--ui-dur-fast) var(--ui-ease-out), background-color var(--ui-dur-fast) var(--ui-ease-out);
}
.config-card:hover, .session-card:hover { border-color: var(--ui-border-hover); }
.session-card { display: flex; align-items: center; gap: 8px; }
.config-card.on, .session-card.on {
  border-color: color-mix(in srgb, var(--ui-accent) 45%, transparent);
  background: var(--ui-accent-softer);
}
.cfg-header { display: flex; align-items: center; gap: 8px; margin-bottom: 4px; flex-wrap: wrap; }
.cfg-name { font-size: 12px; font-weight: 700; flex: 1; }
.cfg-type { font-size: 9px; }
.cfg-actions { display: flex; gap: 6px; margin-top: 6px; }
.session-title { font-size: 12px; flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.session-meta { font-size: 9px; }
.empty { font-size: 10px; }

.thread { border-top: 1px solid var(--ui-border); padding-top: 12px; }
.thread-actions { margin-bottom: 8px; }
.mono { font-family: var(--ui-font-mono); border: 1px solid var(--ui-border); padding: 0 4px; border-radius: var(--ui-radius-xs); }
.remote-list { margin-top: 6px; }
.remote-row { display: flex; justify-content: space-between; gap: 8px; font-size: 10px; border-bottom: 1px solid var(--ui-hairline); padding: 2px 0; }
.usage { font-size: 9px; margin-bottom: 6px; }
.messages { display: flex; flex-direction: column; gap: 8px; margin-bottom: 10px; max-height: 420px; overflow-y: auto; }
.msg {
  border: 1px solid var(--ui-border);
  border-radius: var(--ui-radius-md);
  padding: 6px 8px;
  background: color-mix(in srgb, var(--ui-glass) 45%, transparent);
}
.msg-role { font-size: 9px; font-weight: 700; margin-bottom: 4px; }
.msg-body { font-size: 11px; white-space: pre-wrap; word-break: break-word; }
.role-user { border-color: var(--ui-border); }
.role-assistant_tool { border-style: dashed; }
.role-tool { background: color-mix(in srgb, var(--ui-text) 6%, transparent); }
.tool-block { margin-top: 4px; }
.tool-name { font-size: 10px; font-weight: 700; }
.tool-input { font-size: 9px; color: color-mix(in srgb, var(--ui-text) 70%, transparent); white-space: pre-wrap; margin: 4px 0 0; }

.approval {
  border: 1px solid color-mix(in srgb, var(--ui-warning) 55%, transparent);
  border-radius: var(--ui-radius-md);
  padding: 8px;
  margin-bottom: 10px;
  background: color-mix(in srgb, var(--ui-warning) 8%, transparent);
  backdrop-filter: blur(calc(var(--ui-blur) * 0.4));
  -webkit-backdrop-filter: blur(calc(var(--ui-blur) * 0.4));
}
.approval-title { font-size: 10px; font-weight: 700; color: var(--ui-warning); margin-bottom: 4px; display: flex; align-items: center; gap: 6px; }
.approval-text { font-size: 11px; margin-bottom: 8px; word-break: break-word; }

.prompt-box {
  width: 100%;
  min-height: 64px;
  resize: vertical;
  margin-bottom: 8px;
  background: color-mix(in srgb, var(--ui-surface) 70%, transparent);
  border: 1px solid var(--ui-border-strong);
  border-radius: var(--ui-radius-sm);
  color: var(--ui-text);
  font-family: var(--ui-font);
  font-size: var(--ui-fs-sm);
  padding: 8px 10px;
  outline: none;
  transition: border-color var(--ui-dur) var(--ui-ease-out), box-shadow var(--ui-dur) var(--ui-ease-out);
}
.prompt-box:hover { border-color: var(--ui-border-hover); }
.prompt-box:focus {
  border-color: color-mix(in srgb, var(--ui-accent) 70%, transparent);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--ui-accent) 15%, transparent);
}
.text-muted { color: color-mix(in srgb, var(--ui-text) 50%, transparent) !important; }
</style>
