<template>
  <div class="settings-panel">
    <div class="panel-header">
      <div class="header-left">
        <span class="icon-settings"><AppIcon name="solar:settings-bold" /></span>
        <h2 class="panel-title">SETTINGS</h2>
      </div>
    </div>

    <div class="section">
      <h3 class="section-title"><AppIcon name="solar:monitor-bold" :size="13" /> VIEW PREFERENCES</h3>
      <div class="setting-row">
        <span class="setting-label text-muted">DEFAULT VIEW</span>
        <UiSelect
          :model-value="store.viewMode"
          :options="['grid', 'list', 'masonry']"
          style="flex:1;"
          @update:model-value="store.viewMode = $event as 'grid' | 'list' | 'masonry'"
        />
      </div>
      <div class="setting-row">
        <span class="setting-label text-muted">MATRIX RAIN</span>
        <UiToggle v-model="store.matrixRainEnabled" aria-label="MATRIX RAIN" />
      </div>
      <div class="setting-row">
        <span class="setting-label text-muted">SIDEBAR DEFAULT</span>
        <UiToggle
          :model-value="!store.sidebarCollapsed"
          aria-label="SIDEBAR DEFAULT"
          @update:model-value="store.sidebarCollapsed = !$event"
        />
      </div>
    </div>

    <div class="section">
      <h3 class="section-title"><AppIcon name="solar:info-circle-bold" :size="13" /> ABOUT</h3>
      <div class="info-card">
        <div class="info-row"><span class="info-key text-muted">VERSION</span><span class="info-value">0.1.0</span></div>
        <div class="info-row"><span class="info-key text-muted">FRAMEWORK</span><span class="info-value">VUE 3 + PINIA</span></div>
        <div class="info-row"><span class="info-key text-muted">DESKTOP</span><span class="info-value">TAURI V2</span></div>
        <div class="info-row"><span class="info-key text-muted">SEARCH</span><span class="info-value">TANTIVY BM25</span></div>
        <div class="info-row"><span class="info-key text-muted">ENCRYPTION</span><span class="info-value">RUSTPQ (PQC)</span></div>
        <div class="info-row"><span class="info-key text-muted">DATABASE</span><span class="info-value">REDB</span></div>
      </div>
    </div>

    <div class="section">
      <h3 class="section-title"><AppIcon name="solar:server-bold" :size="13" /> CONNECTION</h3>
      <div class="setting-row">
        <span class="setting-label text-muted">MODE</span>
        <span class="info-value">{{ activeTransport }}</span>
      </div>
      <div class="setting-row">
        <span class="setting-label text-muted">API URL</span>
        <span class="info-value mono">{{ effectiveApiUrl }}</span>
      </div>
      <div class="setting-row" style="align-items:flex-start;">
        <span class="setting-label text-muted">REMOTE<br/>DASHBOARD</span>
        <div style="flex:1;display:flex;flex-direction:column;gap:6px;">
          <div style="display:flex;gap:6px;">
            <UiInput
              v-model="serverUrlDraft"
              style="flex:1;"
              placeholder="https://my-server:3456 (empty = auto)"
              aria-label="Remote dashboard URL"
              @enter="saveServerUrl"
            />
            <UiButton variant="primary" size="sm" icon="solar:diskette-bold" title="SAVE" aria-label="SAVE SERVER URL" @click="saveServerUrl" />
            <UiButton v-if="serverUrlDraft || currentServerUrl" size="sm" icon="solar:close-bold" title="Forget the remote dashboard" aria-label="CLEAR SERVER URL" @click="clearServerUrl" />
          </div>
          <p class="text-muted" style="font-size:9px;margin:0;">STATIC BUILD + YOUR OWN SERVER = FULL OAUTH, SYNC + QUOTA HERE. PAGE RELOADS TO RECONNECT.</p>
        </div>
      </div>
    </div>

    <div class="section">
      <h3 class="section-title"><AppIcon name="solar:key-bold" :size="13" /> SUPABASE BROKER</h3>
      <div class="setting-row">
        <span class="setting-label text-muted">STATUS</span>
        <span class="info-value">{{ supabaseStatus }}</span>
      </div>
      <div class="setting-row" style="align-items:flex-start;">
        <span class="setting-label text-muted">PROJECT<br/>URL</span>
        <UiInput
          v-model="supabaseUrlDraft"
          style="flex:1;"
          placeholder="https://xyzcompany.supabase.co"
          aria-label="Supabase project URL"
          autocomplete="off"
        />
      </div>
      <div class="setting-row" style="align-items:flex-start;">
        <span class="setting-label text-muted">ANON/<br/>PUBLISHABLE<br/>KEY</span>
        <div style="flex:1;display:flex;flex-direction:column;gap:6px;">
          <div style="display:flex;gap:6px;">
            <UiInput
              v-model="supabaseKeyDraft"
              style="flex:1;"
              type="password"
              placeholder="sb_publishable_… or eyJ…"
              aria-label="Supabase anon key"
              autocomplete="off"
            />
            <UiButton variant="primary" size="sm" icon="solar:diskette-bold" title="SAVE" aria-label="SAVE SUPABASE CONFIG" @click="saveSupabase" />
            <UiButton v-if="supabaseConfiguredNow" size="sm" icon="solar:close-bold" title="Forget Supabase config" aria-label="CLEAR SUPABASE CONFIG" @click="clearSupabase" />
          </div>
          <p class="text-muted" style="font-size:9px;margin:0;">STATIC BUILD OAUTH BROKER: GITHUB / GOOGLE / GITLAB LOGIN WITHOUT YOUR OWN SERVER. ENABLE THE PROVIDERS IN SUPABASE → AUTHENTICATION → SIGN-IN, AND ADD THIS PAGE'S URL TO REDIRECT URLS.</p>
        </div>
      </div>
    </div>

    <div class="section">
      <h3 class="section-title"><AppIcon name="solar:refresh-bold" :size="13" /> AUTO-REFRESH</h3>
      <div class="setting-row">
        <span class="setting-label text-muted">INTERVAL</span>
        <UiSelect
          :model-value="String(store.autoRefreshInterval)"
          :options="REFRESH_OPTIONS"
          style="flex:1;"
          @update:model-value="store.autoRefreshInterval = Number($event)"
        />
      </div>
    </div>

    <div class="section">
      <h3 class="section-title"><AppIcon name="solar:database-bold" :size="13" /> MANAGE</h3>
      <UiButton block icon="solar:refresh-bold" @click="handleRefresh">REFRESH ALL DATA</UiButton>
      <p class="text-muted" style="font-size:9px;margin-top:4px;">RE-FETCH FILES, ACCOUNTS, COLLECTIONS, FACE GROUPS, AND SYNC CONFIGS.</p>
    </div>

    <div class="section" v-if="touchConfig">
      <h3 class="section-title"><AppIcon name="solar:cursor-square-bold" :size="13" /> GESTURES</h3>
      <p class="text-muted" style="font-size:9px;margin-bottom:6px;">DEVICE: {{ touchConfig.state.touchSupported ? 'TOUCH ENABLED' : 'NO TOUCH' }} | {{ touchConfig.state.isMobile ? 'MOBILE' : 'DESKTOP' }}</p>
      <div class="gesture-table">
        <div v-for="gesture in touchConfig.getAllGestures()" :key="gesture" class="gesture-row">
          <span class="gesture-label text-muted">{{ touchConfig.getGestureLabel(gesture) }}</span>
          <UiSelect
            class="gesture-select"
            :model-value="touchConfig.getAction(gesture)"
            :options="touchConfig.getAllActions().map(a => ({ label: touchConfig.getActionLabel(a), value: a }))"
            @update:model-value="onGestureChange(gesture, $event)"
          />
          <button class="gesture-reset" @click="onGestureReset(gesture)" title="RESET" aria-label="RESET GESTURE"><AppIcon name="solar:undo-left-round-bold" :size="13" /></button>
        </div>
      </div>
      <div class="gesture-settings">
        <div class="gs-row">
          <span class="gs-label text-muted">SWIPE THRESHOLD</span>
          <input class="gs-input" type="number" :value="touchConfig.state.threshold" @change="touchConfig.state.threshold = parseInt(($event.target as HTMLInputElement).value)" min="10" max="200" />
        </div>
        <div class="gs-row">
          <span class="gs-label text-muted">LONG PRESS (MS)</span>
          <input class="gs-input" type="number" :value="touchConfig.state.longPressThreshold" @change="touchConfig.state.longPressThreshold = parseInt(($event.target as HTMLInputElement).value)" min="200" max="2000" step="50" />
        </div>
        <div class="gs-row">
          <span class="gs-label text-muted">EDGE ZONE (PX)</span>
          <input class="gs-input" type="number" :value="touchConfig.state.edgeZoneSize" @change="touchConfig.state.edgeZoneSize = parseInt(($event.target as HTMLInputElement).value)" min="10" max="100" />
        </div>
        <div class="gs-row">
          <span class="gs-label text-muted">DOUBLE TAP (MS)</span>
          <input class="gs-input" type="number" :value="touchConfig.state.doubleTapTimeout" @change="touchConfig.state.doubleTapTimeout = parseInt(($event.target as HTMLInputElement).value)" min="100" max="600" step="50" />
        </div>
      </div>
      <div class="gesture-actions">
        <UiButton icon="solar:undo-left-round-bold" @click="touchConfig.resetAll()">RESET ALL GESTURES</UiButton>
        <UiButton icon="solar:download-bold" @click="exportTouchConfig">EXPORT GESTURES</UiButton>
        <UiButton icon="solar:upload-bold" @click="importTouchConfig">IMPORT GESTURES</UiButton>
      </div>
      <input ref="touchImportRef" type="file" accept=".json" style="display:none" @change="handleTouchImport" />
    </div>

    <div class="section" v-if="shortcuts">
      <h3 class="section-title"><AppIcon name="solar:keyboard-bold" :size="13" /> KEYBOARD BINDINGS</h3>
      <div class="shortcuts-table">
        <div v-for="sc in shortcuts.getAllShortcuts()" :key="sc.action" class="sc-row">
          <span class="sc-action text-muted">{{ sc.description }}</span>
          <div class="sc-binding">
            <input
              class="sc-input"
              :value="getBindingDisplay(sc.action)"
              @focus="startRebind(sc.action, $event)"
              @keydown="captureRebind($event)"
              :ref="(el: any) => { if (el) rebindInputs[sc.action] = el as HTMLInputElement }"
              :placeholder="sc.keys"
              readonly
            />
            <button class="sc-reset" @click="resetBinding(sc.action)" title="RESET TO DEFAULT" aria-label="RESET TO DEFAULT"><AppIcon name="solar:undo-left-round-bold" :size="13" /></button>
          </div>
        </div>
      </div>
      <div class="sc-actions">
        <UiButton icon="solar:download-bold" @click="exportKeymap">EXPORT KEYMAP</UiButton>
        <UiButton icon="solar:upload-bold" @click="importKeymap">IMPORT KEYMAP</UiButton>
        <UiButton icon="solar:undo-left-round-bold" @click="resetAllBindings">RESET ALL</UiButton>
      </div>
      <input
        ref="importInputRef"
        type="file"
        accept=".kpl,.kpd,.json"
        style="display:none"
        @change="handleImportFile"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import AppIcon from '@/components/AppIcon.vue'
import { ref, inject, computed } from 'vue'
import { useAppStore } from '@/stores/app'
import { isWebMode, isTauri, getServerUrl, setServerUrl } from '@/composables/useTauri'
import {
  getSupabaseConfig,
  setSupabaseConfig,
  clearSupabaseConfig,
  supabaseConfigured,
  supabaseSignOut,
} from '@/composables/useSupabase'
import { wasmBackendActive } from '@/composables/useWasmBackend'
import { ShortcutsKey } from '@/composables/shortcutsKey'
import { useTouchConfig, type GestureType, type TouchAction } from '@/composables/useTouchConfig'

/** Active transport: tauri IPC, REST dashboard, or local WASM (GitHub Pages). */
const activeTransport = computed(() => {
  if (import.meta.env.VITE_TRANSPORT) return String(import.meta.env.VITE_TRANSPORT).toUpperCase()
  if (isTauri()) return 'TAURI DESKTOP (REST_FIRST: os/disk via :3456)'
  if (wasmBackendActive()) return 'WASM LOCAL (PAGES)'
  if (isWebMode()) return 'WEB / REST (:3456)'
  return 'UNKNOWN'
})

const store = useAppStore()
const shortcuts = inject(ShortcutsKey, null)
const touchConfig = useTouchConfig()

const REFRESH_OPTIONS = [
  { label: 'DISABLED', value: '0' },
  { label: '10 SECONDS', value: '10' },
  { label: '30 SECONDS', value: '30' },
  { label: '1 MINUTE', value: '60' },
  { label: '5 MINUTES', value: '300' },
]

const currentServerUrl = computed(() => getServerUrl())
const serverUrlDraft = ref(getServerUrl())
const effectiveApiUrl = computed(() => {
  if (currentServerUrl.value) return currentServerUrl.value.toUpperCase()
  if (isTauri()) return 'TAURI IPC + HTTP://LOCALHOST:3456'
  return 'HTTP://LOCALHOST:3456'
})

function saveServerUrl() {
  setServerUrl(serverUrlDraft.value.trim())
  window.location.reload()
}

function clearServerUrl() {
  serverUrlDraft.value = ''
  setServerUrl('')
  window.location.reload()
}

const supabaseUrlDraft = ref(getSupabaseConfig().url)
const supabaseKeyDraft = ref('')
const supabaseConfiguredNow = computed(() => supabaseConfigured())
const supabaseStatus = computed(() =>
  supabaseConfiguredNow.value ? `CONFIGURED (${getSupabaseConfig().url})` : 'NOT CONFIGURED',
)

function saveSupabase() {
  if (!supabaseUrlDraft.value.trim() || !supabaseKeyDraft.value.trim()) return
  setSupabaseConfig(supabaseUrlDraft.value, supabaseKeyDraft.value)
  supabaseKeyDraft.value = ''
}

async function clearSupabase() {
  await supabaseSignOut().catch(() => {})
  clearSupabaseConfig()
  supabaseUrlDraft.value = ''
}

const rebindInputs: Record<string, HTMLInputElement> = {}
const rebindingAction = ref<string | null>(null)
const importInputRef = ref<HTMLInputElement | null>(null)

function getBindingDisplay(action: string): string {
  return shortcuts?.getShortcut(action)?.replace(/,/g, ', ') || ''
}

function startRebind(action: string, e: FocusEvent) {
  rebindingAction.value = action
  const input = e.target as HTMLInputElement
  input.value = 'PRESS KEYS...'
  input.select()
}

function captureRebind(e: KeyboardEvent) {
  if (!rebindingAction.value) return
  e.preventDefault()
  e.stopPropagation()
  const parts: string[] = []
  if (e.ctrlKey) parts.push('Ctrl')
  if (e.altKey) parts.push('Alt')
  if (e.shiftKey) parts.push('Shift')
  if (e.metaKey) parts.push('Meta')
  const key = e.key
  if (!['Control', 'Alt', 'Shift', 'Meta'].includes(key)) {
    parts.push(key.length === 1 ? key.toUpperCase() : key)
  }
  if (parts.length === 0) return
  const seq = parts.join('+')
  const el = rebindInputs[rebindingAction.value]
  if (el) el.value = seq
  saveOverride(rebindingAction.value, seq)
  rebindingAction.value = null
  ;(e.target as HTMLInputElement).blur()
}

function saveOverride(action: string, keys: string) {
  try {
    const raw = localStorage.getItem('cybermanju_keybindings')
    const overrides = raw ? JSON.parse(raw) : {}
    overrides[action] = keys
    localStorage.setItem('cybermanju_keybindings', JSON.stringify(overrides))
    window.location.reload()
  } catch {}
}

function resetBinding(action: string) {
  try {
    const raw = localStorage.getItem('cybermanju_keybindings')
    const overrides = raw ? JSON.parse(raw) : {}
    delete overrides[action]
    localStorage.setItem('cybermanju_keybindings', JSON.stringify(overrides))
    window.location.reload()
  } catch {}
}

function resetAllBindings() {
  localStorage.removeItem('cybermanju_keybindings')
  window.location.reload()
}

function exportKeymap() {
  const all = shortcuts?.getAllShortcuts() || []
  const lines = ['[Global]', 'name=Cybermanju Exported', 'version=1.0', 'description=Exported from Settings', '']
  lines.push('[Global Shortcuts]')
  for (const s of all) {
    const keys = shortcuts?.getShortcut(s.action) || s.keys
    lines.push(`${s.action}=${keys.replace(/,\s*/g, ',')}`)
  }
  const blob = new Blob([lines.join('\n')], { type: 'text/plain' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = 'cybermanju-shortcuts.kpl'
  a.click()
  URL.revokeObjectURL(url)
}

function importKeymap() {
  importInputRef.value?.click()
}

function handleImportFile(e: Event) {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  const reader = new FileReader()
  reader.onload = () => {
    const text = reader.result as string
    try {
      const parsed = JSON.parse(text)
      localStorage.setItem('cybermanju_keybindings', JSON.stringify(parsed))
      window.location.reload()
    } catch {
      const overrides: Record<string, string> = {}
      for (const line of text.split('\n')) {
        const eqIdx = line.indexOf('=')
        if (eqIdx === -1 || line.startsWith('[') || line.startsWith('#') || line.startsWith(';')) continue
        const key = line.slice(0, eqIdx).trim()
        const val = line.slice(eqIdx + 1).trim()
        if (key && val) overrides[key] = val
      }
      localStorage.setItem('cybermanju_keybindings', JSON.stringify(overrides))
      window.location.reload()
    }
  }
  reader.readAsText(file)
}

const touchImportRef = ref<HTMLInputElement | null>(null)

function onGestureChange(gesture: GestureType, action: string) {
  touchConfig.setAction(gesture, action as TouchAction)
}

function onGestureReset(gesture: GestureType) {
  touchConfig.resetGesture(gesture)
}

function exportTouchConfig() {
  const json = touchConfig.exportConfig()
  const blob = new Blob([json], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = 'cybermanju-touch-config.json'
  a.click()
  URL.revokeObjectURL(url)
}

function importTouchConfig() {
  touchImportRef.value?.click()
}

function handleTouchImport(e: Event) {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  const reader = new FileReader()
  reader.onload = () => {
    const text = reader.result as string
    if (touchConfig.importConfig(text)) {
      window.location.reload()
    }
  }
  reader.readAsText(file)
}

async function handleRefresh() {
  await Promise.allSettled([
    store.fetchFiles(),
    store.fetchAccounts(),
    store.fetchCollections(),
    store.fetchFaceGroups(),
    store.fetchLooseGroups(),
    store.fetchEncryptionStatus(),
    store.fetchSyncConfigs(),
  ])
}
</script>

<style scoped>
.settings-panel {
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
.icon-settings { font-size: 16px; }
.panel-title { font-size: 14px; font-weight: 800; letter-spacing: 1px; margin: 0; }

.section { margin-bottom: 16px; }

.section-title {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 1px;
  color: color-mix(in srgb, var(--ui-text) 60%, transparent);
  margin: 0 0 8px;
  padding-bottom: 4px;
  border-bottom: 1px solid var(--ui-hairline);
}

.setting-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 0;
}

.setting-label {
  font-size: 10px;
  min-width: 100px;
  flex-shrink: 0;
}

.info-card {
  border: 1px solid var(--ui-border);
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.info-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.info-key { font-size: 10px; }
.info-value { font-size: 10px; font-weight: 700; }
.mono { font-family: var(--ui-font); }

.bw-input {
  background: var(--ui-surface);
  border: 1px solid var(--ui-border);
  padding: 4px 8px;
  color: var(--ui-text);
  font-family: var(--ui-font);
  font-size: 10px;
}

.bw-btn {
  padding: 4px 12px;
  background: var(--ui-surface);
  border: 1px solid var(--ui-border);
  color: var(--ui-text);
  font-family: var(--ui-font);
  font-size: 10px;
  font-weight: 700;
  cursor: pointer;
}

.bw-btn:hover { background: var(--ui-glass-2); color: var(--ui-text); }
.bw-btn-inverse { background: var(--ui-glass-2); color: var(--ui-text); }
.bw-btn-inverse:hover { background: var(--ui-surface); color: var(--ui-text); }

.shortcuts-table {
  border: 1px solid var(--ui-hairline);
  max-height: 300px;
  overflow-y: auto;
  margin-bottom: 8px;
}

.sc-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 3px 8px;
  font-size: 10px;
  border-bottom: 1px solid var(--ui-border);
}

.sc-action {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sc-binding {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
}

.sc-input {
  width: 120px;
  background: var(--ui-surface);
  border: 1px solid var(--ui-border);
  color: var(--ui-text);
  font-family: var(--ui-font);
  font-size: 9px;
  padding: 2px 4px;
  cursor: pointer;
  text-align: center;
}

.sc-input:focus {
  background: var(--ui-glass-2);
  color: var(--ui-text);
}

.sc-reset {
  background: transparent;
  border: 1px solid var(--ui-border);
  color: color-mix(in srgb, var(--ui-text) 60%, transparent);
  font-family: var(--ui-font);
  font-size: 8px;
  padding: 1px 4px;
  cursor: pointer;
}

.sc-reset:hover {
  background: var(--ui-glass-2);
  color: var(--ui-text);
}

.sc-actions {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

.gesture-table {
  border: 1px solid var(--ui-hairline);
  max-height: 300px;
  overflow-y: auto;
  margin-bottom: 8px;
}

.gesture-row {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 3px 6px;
  font-size: 9px;
  border-bottom: 1px solid var(--ui-border);
}

.gesture-label {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 9px;
}

.gesture-select {
  width: 140px;
  background: var(--ui-surface);
  border: 1px solid var(--ui-border);
  color: var(--ui-text);
  font-family: var(--ui-font);
  font-size: 8px;
  padding: 1px 2px;
}

.gesture-reset {
  background: transparent;
  border: 1px solid var(--ui-border);
  color: color-mix(in srgb, var(--ui-text) 50%, transparent);
  font-family: var(--ui-font);
  font-size: 8px;
  padding: 1px 4px;
  cursor: pointer;
}

.gesture-reset:hover { background: var(--ui-glass-2); color: var(--ui-text); }

.gesture-settings {
  border: 1px solid var(--ui-hairline);
  padding: 6px;
  margin-bottom: 8px;
}

.gs-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 3px 0;
  font-size: 9px;
}

.gs-label { font-size: 9px; }
.gs-input {
  width: 60px;
  background: var(--ui-surface);
  border: 1px solid var(--ui-border);
  color: var(--ui-text);
  font-family: var(--ui-font);
  font-size: 9px;
  padding: 1px 4px;
  text-align: center;
}

.gesture-actions {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

.text-muted { color: color-mix(in srgb, var(--ui-text) 50%, transparent) !important; }
</style>
