<template>
  <div class="compression-panel">
    <div class="panel-header">
      <div class="header-left">
        <span class="icon-compress"><AppIcon name="solar:archive-bold" /></span>
        <h2 class="panel-title">COMPRESSION ENGINE</h2>
      </div>
      <button class="close-btn" @click="$emit('close')" aria-label="CLOSE"><AppIcon name="solar:close-bold" :size="13" /></button>
    </div>

    <div class="section">
      <h3 class="section-title"><AppIcon name="solar:archive-bold" :size="13" /> COMPRESSION ALGORITHMS</h3>
      <div class="algo-list">
        <button
          v-for="(info, type) in COMPRESSION_INFO"
          :key="type"
          class="algo-btn"
          :class="{ selected: selectedAlgo === type }"
          :disabled="webLocked || !layerCapable(type as CompressionType)"
          :title="webLocked || !layerCapable(type as CompressionType) ? 'Not available in this build' : `Compress with ${info.name}`"
          @click="selectedAlgo = type as CompressionType"
        >
          <div class="algo-header">
            <span class="algo-name">{{ info.name }}</span>
            <span class="algo-speed">{{ info.speed }}</span>
          </div>
          <span class="algo-desc text-muted">{{ info.description }}</span>
        </button>
      </div>
    </div>

    <div class="web-note" :class="{ info: !webLocked }">{{ compressNote }}</div>

    <div class="section" v-if="selectedFile">
      <h3 class="section-title"><AppIcon name="solar:file-bold" :size="13" /> SELECTED FILE</h3>
      <p class="selected-file-name">{{ selectedFile.name }}</p>
      <button class="compress-btn" :disabled="webLocked" :title="webLocked ? 'Needs the desktop app or offline build' : 'Compress file'" @click="handleCompress"><AppIcon name="solar:archive-bold" :size="14" /> COMPRESS</button>
    </div>

    <div class="section" v-if="compressionStats">
      <h3 class="section-title"><AppIcon name="solar:chart-bold" :size="13" /> RESULTS</h3>
      <div class="stats-card">
        <div class="stat-row">
          <span class="stat-key text-muted">ORIGINAL</span>
          <span class="stat-value">{{ humanBytes(compressionStats.originalSize) }}</span>
        </div>
        <div class="stat-row">
          <span class="stat-key text-muted">COMPRESSED</span>
          <span class="stat-value">{{ humanBytes(compressionStats.compressedSize) }}</span>
        </div>
        <div class="stat-row">
          <span class="stat-key text-muted">RATIO</span>
          <span class="stat-value">{{ (compressionStats.ratio * 100).toFixed(1) }}%</span>
        </div>
        <div class="stat-row">
          <span class="stat-key text-muted">DURATION</span>
          <span class="stat-value">{{ compressionStats.durationMs }}ms</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import AppIcon from '@/components/AppIcon.vue'
import { ref, computed } from 'vue'
import { useAppStore } from '@/stores/app'
import { humanBytes } from '@/utils/format'
import { isStaticHost } from '@/composables/useTauri'
import { compressionCapable } from '@/composables/useWasmCrypto'
import type { CompressionType } from '@/types'
import { COMPRESSION_INFO } from '@/types'

const store = useAppStore()
const emit = defineEmits<{ close: [] }>()

// The wasm pack ships lz4 + brotli; zstd (and therefore triple) stays in the
// desktop app. The dashboard build has no compression endpoint at all.
const staticWasm = isStaticHost()
const webLocked = computed(() => !staticWasm)
const compressNote = computed(() => {
  if (webLocked) {
    return 'COMPRESSION NEEDS THE TAURI DESKTOP APP OR THE OFFLINE BROWSER BUILD — THIS DASHBOARD BUILD SERVES NO COMPRESSION ENDPOINT.'
  }
  return 'LZ4 + BROTLI RUN IN THE WASM PACK AND WRITE INTO .CYBERMANJU. ZSTD / TRIPLE NEED THE DESKTOP APP (THE WASM PACK HAS NO ZSTD).'
})

function layerCapable(type: CompressionType): boolean {
  return !staticWasm || compressionCapable(type)
}

const selectedFile = computed(() => store.selectedFile)
const compressionStats = computed(() => store.compressionStats)
const selectedAlgo = ref<CompressionType>(staticWasm ? 'lz4' : 'zstd')


async function handleCompress() {
  if (!store.selectedFileId) return
  await store.compressFile(store.selectedFileId, selectedAlgo.value)
}
</script>

<style scoped>
.web-note {
  border: 1px dashed var(--ui-warning);
  color: var(--ui-warning);
  font-size: 9px;
  line-height: 1.5;
  padding: 8px 10px;
  letter-spacing: 0.3px;
}
.web-note.info {
  border-color: color-mix(in srgb, var(--ui-accent) 60%, transparent);
  color: var(--ui-accent);
}

.algo-btn:disabled,
.compress-btn:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}
.compression-panel {
  width: 400px;
  height: 100%;
  background: var(--ui-surface);
  border-left: 1px solid var(--ui-border);
  overflow-y: auto;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  font-family: var(--ui-font);
  color: var(--ui-text);
}

.compression-panel::-webkit-scrollbar { width: 4px; }
.compression-panel::-webkit-scrollbar-track { background: var(--ui-surface); }
.compression-panel::-webkit-scrollbar-thumb { background: var(--ui-glass-2); }

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-bottom: 10px;
  border-bottom: 1px solid var(--ui-border);
}

.header-left {
  display: flex;
  align-items: center;
  gap: 8px;
}

.icon-compress {
  font-family: var(--ui-font);
  font-size: 16px;
  color: var(--ui-text);
}

.panel-title {
  font-size: 14px;
  font-weight: 800;
  letter-spacing: 1px;
  color: var(--ui-text);
  margin: 0;
}

.close-btn {
  background: none;
  border: 1px solid var(--ui-border);
  color: var(--ui-text);
  cursor: pointer;
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 11px;
  font-family: var(--ui-font);
  font-weight: 700;
}

.close-btn:hover {
  background: var(--ui-glass-2);
  color: var(--ui-text);
}

.section {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.section-title {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 1px;
  color: color-mix(in srgb, var(--ui-text) 60%, transparent);
  margin: 0;
  padding-bottom: 4px;
  border-bottom: 1px solid var(--ui-hairline);
}

.algo-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.algo-btn {
  background: var(--ui-surface);
  border: 1px solid var(--ui-border);
  padding: 8px 10px;
  cursor: pointer;
  text-align: left;
  display: flex;
  flex-direction: column;
  gap: 3px;
  color: var(--ui-text);
  font-family: var(--ui-font);
}

.algo-btn:hover,
.algo-btn.selected {
  background: var(--ui-glass-2);
  color: var(--ui-text);
}

.algo-btn:hover .algo-desc,
.algo-btn.selected .algo-desc {
  color: var(--ui-text) !important;
}

.algo-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.algo-name {
  font-size: 11px;
  font-weight: 700;
}

.algo-speed {
  font-size: 9px;
  border: 1px solid;
  padding: 0 4px;
  opacity: 0.7;
}

.algo-desc {
  font-size: 10px;
  line-height: 1.3;
}

.selected-file-name {
  font-size: 11px;
  color: var(--ui-text);
  background: color-mix(in srgb, var(--ui-text) 6%, transparent);
  padding: 4px 8px;
  border: 1px solid var(--ui-hairline);
  word-break: break-all;
  margin: 0;
}

.compress-btn {
  background: var(--ui-glass-2);
  color: var(--ui-text);
  border: 1px solid var(--ui-border);
  padding: 8px 16px;
  font-size: 11px;
  font-weight: 800;
  cursor: pointer;
  font-family: var(--ui-font);
  width: 100%;
}

.compress-btn:hover {
  background: var(--ui-surface);
  color: var(--ui-text);
}

.stats-card {
  border: 1px solid var(--ui-border);
  padding: 10px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.stat-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.stat-key {
  font-size: 10px;
}

.stat-value {
  font-size: 11px;
  font-weight: 700;
  font-family: var(--ui-font);
}

.text-muted { color: color-mix(in srgb, var(--ui-text) 50%, transparent) !important; }
</style>
