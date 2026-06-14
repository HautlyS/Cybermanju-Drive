<template>
  <Teleport to="body">
    <div
      v-if="file && visible"
      class="file-tooltip"
      :style="{ left: x + 'px', top: y + 'px' }"
    >
      <div class="tt-header">
        <span class="tt-icon">{{ icon }}</span>
        <span class="tt-name truncate">{{ file.name }}</span>
      </div>
      <div class="tt-meta">
        <div class="tt-row"><span class="tt-label">SIZE</span><span>{{ formatSize(file.sizeBytes) }}</span></div>
        <div class="tt-row"><span class="tt-label">TYPE</span><span>{{ file.mimeType || (file.fileType === 'folder' ? 'FOLDER' : 'FILE') }}</span></div>
        <div class="tt-row"><span class="tt-label">MODIFIED</span><span>{{ formatDate(file.modifiedAt) }}</span></div>
        <div class="tt-row" v-if="file.hashBlake3"><span class="tt-label">BLAKE3</span><span class="tt-hash">{{ file.hashBlake3.substring(0, 16) }}..</span></div>
      </div>
      <div class="tt-badges" v-if="file.encrypted || (file.compressionLayers && file.compressionLayers[0] && file.compressionLayers[0] !== 'none') || file.gpsLat || file.isStarred">
        <span v-if="file.encrypted" class="tt-badge">ENC {{ file.encryptionAlgorithm || '' }}</span>
        <span v-if="file.compressionLayers && file.compressionLayers[0] && file.compressionLayers[0] !== 'none'" class="tt-badge">CMP {{ file.compressionLayers.join('+') }}</span>
        <span v-if="file.isStarred" class="tt-badge">STARRED</span>
        <span v-if="file.gpsLat" class="tt-badge">GPS</span>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import type { FileNode } from '@/types'

defineProps<{
  file: FileNode | null
  visible: boolean
  x: number
  y: number
}>()

function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const k = 1024
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + units[i]
}

function formatDate(dateStr: string): string {
  if (!dateStr) return '--'
  const d = new Date(dateStr)
  return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })
}

function getIcon(file: FileNode): string {
  if (file.fileType === 'folder') return '[+]'
  if (file.encrypted) return '[@]'
  if (file.compressionLayers && file.compressionLayers[0] && file.compressionLayers[0] !== 'none') return '[$]'
  if (file.mimeType?.startsWith('image/')) return '[I]'
  if (file.mimeType?.startsWith('text/') || file.mimeType?.includes('json') || file.mimeType?.includes('xml')) return '[T]'
  return '[=]'
}

const icon = getIcon
</script>

<style scoped>
.file-tooltip {
  position: fixed;
  z-index: 1000;
  background: #000;
  border: 2px solid #FFFFFF;
  padding: 10px 12px;
  min-width: 200px;
  max-width: 280px;
  pointer-events: none;
  font-family: 'Courier New', monospace;
  color: #FFFFFF;
  transform: translate(12px, -50%);
  box-shadow: 3px 3px 0 rgba(255,255,255,0.15);
}

.tt-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
  padding-bottom: 6px;
  border-bottom: 1px solid rgba(255,255,255,0.2);
}

.tt-icon {
  font-size: 14px;
  flex-shrink: 0;
}

.tt-name {
  font-size: 11px;
  font-weight: 700;
}

.tt-meta {
  display: flex;
  flex-direction: column;
  gap: 3px;
  margin-bottom: 6px;
}

.tt-row {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  font-size: 9px;
}

.tt-label {
  color: rgba(255,255,255,0.4);
  flex-shrink: 0;
}

.tt-hash {
  font-size: 8px;
  color: rgba(255,255,255,0.5);
}

.tt-badges {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  padding-top: 6px;
  border-top: 1px solid rgba(255,255,255,0.15);
}

.tt-badge {
  font-size: 8px;
  font-weight: 700;
  border: 1px solid #FFFFFF;
  padding: 1px 4px;
}
</style>
