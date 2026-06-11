<template>
  <div class="compression-panel">
    <!-- Header -->
    <div class="panel-header">
      <div class="header-left">
        <Archive :size="22" class="icon-archive" />
        <h2 class="panel-title">Triple Compression Engine</h2>
      </div>
      <button class="close-btn" @click="$emit('close')">
        <span class="close-x">✕</span>
      </button>
    </div>

    <!-- Compression Mode Selector -->
    <div class="section">
      <h3 class="section-title">
        <Gauge :size="16" />
        Compression Mode
      </h3>
      <div class="mode-buttons">
        <button
          v-for="(info, mode) in COMPRESSION_INFO"
          :key="mode"
          class="mode-btn"
          :class="{ active: selectedMode === mode }"
          :style="{
            borderColor: selectedMode === mode ? info.color : '#000',
            '--mode-color': info.color,
          }"
          @click="selectedMode = mode as CompressionType"
        >
          <span class="mode-name">{{ info.name }}</span>
          <span class="mode-speed">{{ info.speed }}</span>
        </button>
      </div>
    </div>

    <!-- Compression Pipeline Visualization -->
    <div class="section" v-if="compressionStats">
      <h3 class="section-title">
        <Database :size="16" />
        Compression Pipeline
      </h3>

      <div class="pipeline">
        <div
          v-for="(layer, idx) in compressionStats.layerDetails"
          :key="idx"
          class="pipeline-stage"
        >
          <!-- Layer Box -->
          <div
            class="layer-box"
            :style="{
              borderColor: layer.color,
              '--layer-color': layer.color,
            }"
          >
            <div class="layer-header">
              <Zap :size="14" :style="{ color: layer.color }" />
              <span class="layer-name">{{ layer.algorithm }}</span>
              <span
                class="layer-stage-label"
                :style="{ backgroundColor: layer.color + '22', color: layer.color }"
              >
                Layer {{ idx + 1 }}
              </span>
            </div>

            <div class="layer-sizes">
              <div class="size-item">
                <span class="size-label">IN</span>
                <span class="mono">{{ formatSize(layer.inputSize) }}</span>
              </div>
              <ChevronRight :size="14" class="arrow-icon" />
              <div class="size-item">
                <span class="size-label">OUT</span>
                <span class="mono">{{ formatSize(layer.outputSize) }}</span>
              </div>
            </div>

            <!-- Ratio Bar -->
            <div class="ratio-bar-track">
              <div
                class="ratio-bar-fill"
                :style="{
                  width: Math.min(layer.ratio * 100, 100) + '%',
                  backgroundColor: layer.color,
                }"
              />
              <span class="ratio-text">{{ (layer.ratio * 100).toFixed(1) }}%</span>
            </div>
          </div>

          <!-- Arrow between layers -->
          <div class="pipeline-arrow" v-if="idx < compressionStats.layerDetails.length - 1">
            <ChevronRight :size="20" class="pipeline-arrow-icon" />
          </div>
        </div>
      </div>

      <!-- Overall Stats -->
      <div class="overall-stats">
        <div class="stat-row">
          <span class="stat-label">Original</span>
          <span class="mono">{{ formatSize(compressionStats.originalSize) }}</span>
        </div>
        <div class="stat-row">
          <span class="stat-label">Final</span>
          <span class="mono highlight-cyan">{{ formatSize(compressionStats.compressedSize) }}</span>
        </div>
        <div class="stat-row">
          <span class="stat-label">Total Ratio</span>
          <span class="mono highlight-green">{{ (compressionStats.ratio * 100).toFixed(1) }}%</span>
        </div>
        <div class="stat-row">
          <span class="stat-label">Duration</span>
          <span class="mono">{{ compressionStats.durationMs.toFixed(1) }} ms</span>
        </div>
      </div>

      <!-- BLAKE3 Hash -->
      <div class="hash-section">
        <div class="hash-label">
          <Hash :size="14" />
          BLAKE3 Checksum
        </div>
        <div class="hash-value mono">{{ compressionStats.blake3Hash }}</div>
      </div>
    </div>

    <!-- Compress Button -->
    <div class="section" v-if="selectedFile">
      <h3 class="section-title">
        <Zap :size="16" />
        Action
      </h3>
      <p class="selected-file-name">{{ selectedFile.name }}</p>
      <button class="compress-btn" @click="handleCompress">
        <Archive :size="14" />
        Compress Selected File
      </button>
    </div>

    <!-- Empty state -->
    <div class="empty-state" v-if="!compressionStats && !selectedFile">
      <Archive :size="40" class="empty-icon" />
      <p>Select a file and compress it to see the triple-layer pipeline.</p>
    </div>

    <!-- Status Footer -->
    <div class="status-footer">
      <span>⚡ Triple-layer: LZ4 → Zstandard → Brotli • BLAKE3 integrity hashing</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useAppStore } from '@/stores/app'
import type { CompressionType } from '@/types'
import { COMPRESSION_INFO } from '@/types'
import {
  Archive,
  ChevronRight,
  Zap,
  Database,
  Hash,
  Gauge,
} from 'lucide-vue-next'

const store = useAppStore()

const emit = defineEmits<{
  close: []
}>()

const compressionStats = computed(() => store.compressionStats)
const selectedFile = computed(() => store.selectedFile)

const selectedMode = ref<CompressionType>('none')

function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const k = 1024
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + units[i]
}

async function handleCompress() {
  if (!store.selectedFileId) return
  await store.compressFile(store.selectedFileId, selectedMode.value)
}
</script>

<style scoped>
.compression-panel {
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

.compression-panel::-webkit-scrollbar {
  width: 6px;
}
.compression-panel::-webkit-scrollbar-track {
  background: #0a0a0f;
}
.compression-panel::-webkit-scrollbar-thumb {
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

.icon-archive {
  color: #00D4FF;
  filter: drop-shadow(0 0 6px #00D4FF);
}

.panel-title {
  font-size: 16px;
  font-weight: 800;
  text-transform: uppercase;
  letter-spacing: 2px;
  color: #00D4FF;
  text-shadow: 0 0 10px #00D4FF, 0 0 20px rgba(0, 212, 255, 0.3);
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

/* Sections */
.section {
  display: flex;
  flex-direction: column;
  gap: 12px;
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

/* Mode Buttons */
.mode-buttons {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}

.mode-btn {
  background: #1a1a2e;
  border: 3px solid #000;
  box-shadow: 3px 3px 0 0 #000;
  padding: 10px 12px;
  cursor: pointer;
  text-align: center;
  transition: all 0.15s;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}

.mode-btn.active {
  box-shadow: 3px 3px 0 0 var(--mode-color);
}

.mode-btn:hover {
  transform: translate(-1px, -1px);
  box-shadow: 4px 4px 0 0 #000;
}

.mode-btn:active {
  transform: translate(1px, 1px);
  box-shadow: 2px 2px 0 0 #000;
}

.mode-name {
  font-size: 12px;
  font-weight: 700;
  color: #F5F5F4;
}

.mode-speed {
  font-size: 10px;
  color: #6B7280;
}

/* Pipeline */
.pipeline {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0;
}

.pipeline-stage {
  display: flex;
  flex-direction: column;
  align-items: center;
  width: 100%;
}

.layer-box {
  width: 100%;
  background: #1a1a2e;
  border: 3px solid;
  box-shadow: 3px 3px 0 0 #000;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  transition: all 0.3s;
}

.layer-header {
  display: flex;
  align-items: center;
  gap: 8px;
}

.layer-name {
  font-size: 13px;
  font-weight: 700;
}

.layer-stage-label {
  font-size: 10px;
  font-weight: 800;
  padding: 2px 6px;
  border-radius: 2px;
  letter-spacing: 1px;
}

.layer-sizes {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 0;
}

.size-item {
  display: flex;
  align-items: center;
  gap: 4px;
}

.size-label {
  font-size: 10px;
  font-weight: 700;
  color: #6B7280;
  letter-spacing: 1px;
}

.arrow-icon {
  color: #4B5563;
}

/* Ratio Bar */
.ratio-bar-track {
  position: relative;
  width: 100%;
  height: 22px;
  background: #0a0a0f;
  border: 2px solid #000;
  overflow: hidden;
}

.ratio-bar-fill {
  height: 100%;
  transition: width 0.8s cubic-bezier(0.25, 0.46, 0.45, 0.94);
  opacity: 0.8;
}

.ratio-text {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  font-size: 11px;
  font-weight: 700;
  color: #F5F5F4;
  text-shadow: 0 1px 3px rgba(0, 0, 0, 0.8);
}

/* Pipeline Arrow */
.pipeline-arrow {
  padding: 4px 0;
}

.pipeline-arrow-icon {
  color: #4B5563;
}

/* Overall Stats */
.overall-stats {
  background: #1a1a2e;
  border: 3px solid #000;
  box-shadow: 3px 3px 0 0 #000;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.stat-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.stat-label {
  font-size: 12px;
  color: #9CA3AF;
  font-weight: 600;
}

.highlight-cyan {
  color: #00D4FF;
}

.highlight-green {
  color: #00FF41;
}

/* Hash */
.hash-section {
  background: #0a0a0f;
  border: 3px solid #000;
  box-shadow: 3px 3px 0 0 #000;
  padding: 10px 12px;
}

.hash-label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: #6B7280;
  margin-bottom: 6px;
}

.hash-value {
  font-size: 12px;
  color: #00FF41;
  word-break: break-all;
  line-height: 1.6;
  text-shadow: 0 0 6px rgba(0, 255, 65, 0.3);
}

/* Compress Button */
.selected-file-name {
  font-size: 12px;
  color: #00D4FF;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  background: #0a0a0f;
  padding: 6px 10px;
  border: 2px solid #1a1a2e;
  word-break: break-all;
  margin: 0;
}

.compress-btn {
  width: 100%;
  background: #00D4FF;
  color: #0a0a0f;
  border: 3px solid #000;
  box-shadow: 3px 3px 0 0 #000;
  padding: 10px 16px;
  font-size: 12px;
  font-weight: 800;
  text-transform: uppercase;
  letter-spacing: 1px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  transition: all 0.15s;
}

.compress-btn:hover {
  transform: translate(-1px, -1px);
  box-shadow: 4px 4px 0 0 #000;
  filter: brightness(1.1);
}

.compress-btn:active {
  transform: translate(1px, 1px);
  box-shadow: 2px 2px 0 0 #000;
}

/* Empty State */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  padding: 40px 20px;
  text-align: center;
}

.empty-icon {
  color: #333;
}

.empty-state p {
  font-size: 13px;
  color: #6B7280;
  margin: 0;
  line-height: 1.6;
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
