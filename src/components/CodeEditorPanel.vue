<template>
  <div class="editor-panel">
    <div class="panel-header">
      <div class="header-left">
        <span class="icon-editor">[E]</span>
        <h2 class="panel-title">EDITOR</h2>
        <span class="text-muted transport-tag">{{ transportLabel }}</span>
        <span
          v-if="activeTab?.parse"
          class="engine-badge"
          :class="{ 'engine-ts': activeTab.parse.engine === 'tree-sitter' }"
          :title="activeTab.parse.engine === 'tree-sitter' ? 'Real grammar parse' : 'Regex fallback — same shape, less precise'"
        >{{ (activeTab.parse.engine || 'heuristic').toUpperCase() }}</span>
      </div>
      <div class="header-actions">
        <button class="bw-btn-sm" :disabled="!activeTab || saving" @click="saveActive">
          {{ saving ? '[SAVING…]' : '[SAVE Ctrl+S]' }}
        </button>
        <button class="bw-btn-sm" :disabled="!activeTab" @click="showOutline = !showOutline">
          {{ showOutline ? '[OUTLINE✓]' : '[OUTLINE]' }}
        </button>
        <button class="bw-btn-sm" :disabled="!activeTab" @click="showFind = !showFind">[FIND]</button>
      </div>
    </div>

    <div class="editor-body">
      <!-- Explorer -->
      <div class="explorer">
        <div class="explorer-head">
          <span class="section-title">{{ isWasm ? '[VOLUME] WASM FILES' : '[FILES] EDITABLE' }}</span>
          <button class="ghost-btn" @click="refreshExplorer" title="REFRESH">↻</button>
        </div>
        <input v-model="explorerFilter" class="bw-input explorer-search" placeholder="FILTER…" spellcheck="false" />
        <div v-if="isWasm" class="wasm-nav">
          <button class="ghost-btn" :disabled="wasmCwd === '/'" @click="wasmUp">↑</button>
          <span class="text-muted wasm-path">{{ wasmCwd }}</span>
        </div>
        <div v-if="isWasm" class="wasm-new">
          <input v-model="newFileName" class="bw-input" placeholder="new-file.txt" spellcheck="false" @keyup.enter="createWasmFile" />
          <button class="bw-btn-sm" :disabled="!newFileName.trim()" @click="createWasmFile">[+]</button>
        </div>
        <div class="explorer-list">
          <div
            v-for="entry in explorerEntries"
            :key="entry.key"
            class="explorer-row"
            :class="{ active: entry.key === activeKey, dir: entry.isDir }"
            @click="entry.isDir ? navigateWasm(entry.path) : openEntry(entry)"
            @dblclick="entry.isDir ? navigateWasm(entry.path) : openEntry(entry)"
          >
            <span class="explorer-icon">{{ entry.isDir ? '▸' : '·' }}</span>
            <span class="explorer-name">{{ entry.label }}</span>
            <span v-if="!entry.isDir && entry.sub" class="explorer-sub text-muted">{{ entry.sub }}</span>
          </div>
          <p v-if="!explorerEntries.length" class="text-muted empty">
            {{ isWasm ? 'EMPTY DIRECTORY' : 'NO EDITABLE TEXT FILES (≤1 MiB, NOT ENCRYPTED)' }}
          </p>
        </div>
        <p v-if="!isWasm" class="text-muted hint">NEW FILES COME FROM IMPORTS/UPLOADS — THE EDITOR EDITS, VERSIONS SNAPSHOT ON SAVE.</p>
      </div>

      <!-- Main -->
      <div class="main">
        <div v-if="tabs.length" class="tabs">
          <div
            v-for="tab in tabs"
            :key="tab.key"
            class="tab"
            :class="{ active: tab.key === activeKey, dirty: tab.dirty }"
            @click="activeKey = tab.key"
          >
            <span class="tab-dot" v-if="tab.dirty">●</span>
            <span class="tab-label">{{ tab.label }}</span>
            <button class="tab-x" @click.stop="closeTab(tab.key)" title="CLOSE">×</button>
          </div>
        </div>

        <div v-if="activeTab && showFind" class="findbar">
          <input
            v-model="findQuery"
            class="bw-input find-input"
            placeholder="FIND…"
            spellcheck="false"
            @keyup.enter="findNext(1)"
          />
          <span class="text-muted find-count">{{ findCount ? `${findIndex + 1}/${findCount}` : findQuery ? '0/0' : '' }}</span>
          <button class="bw-btn-sm" @click="findNext(1)">[↓]</button>
          <button class="bw-btn-sm" @click="findNext(-1)">[↑]</button>
        </div>

        <div v-if="!activeTab" class="empty-editor">
          <p class="text-muted">OPEN A FILE FROM THE EXPLORER{{ isWasm ? ' — SAVES STAY IN THIS BROWSER' : '' }}</p>
          <p v-if="lastRefusal" class="refusal">{{ lastRefusal }}</p>
        </div>

        <div v-else-if="activeTab.readOnly" class="empty-editor">
          <p class="refusal">{{ activeTab.refuseReason }}</p>
          <p class="text-muted">READ-ONLY — CONTENT NOT LOADED FOR EDITING</p>
        </div>

        <div v-else class="edit-wrap">
          <div class="gutter" ref="gutterEl">{{ gutterText }}</div>
          <div class="code-area">
            <pre ref="highlightEl" class="highlight" aria-hidden="true"><code v-html="highlightedCode"></code></pre>
            <textarea
              ref="textareaEl"
              v-model="activeTab.content"
              class="editor-input"
              spellcheck="false"
              wrap="off"
              :aria-label="`Editing ${activeTab.label}`"
              @scroll="syncScroll"
              @keydown="onKeydown"
              @keyup="updateCursor"
              @click="updateCursor"
              @select="updateCursor"
              @input="onEdit"
            />
          </div>
        </div>

        <div v-if="activeTab && !activeTab.readOnly" class="statusbar">
          <span>{{ activeTab.language }}</span>
          <span class="text-muted">Ln {{ cursorLine }}, Col {{ cursorCol }}</span>
          <span class="text-muted">{{ activeTab.content.length }} B · {{ lineCount }} LINES</span>
          <span v-if="activeTab.dirty" class="dirty-tag">● UNSAVED</span>
          <span v-else class="text-muted">SAVED</span>
          <span class="text-muted">{{ outlineStatus }}</span>
        </div>

        <!-- Outline -->
        <div v-if="activeTab && showOutline" class="outline">
          <div class="outline-head">
            <span class="section-title">[OUTLINE] {{ outlineSymbols.length }} SYMBOLS</span>
            <button class="ghost-btn" :disabled="parsingOutline" @click="reparseActive">
              {{ parsingOutline ? '…' : '↻' }}
            </button>
          </div>
          <input v-model="outlineFilter" class="bw-input outline-search" placeholder="FILTER OUTLINE…" spellcheck="false" />
          <div v-if="outlineSymbols.length" class="symbol-tree">
            <div
              v-for="sym in outlineSymbols"
              :key="sym.name + sym.startLine"
              class="symbol-row"
              @click="jumpToLine(sym.startLine)"
              :title="sym.detail || sym.name"
            >
              <span class="symbol-kind">{{ sym.kind }}</span>
              <span class="symbol-name">{{ sym.name }}</span>
              <span class="symbol-lines text-muted">:{{ sym.startLine }}</span>
            </div>
          </div>
          <p v-else class="text-muted empty">{{ activeTab.parse ? 'NO MATCHING SYMBOLS' : 'OUTLINE NOT PARSED YET' }}</p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, nextTick, onBeforeUnmount, watch } from 'vue'
import { useAppStore } from '@/stores/app'
import { invoke, isTauri, isStaticHost } from '@/composables/useTauri'
import type { CodeSymbol, FileNode, ParseResult } from '@/types'

interface Tab {
  key: string
  label: string
  kind: 'managed' | 'wasm'
  fileId: string
  path: string
  language: string
  content: string
  savedContent: string
  dirty: boolean
  parse: ParseResult | null
  readOnly: boolean
  refuseReason: string
}

interface ExplorerEntry {
  key: string
  label: string
  path: string
  isDir: boolean
  sub?: string
  node?: FileNode
}

const store = useAppStore()
const isDesktop = isTauri()
const isWasm = computed(() => {
  try {
    return isStaticHost()
  } catch {
    return false
  }
})
const transportLabel = computed(() => {
  if (isDesktop) return 'DESKTOP'
  if (isWasm.value) return 'WASM LOCAL'
  return 'WEB / REST'
})

const tabs = ref<Tab[]>([])
const activeKey = ref('')
const activeTab = computed(() => tabs.value.find(t => t.key === activeKey.value) ?? null)

const showOutline = ref(true)
const showFind = ref(false)
const saving = ref(false)
const parsingOutline = ref(false)
const lastRefusal = ref('')
const explorerFilter = ref('')
const outlineFilter = ref('')
const findQuery = ref('')
const findIndex = ref(-1)
const cursorLine = ref(1)
const cursorCol = ref(1)

const textareaEl = ref<HTMLTextAreaElement | null>(null)
const highlightEl = ref<HTMLElement | null>(null)
const gutterEl = ref<HTMLElement | null>(null)

const wasmCwd = ref('/')
const wasmEntries = ref<string[]>([])
const newFileName = ref('')

const MAX_EDIT_BYTES = 1024 * 1024
const CODE_EXTS = new Set([
  'rs', 'ts', 'tsx', 'js', 'jsx', 'py', 'go', 'c', 'h', 'cpp', 'hpp', 'cc',
  'java', 'rb', 'swift', 'kt', 'html', 'htm', 'css', 'scss', 'less', 'json',
  'toml', 'yaml', 'yml', 'md', 'mdx', 'sql', 'sh', 'bash', 'zsh', 'lua',
  'zig', 'ex', 'vue', 'svelte', 'dart', 'txt', 'log', 'toml', 'ini', 'cfg',
])

function extOf(name: string): string {
  const parts = name.toLowerCase().split('.')
  return parts.length > 1 ? parts[parts.length - 1] : ''
}

function isEditableNode(n: FileNode): boolean {
  if (n.fileType !== 'file' || n.encrypted) return false
  if (n.sizeBytes > MAX_EDIT_BYTES) return false
  const mime = n.mimeType ?? ''
  if (mime.startsWith('text/')) return true
  if (mime === 'application/json') return true
  return CODE_EXTS.has(extOf(n.name))
}

function langOf(name: string): string {
  const ext = extOf(name)
  const map: Record<string, string> = {
    rs: 'rust', py: 'python', js: 'javascript', jsx: 'javascript',
    ts: 'typescript', tsx: 'typescript', go: 'go', sh: 'bash', bash: 'bash',
  }
  return map[ext] ?? (ext || 'text')
}

// ─── Explorer ───

const managedEntries = computed<ExplorerEntry[]>(() => {
  const q = explorerFilter.value.trim().toLowerCase()
  return store.files
    .filter(n => isEditableNode(n))
    .filter(n => !q || n.name.toLowerCase().includes(q))
    .map(n => ({
      key: `m:${n.id}`,
      label: n.name,
      path: n.id,
      isDir: false,
      sub: `${Math.max(0, Math.round(n.sizeBytes / 1024))}K`,
      node: n,
    }))
})

const wasmListEntries = computed<ExplorerEntry[]>(() => {
  const q = explorerFilter.value.trim().toLowerCase()
  return wasmEntries.value
    .filter(name => !q || name.toLowerCase().includes(q))
    .map(name => {
      const isDir = name.endsWith('/')
      const clean = isDir ? name.slice(0, -1) : name
      const full = wasmCwd.value === '/' ? `/${clean}` : `${wasmCwd.value}/${clean}`
      return {
        key: `w:${full}`,
        label: clean,
        path: full,
        isDir,
        sub: isDir ? 'DIR' : undefined,
      }
    })
})

const explorerEntries = computed(() => (isWasm.value ? wasmListEntries.value : managedEntries.value))

async function refreshExplorer() {
  if (isWasm.value) {
    wasmEntries.value = await store.listWasmDir(wasmCwd.value)
  } else {
    await store.fetchFiles()
  }
}

function wasmUp() {
  if (wasmCwd.value === '/') return
  const parts = wasmCwd.value.split('/').filter(Boolean)
  parts.pop()
  wasmCwd.value = parts.length ? `/${parts.join('/')}` : '/'
  void refreshExplorer()
}

function navigateWasm(path: string) {
  wasmCwd.value = path
  void refreshExplorer()
}

async function createWasmFile() {
  const name = newFileName.value.trim().replace(/\//g, '_')
  if (!name) return
  const full = wasmCwd.value === '/' ? `/${name}` : `${wasmCwd.value}/${name}`
  const ok = await store.saveWasmFile(full, '')
  newFileName.value = ''
  await refreshExplorer()
  if (ok) await openWasmPath(full, name)
}

// ─── Tabs ───

function focusTab(key: string) {
  activeKey.value = key
  nextTick(() => {
    updateCursor()
    syncScroll()
    textareaEl.value?.focus()
  })
}

async function openEntry(entry: ExplorerEntry) {
  lastRefusal.value = ''
  if (entry.isDir) return
  if (isWasm.value) {
    await openWasmPath(entry.path, entry.label)
    return
  }
  if (!entry.node) return
  const existing = tabs.value.find(t => t.key === entry.key)
  if (existing) {
    focusTab(existing.key)
    return
  }
  const res = await store.readManagedContent(entry.node.id)
  if (!res) {
    lastRefusal.value = `Could not open ${entry.label} (see notification).`
    return
  }
  tabs.value.push({
    key: entry.key,
    label: entry.node.name,
    kind: 'managed',
    fileId: entry.node.id,
    path: entry.node.id,
    language: langOf(entry.node.name),
    content: res.content,
    savedContent: res.content,
    dirty: false,
    parse: null,
    readOnly: false,
    refuseReason: '',
  })
  focusTab(entry.key)
  void reparseActive()
}

async function openWasmPath(path: string, label: string) {
  const key = `w:${path}`
  const existing = tabs.value.find(t => t.key === key)
  if (existing) {
    focusTab(key)
    return
  }
  const content = await store.readWasmFile(path)
  if (content === null) {
    lastRefusal.value = `Could not open ${label}.`
    return
  }
  tabs.value.push({
    key,
    label,
    kind: 'wasm',
    fileId: '',
    path,
    language: langOf(label),
    content,
    savedContent: content,
    dirty: false,
    parse: null,
    readOnly: false,
    refuseReason: '',
  })
  focusTab(key)
  void reparseActive()
}

function closeTab(key: string) {
  const i = tabs.value.findIndex(t => t.key === key)
  if (i === -1) return
  tabs.value.splice(i, 1)
  if (activeKey.value === key) {
    activeKey.value = tabs.value.length ? tabs.value[Math.max(0, i - 1)].key : ''
  }
}

// ─── Editing ───

const gutterText = computed(() => {
  const n = activeTab.value ? activeTab.value.content.split('\n').length : 0
  return Array.from({ length: n }, (_, i) => String(i + 1)).join('\n')
})

const lineCount = computed(() => (activeTab.value ? activeTab.value.content.split('\n').length : 0))

const highlightedCode = computed(() => {
  const tab = activeTab.value
  if (!tab) return ''
  return highlightSyntax(tab.content, tab.language)
})

function syncScroll() {
  const ta = textareaEl.value
  const hl = highlightEl.value
  const gut = gutterEl.value
  if (!ta) return
  if (hl) {
    hl.scrollTop = ta.scrollTop
    hl.scrollLeft = ta.scrollLeft
  }
  if (gut) gut.scrollTop = ta.scrollTop
}

function updateCursor() {
  const ta = textareaEl.value
  const tab = activeTab.value
  if (!ta || !tab) return
  const pos = ta.selectionStart ?? 0
  const before = tab.content.slice(0, pos)
  cursorLine.value = before.split('\n').length
  const lastNl = before.lastIndexOf('\n')
  cursorCol.value = pos - lastNl
}

function onEdit() {
  const tab = activeTab.value
  if (!tab) return
  tab.dirty = tab.content !== tab.savedContent
  updateCursor()
  scheduleReparse()
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Tab') {
    e.preventDefault()
    const ta = textareaEl.value
    if (!ta) return
    const start = ta.selectionStart ?? 0
    const end = ta.selectionEnd ?? 0
    const tab = activeTab.value
    if (!tab) return
    tab.content = `${tab.content.slice(0, start)}  ${tab.content.slice(end)}`
    nextTick(() => {
      ta.selectionStart = ta.selectionEnd = start + 2
      updateCursor()
    })
    onEdit()
    return
  }
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
    e.preventDefault()
    void saveActive()
  }
}

// ─── Outline (live, via parse_text on every transport) ───

const outlineStatus = computed(() => {
  const tab = activeTab.value
  if (!tab) return ''
  if (parsingOutline.value) return 'PARSING…'
  if (!tab.parse) return 'OUTLINE STALE'
  return `${tab.parse.symbols.length} SYMBOLS · ${(tab.parse.engine || 'heuristic').toUpperCase()}`
})

const outlineSymbols = computed<CodeSymbol[]>(() => {
  const q = outlineFilter.value.trim().toLowerCase()
  const syms = activeTab.value?.parse?.symbols ?? []
  if (!q) return syms
  return syms.filter(s => s.name.toLowerCase().includes(q) || (s.detail ?? '').toLowerCase().includes(q))
})

async function reparseActive() {
  const tab = activeTab.value
  if (!tab || tab.readOnly) return
  if (tab.content.length > 256 * 1024) return // huge buffers: manual parse only
  parsingOutline.value = true
  try {
    const res = await invoke<import('@/types').ParseResult>('parse_text', {
      fileName: tab.label,
      content: tab.content,
    })
    if (tab.key === activeKey.value) tab.parse = res
    else {
      const t = tabs.value.find(t => t.key === tab.key)
      if (t) t.parse = res
    }
  } catch {
    // Outline is best-effort; the toast already fired. Editing continues.
  } finally {
    parsingOutline.value = false
  }
}

let reparseTimer = 0
function scheduleReparse() {
  window.clearTimeout(reparseTimer)
  const tab = activeTab.value
  if (!tab || tab.content.length > 256 * 1024) return
  reparseTimer = window.setTimeout(() => {
    void reparseActive()
  }, 1200)
}

function jumpToLine(line: number) {
  const ta = textareaEl.value
  const tab = activeTab.value
  if (!ta || !tab) return
  const lines = tab.content.split('\n')
  const idx = Math.max(0, Math.min(line - 1, lines.length - 1))
  const pos = lines.slice(0, idx).join('\n').length + (idx > 0 ? 1 : 0)
  ta.focus()
  ta.selectionStart = ta.selectionEnd = pos
  updateCursor()
  syncScroll()
}

// ─── Find ───

const findMatches = computed(() => {
  const tab = activeTab.value
  const q = findQuery.value
  if (!tab || !q) return []
  const out: number[] = []
  let i = tab.content.indexOf(q)
  while (i !== -1 && out.length < 500) {
    out.push(i)
    i = tab.content.indexOf(q, i + q.length)
  }
  return out
})
const findCount = computed(() => findMatches.value.length)

function findNext(dir: 1 | -1) {
  const ta = textareaEl.value
  if (!ta || !findMatches.value.length) {
    findIndex.value = -1
    return
  }
  findIndex.value = (findIndex.value + dir + findMatches.value.length) % findMatches.value.length
  const pos = findMatches.value[findIndex.value]
  ta.focus()
  ta.selectionStart = pos
  ta.selectionEnd = pos + findQuery.value.length
  updateCursor()
}

// ─── Save ───

async function saveActive() {
  const tab = activeTab.value
  if (!tab || tab.readOnly || saving.value) return
  saving.value = true
  try {
    if (tab.kind === 'managed') {
      const saved = await store.saveManagedContent(tab.fileId, tab.content)
      if (saved) {
        tab.savedContent = tab.content
        tab.dirty = false
      }
    } else {
      const ok = await store.saveWasmFile(tab.path, tab.content)
      if (ok) {
        tab.savedContent = tab.content
        tab.dirty = false
      }
    }
    if (!tab.dirty) void reparseActive()
  } finally {
    saving.value = false
  }
}

watch(findQuery, () => {
  findIndex.value = -1
})

onBeforeUnmount(() => {
  window.clearTimeout(reparseTimer)
})

void refreshExplorer()

function highlightSyntax(code: string, _lang: string): string {
  const escaped = code
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
  const keywords = new Set([
    'if', 'else', 'for', 'while', 'do', 'switch', 'case', 'break', 'continue',
    'return', 'function', 'class', 'struct', 'enum', 'interface', 'type',
    'var', 'let', 'const', 'import', 'export', 'from', 'def', 'async', 'await',
    'try', 'catch', 'throw', 'new', 'this', 'super', 'extends', 'implements',
    'pub', 'fn', 'mut', 'use', 'mod', 'impl', 'trait', 'where',
    'package', 'void', 'int', 'float', 'double', 'char', 'bool', 'string',
    'null', 'undefined', 'true', 'false', 'static', 'private', 'public',
    'protected', 'readonly', 'abstract', 'virtual', 'override',
  ])
  const strRe = /("(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'|`(?:[^`\\]|\\.)*`)/g
  const commentRe = /(\/\/.*$|\/\*[\s\S]*?\*\/|#.*$)/gm
  let highlighted = escaped
  highlighted = highlighted.replace(commentRe, '<span class="syn-comment">$1</span>')
  highlighted = highlighted.replace(strRe, '<span class="syn-string">$1</span>')
  highlighted = highlighted.replace(/\b(\d+(?:\.\d+)?)\b/g, '<span class="syn-number">$1</span>')
  highlighted = highlighted.replace(/\b([a-zA-Z_]\w*)\b/g, (match) => {
    if (keywords.has(match)) return `<span class="syn-keyword">${match}</span>`
    if (match[0] === match[0].toUpperCase() && match[0] !== match[0].toLowerCase()) {
      return `<span class="syn-type">${match}</span>`
    }
    return match
  })
  return highlighted
}
</script>

<style scoped>
.editor-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #000;
  color: #fff;
  font-family: 'Courier New', monospace;
  font-size: 12px;
  overflow: hidden;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  border-bottom: 2px solid #FFFFFF;
  flex: 0 0 auto;
}

.header-left { display: flex; align-items: center; gap: 8px; }
.header-actions { display: flex; gap: 4px; }
.icon-editor { font-size: 14px; }
.panel-title { font-size: 13px; font-weight: 800; letter-spacing: 1px; margin: 0; }
.transport-tag { font-size: 9px; color: rgba(255,255,255,0.5); }

.engine-badge {
  font-size: 9px;
  font-weight: 700;
  padding: 1px 6px;
  border: 1px solid rgba(255,255,255,0.4);
  color: rgba(255,255,255,0.6);
}
.engine-badge.engine-ts { background: #FFFFFF; color: #000; border-color: #FFFFFF; }

.bw-btn {
  padding: 6px 12px;
  background: #FFFFFF;
  color: #000;
  border: 2px solid #FFFFFF;
  font-family: inherit;
  font-size: 10px;
  font-weight: 700;
  cursor: pointer;
}
.bw-btn:disabled { opacity: 0.4; cursor: default; }
.bw-btn-sm {
  padding: 2px 6px;
  background: transparent;
  border: 1px solid rgba(255,255,255,0.5);
  color: #fff;
  font-family: inherit;
  font-size: 10px;
  font-weight: 700;
  cursor: pointer;
}
.bw-btn-sm:disabled { opacity: 0.4; cursor: default; }
.bw-btn-sm:hover:not(:disabled) { background: #fff; color: #000; }
.ghost-btn {
  background: transparent;
  border: 1px solid rgba(255,255,255,0.3);
  color: rgba(255,255,255,0.7);
  font-family: inherit;
  font-size: 11px;
  padding: 2px 7px;
  cursor: pointer;
}
.bw-input {
  background: #000;
  border: 1px solid rgba(255,255,255,0.5);
  color: #fff;
  font-family: inherit;
  font-size: 11px;
  padding: 4px 6px;
  outline: none;
}
.bw-input:focus { border-color: #fff; }

.editor-body { display: flex; flex: 1 1 auto; min-height: 0; }

.explorer {
  width: 220px;
  flex: 0 0 auto;
  border-right: 1px solid rgba(255,255,255,0.15);
  display: flex;
  flex-direction: column;
  padding: 8px;
  gap: 6px;
  overflow: hidden;
}
.explorer-head { display: flex; align-items: center; justify-content: space-between; }
.section-title { font-size: 10px; letter-spacing: 1px; color: rgba(255,255,255,0.55); margin: 0; }
.explorer-search { width: 100%; box-sizing: border-box; }
.wasm-nav { display: flex; align-items: center; gap: 6px; }
.wasm-path { font-size: 10px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.wasm-new { display: flex; gap: 4px; }
.wasm-new .bw-input { flex: 1; min-width: 0; }
.explorer-list { flex: 1 1 auto; overflow-y: auto; }
.explorer-row {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 3px 4px;
  cursor: pointer;
  font-size: 11px;
}
.explorer-row:hover { background: rgba(255,255,255,0.08); }
.explorer-row.active { background: rgba(255,255,255,0.14); }
.explorer-row.dir { color: #9aedfe; }
.explorer-icon { flex: 0 0 auto; }
.explorer-name { flex: 1 1 auto; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.explorer-sub { font-size: 9px; flex: 0 0 auto; }
.hint { font-size: 9px; margin: 0; line-height: 1.5; }
.empty { font-size: 10px; }

.main { flex: 1 1 auto; display: flex; flex-direction: column; min-width: 0; }

.tabs {
  display: flex;
  gap: 2px;
  padding: 6px 8px 0;
  overflow-x: auto;
  flex: 0 0 auto;
  border-bottom: 1px solid rgba(255,255,255,0.15);
}
.tab {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 6px 4px 8px;
  border: 1px solid rgba(255,255,255,0.2);
  border-bottom: none;
  font-size: 11px;
  cursor: pointer;
  white-space: nowrap;
  color: rgba(255,255,255,0.6);
}
.tab.active { background: rgba(255,255,255,0.1); color: #fff; }
.tab-dot { color: #f3f99d; font-size: 9px; }
.tab-x { background: none; border: none; color: inherit; cursor: pointer; font-size: 12px; padding: 0 2px; }
.tab-x:hover { color: #ff5f56; }

.findbar {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
  border-bottom: 1px solid rgba(255,255,255,0.1);
  flex: 0 0 auto;
}
.find-input { flex: 1; }
.find-count { font-size: 10px; min-width: 40px; text-align: right; }

.empty-editor { padding: 24px; text-align: center; }
.refusal { color: #ff8080; font-size: 11px; }

.edit-wrap { flex: 1 1 auto; display: flex; min-height: 0; overflow: hidden; }
.gutter {
  flex: 0 0 auto;
  width: 44px;
  padding: 8px 6px 8px 0;
  text-align: right;
  color: rgba(255,255,255,0.3);
  font-size: 12px;
  line-height: 1.5;
  overflow: hidden;
  white-space: pre;
  user-select: none;
}
.code-area { flex: 1 1 auto; position: relative; min-width: 0; }
.highlight,
.editor-input {
  margin: 0;
  padding: 8px;
  font-family: 'Courier New', monospace;
  font-size: 12px;
  line-height: 1.5;
  white-space: pre;
  tab-size: 2;
}
.highlight {
  position: absolute;
  inset: 0;
  overflow: hidden;
  pointer-events: none;
  color: #e6e6e6;
}
.highlight code { font-family: inherit; }
.editor-input {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  background: transparent;
  border: none;
  outline: none;
  resize: none;
  color: transparent;
  caret-color: #fff;
  overflow: auto;
}

.statusbar {
  display: flex;
  gap: 14px;
  padding: 4px 10px;
  border-top: 1px solid rgba(255,255,255,0.15);
  font-size: 10px;
  flex: 0 0 auto;
}
.dirty-tag { color: #f3f99d; }
.text-muted { color: rgba(255,255,255,0.5) !important; }

.outline {
  flex: 0 0 auto;
  max-height: 220px;
  display: flex;
  flex-direction: column;
  border-top: 1px solid rgba(255,255,255,0.15);
  padding: 6px 8px;
  gap: 6px;
}
.outline-head { display: flex; align-items: center; justify-content: space-between; }
.symbol-tree { overflow-y: auto; border: 1px solid rgba(255,255,255,0.15); }
.symbol-row {
  display: flex;
  gap: 6px;
  padding: 2px 6px;
  font-size: 10px;
  cursor: pointer;
}
.symbol-row:hover { background: rgba(255,255,255,0.1); }
.symbol-kind { font-size: 8px; color: rgba(255,255,255,0.6); text-transform: uppercase; min-width: 44px; }
.symbol-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.symbol-lines { font-size: 9px; }
</style>

<style>
.syn-keyword { color: #FFFFFF; font-weight: 700; }
.syn-string { color: rgba(255,255,255,0.6); }
.syn-comment { color: rgba(255,255,255,0.3); font-style: italic; }
.syn-number { color: #FFFFFF; }
.syn-type { color: #FFFFFF; text-decoration: underline; }
</style>
