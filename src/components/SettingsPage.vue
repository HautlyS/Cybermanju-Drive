<template>
  <div class="settings-panel">
    <div class="panel-header">
      <div class="header-left">
        <span class="icon-settings">[@]</span>
        <h2 class="panel-title">SETTINGS</h2>
      </div>
    </div>

    <div class="section">
      <h3 class="section-title">[DISPLAY] VIEW PREFERENCES</h3>
      <div class="setting-row">
        <span class="setting-label text-muted">DEFAULT VIEW</span>
        <select v-model="store.viewMode" class="bw-input" style="flex:1;">
          <option value="grid">GRID</option>
          <option value="list">LIST</option>
          <option value="masonry">MASONRY</option>
        </select>
      </div>
      <div class="setting-row">
        <span class="setting-label text-muted">MATRIX RAIN</span>
        <button class="bw-btn" :class="{ 'bw-btn-inverse': store.matrixRainEnabled }" @click="store.matrixRainEnabled = !store.matrixRainEnabled">
          {{ store.matrixRainEnabled ? '[ON]' : '[OFF]' }}
        </button>
      </div>
      <div class="setting-row">
        <span class="setting-label text-muted">SIDEBAR DEFAULT</span>
        <button class="bw-btn" :class="{ 'bw-btn-inverse': !store.sidebarCollapsed }" @click="store.sidebarCollapsed = !store.sidebarCollapsed">
          {{ store.sidebarCollapsed ? 'COLLAPSED' : 'EXPANDED' }}
        </button>
      </div>
    </div>

    <div class="section">
      <h3 class="section-title">[INFO] ABOUT</h3>
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
      <h3 class="section-title">[SERVER] CONNECTION</h3>
      <div class="setting-row">
        <span class="setting-label text-muted">MODE</span>
        <span class="info-value">{{ isWebMode() ? 'WEB / REST' : 'TAURI DESKTOP' }}</span>
      </div>
      <div class="setting-row">
        <span class="setting-label text-muted">API URL</span>
        <span class="info-value mono">HTTP://LOCALHOST:3456</span>
      </div>
    </div>

    <div class="section">
      <h3 class="section-title">[REFRESH] AUTO-REFRESH</h3>
      <div class="setting-row">
        <span class="setting-label text-muted">INTERVAL</span>
        <select v-model.number="store.autoRefreshInterval" class="bw-input" style="flex:1;">
          <option :value="0">DISABLED</option>
          <option :value="10">10 SECONDS</option>
          <option :value="30">30 SECONDS</option>
          <option :value="60">1 MINUTE</option>
          <option :value="300">5 MINUTES</option>
        </select>
      </div>
    </div>

    <div class="section">
      <h3 class="section-title">[DATA] MANAGE</h3>
      <button class="bw-btn" style="width:100%;" @click="handleRefresh">[REFRESH ALL DATA]</button>
      <p class="text-muted" style="font-size:9px;margin-top:4px;">RE-FETCH FILES, ACCOUNTS, COLLECTIONS, FACE GROUPS, AND SYNC CONFIGS.</p>
    </div>

    <div class="section" v-if="touchConfig">
      <h3 class="section-title">[TOUCH] GESTURES</h3>
      <p class="text-muted" style="font-size:9px;margin-bottom:6px;">DEVICE: {{ touchConfig.state.touchSupported ? 'TOUCH ENABLED' : 'NO TOUCH' }} | {{ touchConfig.state.isMobile ? 'MOBILE' : 'DESKTOP' }}</p>
      <div class="gesture-table">
        <div v-for="gesture in touchConfig.getAllGestures()" :key="gesture" class="gesture-row">
          <span class="gesture-label text-muted">{{ touchConfig.getGestureLabel(gesture) }}</span>
          <select
            class="gesture-select"
            :value="touchConfig.getAction(gesture)"
            @change="onGestureChange(gesture, ($event.target as HTMLSelectElement).value)"
          >
            <option v-for="a in touchConfig.getAllActions()" :key="a" :value="a">{{ touchConfig.getActionLabel(a) }}</option>
          </select>
          <button class="gesture-reset" @click="onGestureReset(gesture)" title="RESET">[R]</button>
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
        <button class="bw-btn" @click="touchConfig.resetAll()">[RESET ALL GESTURES]</button>
        <button class="bw-btn" @click="exportTouchConfig">[EXPORT GESTURES]</button>
        <button class="bw-btn" @click="importTouchConfig">[IMPORT GESTURES]</button>
      </div>
      <input ref="touchImportRef" type="file" accept=".json" style="display:none" @change="handleTouchImport" />
    </div>

    <div class="section" v-if="shortcuts">
      <h3 class="section-title">[SHORTCUTS] KEYBOARD BINDINGS</h3>
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
            <button class="sc-reset" @click="resetBinding(sc.action)" title="RESET TO DEFAULT">[R]</button>
          </div>
        </div>
      </div>
      <div class="sc-actions">
        <button class="bw-btn" @click="exportKeymap">[EXPORT KEYMAP]</button>
        <button class="bw-btn" @click="importKeymap">[IMPORT KEYMAP]</button>
        <button class="bw-btn" @click="resetAllBindings">[RESET ALL]</button>
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
import { ref, inject } from 'vue'
import { useAppStore } from '@/stores/app'
import { isWebMode } from '@/composables/useTauri'
import { ShortcutsKey } from '@/composables/shortcutsKey'
import { useTouchConfig, type GestureType, type TouchAction } from '@/composables/useTouchConfig'

const store = useAppStore()
const shortcuts = inject(ShortcutsKey, null)
const touchConfig = useTouchConfig()

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
.icon-settings { font-size: 16px; }
.panel-title { font-size: 14px; font-weight: 800; letter-spacing: 1px; margin: 0; }

.section { margin-bottom: 16px; }

.section-title {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 1px;
  color: rgba(255,255,255,0.6);
  margin: 0 0 8px;
  padding-bottom: 4px;
  border-bottom: 2px solid rgba(255,255,255,0.2);
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
  border: 2px solid #FFFFFF;
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
.mono { font-family: 'Courier New', monospace; }

.bw-input {
  background: #000;
  border: 2px solid #FFFFFF;
  padding: 4px 8px;
  color: #FFFFFF;
  font-family: 'Courier New', monospace;
  font-size: 10px;
}

.bw-btn {
  padding: 4px 12px;
  background: #000000;
  border: 2px solid #FFFFFF;
  color: #FFFFFF;
  font-family: 'Courier New', monospace;
  font-size: 10px;
  font-weight: 700;
  cursor: pointer;
}

.bw-btn:hover { background: #FFFFFF; color: #000000; }
.bw-btn-inverse { background: #FFFFFF; color: #000000; }
.bw-btn-inverse:hover { background: #000000; color: #FFFFFF; }

.shortcuts-table {
  border: 2px solid rgba(255,255,255,0.3);
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
  border-bottom: 1px solid rgba(255,255,255,0.08);
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
  background: #000;
  border: 2px solid #FFFFFF;
  color: #FFFFFF;
  font-family: 'Courier New', monospace;
  font-size: 9px;
  padding: 2px 4px;
  cursor: pointer;
  text-align: center;
}

.sc-input:focus {
  background: #FFFFFF;
  color: #000;
}

.sc-reset {
  background: transparent;
  border: 1px solid rgba(255,255,255,0.3);
  color: rgba(255,255,255,0.6);
  font-family: 'Courier New', monospace;
  font-size: 8px;
  padding: 1px 4px;
  cursor: pointer;
}

.sc-reset:hover {
  background: #FFFFFF;
  color: #000;
}

.sc-actions {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

.gesture-table {
  border: 2px solid rgba(255,255,255,0.3);
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
  border-bottom: 1px solid rgba(255,255,255,0.06);
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
  background: #000;
  border: 2px solid #FFFFFF;
  color: #FFFFFF;
  font-family: 'Courier New', monospace;
  font-size: 8px;
  padding: 1px 2px;
}

.gesture-reset {
  background: transparent;
  border: 1px solid rgba(255,255,255,0.3);
  color: rgba(255,255,255,0.5);
  font-family: 'Courier New', monospace;
  font-size: 8px;
  padding: 1px 4px;
  cursor: pointer;
}

.gesture-reset:hover { background: #FFFFFF; color: #000; }

.gesture-settings {
  border: 2px solid rgba(255,255,255,0.3);
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
  background: #000;
  border: 2px solid #FFFFFF;
  color: #FFFFFF;
  font-family: 'Courier New', monospace;
  font-size: 9px;
  padding: 1px 4px;
  text-align: center;
}

.gesture-actions {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

.text-muted { color: rgba(255,255,255,0.5) !important; }
</style>
