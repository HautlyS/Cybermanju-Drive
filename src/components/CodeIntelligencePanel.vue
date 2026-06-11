<template>
  <div class="code-intel-panel">
    <!-- Header -->
    <div class="panel-header">
      <div class="header-left">
        <Code :size="22" class="icon-code" />
        <h2 class="panel-title">Code Intelligence</h2>
      </div>
      <button class="close-btn" @click="$emit('close')">
        <span class="close-x">✕</span>
      </button>
    </div>

    <!-- Parse Result -->
    <template v-if="parseResult">
      <!-- File Info -->
      <div class="file-info-card">
        <div class="info-row">
          <div class="info-item">
            <FileCode :size="14" class="info-icon" />
            <span class="info-label">Language</span>
            <span class="language-badge">{{ parseResult.language }}</span>
          </div>
          <div class="info-item">
            <Hash :size="14" class="info-icon" />
            <span class="info-label">Lines</span>
            <span class="mono">{{ parseResult.totalLines.toLocaleString() }}</span>
          </div>
          <div class="info-item">
            <Gauge :size="14" class="info-icon" />
            <span class="info-label">Parse</span>
            <span class="mono">{{ parseResult.parseTimeMs.toFixed(1) }} ms</span>
          </div>
        </div>
      </div>

      <!-- Symbol Tree -->
      <div class="section" v-if="parseResult.symbols.length > 0">
        <h3 class="section-title">
          <GitBranch :size="16" />
          Symbol Tree ({{ parseResult.symbols.length }})
        </h3>
        <div class="symbol-tree">
          <SymbolTreeNode
            v-for="sym in parseResult.symbols"
            :key="sym.name + sym.startLine"
            :symbol="sym"
            :depth="0"
          />
        </div>
      </div>

      <!-- Empty Symbols -->
      <div class="section" v-else>
        <div class="no-symbols">
          <Braces :size="24" class="no-sym-icon" />
          <p>No symbols found in this file.</p>
        </div>
      </div>
    </template>

    <!-- Empty State -->
    <div class="empty-state" v-if="!parseResult">
      <div class="empty-visual">
        <Code :size="40" class="empty-icon" />
        <Type :size="24" class="empty-icon-sub" />
      </div>
      <p>Select a code file to see its structure. Supports 200+ languages via Tree-sitter.</p>
    </div>

    <!-- Status Footer -->
    <div class="status-footer">
      <span>🌳 Powered by tree-sitter (Rust bindings) • Arborium grammar distribution</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, defineComponent, h } from 'vue'
import { useAppStore } from '@/stores/app'
import type { CodeSymbol } from '@/types'
import {
  Code,
  Hash,
  Braces,
  FileCode,
  GitBranch,
  Type,
  Gauge,
} from 'lucide-vue-next'

const store = useAppStore()

const emit = defineEmits<{
  close: []
}>()

const parseResult = computed(() => store.parseResult)

// ── Symbol icon/color helpers ─────────────────────────────
function getSymbolIcon(kind: string): string {
  const icons: Record<string, string> = {
    function: '⬡', fn: '⬡', method: '⬡',
    class: '◈', struct: '◈',
    interface: '◇', type_alias: '◇', type: '◇', trait: '◇',
    variable: '●', const: '●', let: '●', property: '◆', field: '◆',
    impl: '⚙', constructor: '🔧',
    module: '📦', enum: '◆', macro: '⚡', import: '📥',
  }
  return icons[kind] || '●'
}

function getSymbolColor(kind: string): string {
  const colors: Record<string, string> = {
    function: '#FFB800', fn: '#FFB800', method: '#FFB800',
    class: '#00D4FF', struct: '#00D4FF',
    interface: '#A855F7', type_alias: '#A855F7', type: '#A855F7', trait: '#A855F7',
    variable: '#9CA3AF', const: '#FACC15', let: '#9CA3AF', property: '#9CA3AF', field: '#9CA3AF',
    impl: '#FF6B2B', constructor: '#FF6B2B',
    module: '#1E3A8A', enum: '#FF2D6F', macro: '#00FF41', import: '#6B7280',
  }
  return colors[kind] || '#9CA3AF'
}

// ── Recursive SymbolTreeNode (render-function component) ───
const SymbolTreeNode = defineComponent({
  name: 'SymbolTreeNode',
  props: {
    symbol: { type: Object as () => CodeSymbol, required: true },
    depth: { type: Number, required: true },
  },
  setup(props) {
    function handleClick() {
      console.log('Jump to', props.symbol.name, 'line', props.symbol.startLine)
    }

    return () => h('div', { class: 'symbol-node-wrapper' }, [
      h('div', {
        class: 'symbol-node',
        style: { paddingLeft: (props.depth * 16 + 8) + 'px' },
        onClick: handleClick,
      }, [
        h('span', {
          class: 'sym-icon',
          style: { color: getSymbolColor(props.symbol.kind) },
        }, getSymbolIcon(props.symbol.kind)),
        h('span', { class: 'sym-name' }, props.symbol.name),
        h('span', {
          class: 'sym-kind',
          style: { color: getSymbolColor(props.symbol.kind), borderColor: getSymbolColor(props.symbol.kind) + '44' },
        }, props.symbol.kind),
        h('span', { class: 'sym-lines' }, `L${props.symbol.startLine}–${props.symbol.endLine}`),
        props.symbol.children && props.symbol.children.length > 0
          ? h('span', { class: 'sym-toggle' }, '▾')
          : null,
      ]),
      ...(props.symbol.children || []).map(child =>
        h(SymbolTreeNode, {
          symbol: child,
          depth: props.depth + 1,
          key: child.name + child.startLine,
        })
      ),
    ])
  },
})
</script>

<style scoped>
.code-intel-panel {
  width: 400px;
  height: 100%;
  background: var(--cyber-bg-panel, #12121a);
  border-left: 3px solid #000;
  box-shadow: -4px 0 0 0 #000;
  overflow-y: auto;
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 20px;
  font-family: 'Inter', system-ui, sans-serif;
  color: #F5F5F4;
}

.code-intel-panel::-webkit-scrollbar {
  width: 6px;
}
.code-intel-panel::-webkit-scrollbar-track {
  background: #0a0a0f;
}
.code-intel-panel::-webkit-scrollbar-thumb {
  background: #333;
  border-radius: 3px;
}

/* Header */
.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-bottom: 12px;
  border-bottom: 3px solid #000;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 10px;
}

.icon-code {
  color: #00FF41;
  filter: drop-shadow(0 0 6px #00FF41);
}

.panel-title {
  font-size: 18px;
  font-weight: 800;
  text-transform: uppercase;
  letter-spacing: 2px;
  color: #00FF41;
  text-shadow: 0 0 10px #00FF41, 0 0 20px rgba(0, 255, 65, 0.3);
  margin: 0;
}

.close-btn {
  background: none;
  border: 2px solid #333;
  color: #9CA3AF;
  cursor: pointer;
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
  transition: all 0.15s;
}
.close-btn:hover {
  border-color: #FF2D6F;
  color: #FF2D6F;
}

/* File Info Card */
.file-info-card {
  background: #1a1a2e;
  border: 3px solid #000;
  box-shadow: 3px 3px 0 0 #000;
  padding: 12px 14px;
}

.info-row {
  display: flex;
  flex-wrap: wrap;
  gap: 16px;
}

.info-item {
  display: flex;
  align-items: center;
  gap: 6px;
}

.info-icon {
  color: #00FF41;
  opacity: 0.6;
}

.info-label {
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: #6B7280;
}

.language-badge {
  font-size: 11px;
  font-weight: 700;
  color: #00FF41;
  background: rgba(0, 255, 65, 0.1);
  border: 1px solid rgba(0, 255, 65, 0.3);
  padding: 2px 8px;
  border-radius: 2px;
  text-transform: capitalize;
}

/* Sections */
.section {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.section-title {
  font-size: 13px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 1.5px;
  color: #9CA3AF;
  margin: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  padding-bottom: 6px;
  border-bottom: 2px solid #1a1a2e;
}

/* Symbol Tree */
.symbol-tree {
  display: flex;
  flex-direction: column;
  background: #1a1a2e;
  border: 3px solid #000;
  box-shadow: 3px 3px 0 0 #000;
  overflow: hidden;
}

.symbol-node-wrapper {
  display: flex;
  flex-direction: column;
}

.symbol-node {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
  cursor: pointer;
  transition: background 0.1s;
  border-bottom: 1px solid rgba(255, 255, 255, 0.03);
  min-height: 32px;
}

.symbol-node:hover {
  background: rgba(0, 255, 65, 0.04);
}

.sym-icon {
  font-size: 14px;
  line-height: 1;
  flex-shrink: 0;
  width: 18px;
  text-align: center;
}

.sym-name {
  font-size: 12px;
  font-weight: 600;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  color: #F5F5F4;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sym-kind {
  font-size: 9px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  padding: 1px 5px;
  border: 1px solid;
  border-radius: 2px;
  flex-shrink: 0;
}

.sym-lines {
  font-size: 10px;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  color: #4B5563;
  margin-left: auto;
  flex-shrink: 0;
}

.sym-toggle {
  font-size: 10px;
  color: #4B5563;
  flex-shrink: 0;
}

/* No Symbols */
.no-symbols {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 20px;
}

.no-sym-icon {
  color: #333;
}

.no-symbols p {
  font-size: 12px;
  color: #6B7280;
  margin: 0;
}

/* Empty State */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
  padding: 50px 20px;
  text-align: center;
}

.empty-visual {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
}

.empty-icon {
  color: #1a1a2e;
}

.empty-icon-sub {
  color: #252540;
}

.empty-state p {
  font-size: 13px;
  color: #6B7280;
  margin: 0;
  line-height: 1.6;
  max-width: 300px;
}

/* Footer */
.status-footer {
  margin-top: auto;
  padding-top: 12px;
  border-top: 2px solid #1a1a2e;
  font-size: 10px;
  color: #4B5563;
  text-align: center;
  letter-spacing: 0.5px;
}

/* Utility */
.mono {
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 12px;
}
</style>
