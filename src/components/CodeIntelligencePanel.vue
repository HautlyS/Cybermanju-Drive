<template>
  <div class="code-panel">
    <div class="panel-header">
      <div class="header-left">
        <span class="icon-code"><AppIcon name="solar:code-2-bold" /></span>
        <h2 class="panel-title">CODE INTELLIGENCE</h2>
        <span
          v-if="parseResult"
          class="engine-badge"
          :class="{ 'engine-ts': isTreeSitter, 'engine-heu': !isTreeSitter }"
          :title="engineTitle"
        >{{ engineLabel }}</span>
      </div>
      <div class="header-actions">
        <button
          v-if="parseResult"
          class="bw-btn-sm"
          @click="showAst = !showAst"
          title="TOGGLE OUTLINE VIEW"
        ><AppIcon :name="showAst ? 'solar:folder-tree-bold' : 'solar:git-fork-bold'" :size="13" /> {{ showAst ? 'TREE' : 'AST' }}</button>
        <button
          v-if="parsedContent"
          class="bw-btn-sm"
          @click="showSource = !showSource"
          title="TOGGLE SOURCE VIEW"
        ><AppIcon name="solar:file-code-bold" :size="13" /> SRC</button>
      </div>
    </div>

    <div class="section">
      <h3 class="section-title"><AppIcon name="solar:file-code-bold" :size="13" /> WHAT TO PARSE</h3>
      <div class="mode-tabs" role="tablist" aria-label="Parse source">
        <button class="mode-tab" :class="{ on: mode === 'file' }" @click="mode = 'file'">FILE</button>
        <button class="mode-tab" :class="{ on: mode === 'paste' }" @click="mode = 'paste'">PASTE</button>
        <button v-if="isDesktop" class="mode-tab" :class="{ on: mode === 'path' }" @click="mode = 'path'">PATH</button>
      </div>

      <div v-if="mode === 'file'" class="mode-body">
        <div v-if="selectedFile" class="selected-file">
          <span>{{ selectedFile.name }}</span>
          <span v-if="!fileContent" class="text-muted warn"> — NO LOADED TEXT (open it first, or use PASTE)</span>
        </div>
        <p v-else class="text-muted">SELECT A FILE IN THE BROWSER FIRST</p>
        <button class="bw-btn" style="margin-top:6px;" :disabled="!fileContent || parsing" @click="handleParseFile">
          <AppIcon :name="parsing ? 'solar:loader-bold' : 'solar:play-bold'" :size="13" /> {{ parsing ? 'PARSING…' : 'PARSE SELECTED FILE' }}
        </button>
      </div>

      <div v-if="mode === 'paste'" class="mode-body">
        <label class="field-label">FILENAME (DRIVES LANGUAGE DETECTION)
          <input v-model="pastedName" class="bw-input" placeholder="snippet.rs" spellcheck="false" />
        </label>
        <label class="field-label">SOURCE
          <textarea
            v-model="pastedContent"
            class="paste-area"
            placeholder="fn main() { … }"
            spellcheck="false"
            rows="6"
          />
        </label>
        <div class="row-between">
          <span class="text-muted counter">{{ pastedContent.length }} CHARS / 1 MiB LIMIT</span>
          <button class="bw-btn" :disabled="!pastedContent.trim() || parsing" @click="handleParsePaste">
            <AppIcon :name="parsing ? 'solar:loader-bold' : 'solar:play-bold'" :size="13" /> {{ parsing ? 'PARSING…' : 'PARSE TEXT' }}
          </button>
        </div>
      </div>

      <div v-if="mode === 'path' && isDesktop" class="mode-body">
        <label class="field-label">DESKTOP FILE PATH
          <input v-model="pathInput" class="bw-input" placeholder="/home/user/main.rs" spellcheck="false" @keyup.enter="handleParsePath" />
        </label>
        <button class="bw-btn" style="margin-top:6px;" :disabled="!pathInput.trim() || parsing" @click="handleParsePath">
          <AppIcon :name="parsing ? 'solar:loader-bold' : 'solar:play-bold'" :size="13" /> {{ parsing ? 'PARSING…' : 'PARSE FILE BY PATH' }}
        </button>
        <p class="text-muted hint">DESKTOP ONLY — reads from disk via Tauri.</p>
      </div>

      <p v-if="localError" class="error-line">{{ localError }}</p>
    </div>

    <div class="section" v-if="parseResult">
      <h3 class="section-title"><AppIcon name="solar:database-bold" :size="13" /> PARSE RESULT</h3>
      <div class="parse-meta">
        <div class="meta-row"><span class="meta-key text-muted">FILE</span><span class="meta-value truncate">{{ parseResult.filePath }}</span></div>
        <div class="meta-row"><span class="meta-key text-muted">LANGUAGE</span><span class="meta-value language-badge">{{ parseResult.language }}</span></div>
        <div class="meta-row">
          <span class="meta-key text-muted">ENGINE</span>
          <span class="meta-value" :class="{ 'engine-ts-text': isTreeSitter }">{{ engineLabel }} — {{ engineHint }}</span>
        </div>
        <div class="meta-row"><span class="meta-key text-muted">LINES</span><span class="meta-value">{{ parseResult.totalLines }}</span></div>
        <div class="meta-row"><span class="meta-key text-muted">SYMBOLS</span><span class="meta-value">{{ parseResult.symbols.length }}</span></div>
        <div class="meta-row"><span class="meta-key text-muted">PARSE TIME</span><span class="meta-value">{{ parseResult.parseTimeMs }}ms</span></div>
      </div>
      <div v-if="kindCounts.length" class="kind-summary">
        <span v-for="kc in kindCounts" :key="kc.kind" class="kind-pill">{{ kc.kind }}:{{ kc.count }}</span>
      </div>
    </div>

    <div class="section" v-if="parseResult && showAst">
      <h3 class="section-title"><AppIcon name="solar:folder-tree-bold" :size="13" /> SYMBOL OUTLINE</h3>
      <div class="outline-tools">
        <input
          v-model="filter"
          class="bw-input outline-search"
          placeholder="FILTER SYMBOLS…"
          spellcheck="false"
          aria-label="FILTER SYMBOLS"
        />
        <div class="kind-chips">
          <button class="chip" :class="{ on: kindFilter === '' }" @click="kindFilter = ''">ALL</button>
          <button
            v-for="kc in kindCounts"
            :key="kc.kind"
            class="chip"
            :class="{ on: kindFilter === kc.kind }"
            @click="kindFilter = kindFilter === kc.kind ? '' : kc.kind"
          >{{ kc.kind }} ({{ kc.count }})</button>
        </div>
      </div>
      <div v-if="filteredSymbols.length" class="symbol-tree">
        <div
          v-for="sym in filteredSymbols"
          :key="sym.name + sym.startLine"
          class="symbol-row"
          :class="{ active: sym.startLine === activeLine }"
          @click="jumpTo(sym.startLine)"
          :title="sym.detail || sym.name"
        >
          <span class="symbol-kind">{{ sym.kind }}</span>
          <span class="symbol-name">{{ sym.name }}</span>
          <span class="symbol-lines text-muted">:{{ sym.startLine }}{{ sym.endLine !== sym.startLine ? '–' + sym.endLine : '' }}</span>
        </div>
      </div>
      <p v-else class="text-muted">{{ parseResult.symbols.length ? 'NO SYMBOLS MATCH THE FILTER' : 'NO SYMBOLS FOUND — TRY ANOTHER FILE' }}</p>
    </div>

    <div class="section" v-if="parsedContent && showSource">
      <h3 class="section-title"><AppIcon name="solar:file-code-bold" :size="13" /> SYNTAX VIEW</h3>
      <div class="source-view">
        <div
          v-for="(line, li) in sourceLines"
          :key="li"
          class="source-line"
          :class="{ 'goto-highlight': li + 1 === activeLine }"
          :ref="(el) => { if (li + 1 === activeLine && el) highlightLine(el as HTMLElement) }"
        >
          <span class="source-ln">{{ li + 1 }}</span>
          <span class="source-code" v-html="highlightSyntax(line, parseResult?.language ?? '')"></span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import AppIcon from '@/components/AppIcon.vue'
import { ref, computed, nextTick } from 'vue'
import { useAppStore } from '@/stores/app'
import { isTauri } from '@/composables/useTauri'
import type { CodeSymbol } from '@/types'

const store = useAppStore()
const selectedFile = computed(() => store.selectedFile)
const parseResult = computed(() => store.parseResult)
const isDesktop = isTauri()

const mode = ref<'file' | 'paste' | 'path'>('file')
const showAst = ref(true)
const showSource = ref(false)
const parsing = ref(false)
const localError = ref('')
const filter = ref('')
const kindFilter = ref('')
const activeLine = ref(0)
const pastedName = ref('snippet.rs')
const pastedContent = ref('')
const pathInput = ref('')

/** Source text of the last successful text parse — drives the preview. */
const parsedContent = ref('')

const fileContent = computed(() => selectedFile.value?.contentText ?? '')

const isTreeSitter = computed(() => parseResult.value?.engine === 'tree-sitter')
const engineLabel = computed(() => {
  if (!parseResult.value) return ''
  return isTreeSitter.value ? 'TREE-SITTER' : 'HEURISTIC'
})
const engineTitle = computed(() =>
  isTreeSitter.value
    ? 'Real grammar parse (desktop)'
    : 'Regex fallback — same shape, less precise (web/Pages or unbundled language)',
)
const engineHint = computed(() =>
  isTreeSitter.value ? 'real grammar' : 'regex fallback',
)

const kindCounts = computed(() => {
  const counts = new Map<string, number>()
  for (const s of parseResult.value?.symbols ?? []) {
    counts.set(s.kind, (counts.get(s.kind) ?? 0) + 1)
  }
  return [...counts.entries()]
    .map(([kind, count]) => ({ kind, count }))
    .sort((a, b) => b.count - a.count || a.kind.localeCompare(b.kind))
})

const filteredSymbols = computed(() => {
  const q = filter.value.trim().toLowerCase()
  return (parseResult.value?.symbols ?? []).filter((s: CodeSymbol) => {
    if (kindFilter.value && s.kind !== kindFilter.value) return false
    if (!q) return true
    return (
      s.name.toLowerCase().includes(q) ||
      (s.detail ?? '').toLowerCase().includes(q)
    )
  })
})

const sourceLines = computed(() => (parsedContent.value ? parsedContent.value.split('\n') : []))

async function handleParseFile() {
  if (!fileContent.value || !selectedFile.value) return
  localError.value = ''
  parsing.value = true
  try {
    const res = await store.parseCodeText(selectedFile.value.name, fileContent.value)
    if (res) {
      parsedContent.value = fileContent.value
      activeLine.value = 0
      showSource.value = true
    } else {
      localError.value = 'Parse returned nothing — see notification.'
    }
  } finally {
    parsing.value = false
  }
}

async function handleParsePaste() {
  if (!pastedContent.value.trim()) return
  localError.value = ''
  parsing.value = true
  try {
    const name = pastedName.value.trim() || 'snippet.txt'
    const res = await store.parseCodeText(name, pastedContent.value)
    if (res) {
      parsedContent.value = pastedContent.value
      activeLine.value = 0
      filter.value = ''
      kindFilter.value = ''
      showSource.value = true
    } else {
      localError.value = 'Parse returned nothing — see notification.'
    }
  } finally {
    parsing.value = false
  }
}

async function handleParsePath() {
  if (!pathInput.value.trim()) return
  localError.value = ''
  parsing.value = true
  try {
    await store.parseFileCode(pathInput.value.trim())
    // Path parses have no local bytes: keep the previous preview, if any.
    showSource.value = parsedContent.value !== ''
  } finally {
    parsing.value = false
  }
}

function jumpTo(line: number) {
  activeLine.value = line
  showSource.value = true
}

function highlightLine(el: HTMLElement) {
  nextTick(() => el.scrollIntoView({ block: 'center', behavior: 'smooth' }))
}

function highlightSyntax(code: string, _lang: string): string {
  const escaped = code.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
  const keywords = new Set([
    'if', 'else', 'for', 'while', 'do', 'switch', 'case', 'break', 'continue',
    'return', 'function', 'class', 'struct', 'enum', 'interface', 'type',
    'var', 'let', 'const', 'import', 'export', 'from', 'def', 'async', 'await',
    'try', 'catch', 'throw', 'new', 'this', 'super', 'extends', 'implements',
    'pub', 'fn', 'let', 'mut', 'const', 'use', 'mod', 'impl', 'trait', 'where',
    'package', 'void', 'int', 'float', 'double', 'char', 'bool', 'string',
    'null', 'undefined', 'true', 'false', 'static', 'private', 'public',
    'protected', 'readonly', 'abstract', 'virtual', 'override',
  ])
  const strRe = /("(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'|`(?:[^`\\]|\\.)*`)/g
  const commentRe = /(\/\/.*$|\/\*[\s\S]*?\*\/)/g
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
.code-panel {
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
.header-actions { display: flex; gap: 4px; }
.icon-code { font-size: 16px; }
.panel-title { font-size: 14px; font-weight: 800; letter-spacing: 1px; margin: 0; }

.engine-badge {
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 1px;
  padding: 1px 6px;
  border: 2px solid #FFFFFF;
}
.engine-ts { background: #FFFFFF; color: #000; }
.engine-heu { background: transparent; color: rgba(255,255,255,0.6); border-color: rgba(255,255,255,0.4); }
.engine-ts-text { color: #FFFFFF; }

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

.mode-tabs { display: flex; gap: 4px; margin-bottom: 8px; }
.mode-tab {
  padding: 4px 10px;
  background: transparent;
  border: 2px solid rgba(255,255,255,0.4);
  color: rgba(255,255,255,0.6);
  font-family: 'Courier New', monospace;
  font-size: 10px;
  font-weight: 700;
  cursor: pointer;
}
.mode-tab.on { background: #FFFFFF; color: #000; border-color: #FFFFFF; }
.mode-body { display: flex; flex-direction: column; gap: 6px; }

.field-label { display: flex; flex-direction: column; gap: 4px; font-size: 10px; color: rgba(255,255,255,0.6); }
.bw-input {
  background: #000;
  border: 2px solid #FFFFFF;
  color: #FFFFFF;
  font-family: 'Courier New', monospace;
  font-size: 11px;
  padding: 4px 8px;
}
.paste-area {
  background: #000;
  border: 2px solid #FFFFFF;
  color: #FFFFFF;
  font-family: 'Courier New', monospace;
  font-size: 11px;
  padding: 6px 8px;
  resize: vertical;
  min-height: 90px;
}
.row-between { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
.counter { font-size: 9px; }
.hint { font-size: 9px; margin: 4px 0 0; }
.warn { font-size: 10px; }
.error-line { font-size: 10px; color: #ff8080; margin: 6px 0 0; }

.bw-btn {
  padding: 6px 12px;
  background: #FFFFFF;
  color: #000;
  border: 2px solid #FFFFFF;
  font-family: 'Courier New', monospace;
  font-size: 10px;
  font-weight: 700;
  cursor: pointer;
  align-self: flex-start;
}

.bw-btn:hover:not(:disabled) { background: #000; color: #FFFFFF; }
.bw-btn:disabled { opacity: 0.4; cursor: default; }

.bw-btn-sm {
  padding: 2px 6px;
  background: transparent;
  border: 2px solid #FFFFFF;
  color: #FFFFFF;
  font-family: 'Courier New', monospace;
  font-size: 9px;
  font-weight: 700;
  cursor: pointer;
}

.bw-btn-sm:hover { background: #FFFFFF; color: #000; }

.selected-file {
  font-size: 12px;
  background: rgba(255,255,255,0.05);
  padding: 6px 8px;
  border: 2px solid rgba(255,255,255,0.3);
}

.parse-meta {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-bottom: 12px;
}

.meta-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
}

.meta-key { font-size: 10px; }
.meta-value { font-size: 10px; font-family: 'Courier New', monospace; font-weight: 700; }

.language-badge {
  border: 1px solid #FFFFFF;
  padding: 0 4px;
  font-size: 9px;
}

.truncate { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 60%; }

.kind-summary { display: flex; flex-wrap: wrap; gap: 4px; }
.kind-pill {
  font-size: 9px;
  border: 1px solid rgba(255,255,255,0.4);
  color: rgba(255,255,255,0.7);
  padding: 0 5px;
}

.outline-tools { display: flex; flex-direction: column; gap: 6px; margin-bottom: 8px; }
.outline-search { width: 100%; }
.kind-chips { display: flex; flex-wrap: wrap; gap: 4px; }
.chip {
  padding: 1px 6px;
  background: transparent;
  border: 1px solid rgba(255,255,255,0.4);
  color: rgba(255,255,255,0.6);
  font-family: 'Courier New', monospace;
  font-size: 9px;
  cursor: pointer;
}
.chip.on { background: #FFFFFF; color: #000; border-color: #FFFFFF; }

.symbol-tree {
  border: 2px solid rgba(255,255,255,0.3);
  overflow: hidden;
  max-height: 320px;
  overflow-y: auto;
}

.symbol-row {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 2px 6px;
  font-size: 10px;
  border-bottom: 1px solid rgba(255,255,255,0.1);
  cursor: pointer;
}

.symbol-row:hover {
  background: rgba(255,255,255,0.1);
}

.symbol-row.active {
  background: rgba(255,255,255,0.15);
  border-left: 3px solid #FFFFFF;
}

.symbol-kind {
  font-size: 8px;
  color: rgba(255,255,255,0.6);
  text-transform: uppercase;
  flex-shrink: 0;
  min-width: 40px;
}

.symbol-name { color: #FFFFFF; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex: 1; }
.symbol-lines { flex-shrink: 0; font-size: 9px; }

.source-view {
  border: 2px solid rgba(255,255,255,0.3);
  font-family: 'Courier New', monospace;
  font-size: 10px;
  max-height: 400px;
  overflow-y: auto;
  line-height: 1.5;
}

.source-line {
  display: flex;
  padding: 0 4px;
}

.source-line:hover {
  background: rgba(255,255,255,0.05);
}

.source-line.goto-highlight {
  background: rgba(255,255,255,0.15);
  border-left: 3px solid #FFFFFF;
}

.source-ln {
  flex-shrink: 0;
  width: 32px;
  text-align: right;
  padding-right: 8px;
  color: rgba(255,255,255,0.3);
  user-select: none;
}

.source-code {
  flex: 1;
  white-space: pre;
  overflow-x: auto;
}

.text-muted { color: rgba(255,255,255,0.5) !important; }
</style>

<style>
.syn-keyword { color: #FFFFFF; font-weight: 700; }
.syn-string { color: rgba(255,255,255,0.6); }
.syn-comment { color: rgba(255,255,255,0.3); font-style: italic; }
.syn-number { color: #FFFFFF; }
.syn-type { color: #FFFFFF; text-decoration: underline; }
</style>
