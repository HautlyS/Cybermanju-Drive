<template>
  <div class="code-panel">
    <div class="panel-header">
      <div class="header-left">
        <span class="icon-code">[T]</span>
        <h2 class="panel-title">CODE INTELLIGENCE</h2>
      </div>
    </div>

    <div class="section">
      <h3 class="section-title">[PARSE] ANALYZE FILE</h3>
      <div v-if="selectedFile" class="selected-file">
        <span>{{ selectedFile.name }}</span>
        <button class="bw-btn" style="margin-top:6px;" @click="handleParse">[PARSE WITH TREE-SITTER]</button>
      </div>
      <p v-else class="text-muted">SELECT A TEXT/CODE FILE TO PARSE</p>
    </div>

    <div class="section" v-if="parseResult">
      <h3 class="section-title">[DATA] PARSE RESULT</h3>
      <div class="parse-meta">
        <div class="meta-row"><span class="meta-key text-muted">LANGUAGE</span><span class="meta-value">{{ parseResult.language }}</span></div>
        <div class="meta-row"><span class="meta-key text-muted">LINES</span><span class="meta-value">{{ parseResult.totalLines }}</span></div>
        <div class="meta-row"><span class="meta-key text-muted">SYMBOLS</span><span class="meta-value">{{ parseResult.symbols.length }}</span></div>
        <div class="meta-row"><span class="meta-key text-muted">PARSE TIME</span><span class="meta-value">{{ parseResult.parseTimeMs }}ms</span></div>
      </div>

      <div class="symbol-tree">
        <div v-for="sym in parseResult.symbols" :key="sym.name + sym.startLine" class="symbol-row">
          <span class="symbol-kind">{{ sym.kind }}</span>
          <span class="symbol-name">{{ sym.name }}</span>
          <span class="symbol-lines text-muted">:{{ sym.startLine }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useAppStore } from '@/stores/app'

const store = useAppStore()
const selectedFile = computed(() => store.selectedFile)
const parseResult = computed(() => store.parseResult)

async function handleParse() {
  if (!store.selectedFile?.path) return
  await store.parseFileCode(store.selectedFile.path)
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
.icon-code { font-size: 16px; }
.panel-title { font-size: 14px; font-weight: 800; letter-spacing: 1px; margin: 0; }

.section { margin-bottom: 16px; }

.section-title {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 1px;
  color: rgba(255,255,255,0.6);
  margin: 0 0 8px;
}

.bw-btn {
  padding: 6px 12px;
  background: #FFFFFF;
  color: #000;
  border: 2px solid #FFFFFF;
  font-family: 'Courier New', monospace;
  font-size: 10px;
  font-weight: 700;
  cursor: pointer;
}

.bw-btn:hover { background: #000; color: #FFFFFF; }

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
}

.meta-key { font-size: 10px; }
.meta-value { font-size: 10px; font-family: 'Courier New', monospace; font-weight: 700; }

.symbol-tree {
  border: 2px solid rgba(255,255,255,0.3);
  overflow: hidden;
}

.symbol-row {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 2px 6px;
  font-size: 10px;
  border-bottom: 1px solid rgba(255,255,255,0.1);
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

.text-muted { color: rgba(255,255,255,0.5) !important; }
</style>
