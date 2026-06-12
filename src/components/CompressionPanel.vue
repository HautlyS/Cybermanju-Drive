<template>
  <div class="compression-panel">
    <div class="panel-header">
      <div class="header-left">
        <span class="icon-compress">[$]</span>
        <h2 class="panel-title">COMPRESSION ENGINE</h2>
      </div>
      <button class="close-btn" @click="$emit('close')">X</button>
    </div>

    <div class="section">
      <h3 class="section-title">[CMP] COMPRESSION ALGORITHMS</h3>
      <div class="algo-list">
        <button
          v-for="(info, type) in COMPRESSION_INFO"
          :key="type"
          class="algo-btn"
          :class="{ selected: selectedAlgo === type }"
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

    <div class="section" v-if="selectedFile">
      <h3 class="section-title">[FILE] SELECTED FILE</h3>
      <p class="selected-file-name">{{ selectedFile.name }}</p>
      <button class="compress-btn" @click="handleCompress">[COMPRESS]</button>
    </div>

    <div class="section" v-if="compressionStats">
      <h3 class="section-title">[STATS] RESULTS</h3>
      <div class="stats-card">
        <div class="stat-row">
          <span class="stat-key text-muted">ORIGINAL</span>
          <span class="stat-value">{{ formatSize(compressionStats.originalSize) }}</span>
        </div>
        <div class="stat-row">
          <span class="stat-key text-muted">COMPRESSED</span>
          <span class="stat-value">{{ formatSize(compressionStats.compressedSize) }}</span>
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
import { ref, computed } from 'vue'
import { useAppStore } from '@/stores/app'
import type { CompressionType } from '@/types'
import { COMPRESSION_INFO } from '@/types'

const store = useAppStore()
const emit = defineEmits<{ close: [] }>()

const selectedFile = computed(() => store.selectedFile)
const compressionStats = computed(() => store.compressionStats)
const selectedAlgo = ref<CompressionType>('zstd')

function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const k = 1024
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + units[i]
}

async function handleCompress() {
  if (!store.selectedFileId) return
  await store.compressFile(store.selectedFileId, selectedAlgo.value)
}
</script>

<style scoped>
.compression-panel {
  width: 400px;
  height: 100%;
  background: #000;
  border-left: 2px solid #FFFFFF;
  overflow-y: auto;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  font-family: 'Courier New', monospace;
  color: #FFFFFF;
}

.compression-panel::-webkit-scrollbar { width: 4px; }
.compression-panel::-webkit-scrollbar-track { background: #000; }
.compression-panel::-webkit-scrollbar-thumb { background: #FFFFFF; }

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-bottom: 10px;
  border-bottom: 2px solid #FFFFFF;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 8px;
}

.icon-compress {
  font-family: 'Courier New', monospace;
  font-size: 16px;
  color: #FFFFFF;
}

.panel-title {
  font-size: 14px;
  font-weight: 800;
  letter-spacing: 1px;
  color: #FFFFFF;
  margin: 0;
}

.close-btn {
  background: none;
  border: 2px solid #FFFFFF;
  color: #FFFFFF;
  cursor: pointer;
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 11px;
  font-family: 'Courier New', monospace;
  font-weight: 700;
}

.close-btn:hover {
  background: #FFFFFF;
  color: #000;
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
  color: rgba(255,255,255,0.6);
  margin: 0;
  padding-bottom: 4px;
  border-bottom: 2px solid rgba(255,255,255,0.2);
}

.algo-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.algo-btn {
  background: #000;
  border: 2px solid #FFFFFF;
  padding: 8px 10px;
  cursor: pointer;
  text-align: left;
  display: flex;
  flex-direction: column;
  gap: 3px;
  color: #FFFFFF;
  font-family: 'Courier New', monospace;
}

.algo-btn:hover,
.algo-btn.selected {
  background: #FFFFFF;
  color: #000;
}

.algo-btn:hover .algo-desc,
.algo-btn.selected .algo-desc {
  color: #000 !important;
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
  color: #FFFFFF;
  background: rgba(255,255,255,0.05);
  padding: 4px 8px;
  border: 2px solid rgba(255,255,255,0.3);
  word-break: break-all;
  margin: 0;
}

.compress-btn {
  background: #FFFFFF;
  color: #000;
  border: 2px solid #FFFFFF;
  padding: 8px 16px;
  font-size: 11px;
  font-weight: 800;
  cursor: pointer;
  font-family: 'Courier New', monospace;
  width: 100%;
}

.compress-btn:hover {
  background: #000;
  color: #FFFFFF;
}

.stats-card {
  border: 2px solid #FFFFFF;
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
  font-family: 'Courier New', monospace;
}

.text-muted { color: rgba(255,255,255,0.5) !important; }
</style>
