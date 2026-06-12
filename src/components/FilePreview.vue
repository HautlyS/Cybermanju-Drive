<template>
  <aside v-if="store.selectedFile" class="file-preview">
    <div class="preview-header">
      <span class="preview-icon">{{ getIcon(store.selectedFile) }}</span>
      <div class="preview-file-info">
        <span class="preview-filename truncate">{{ store.selectedFile.name }}</span>
        <span class="preview-path truncate text-muted">{{ store.selectedFile.path }}</span>
      </div>
    </div>

    <div class="preview-scroll">
      <div class="preview-section">
        <div class="section-label">METADATA</div>
        <div class="meta-grid">
          <div class="meta-row">
            <span class="meta-key text-muted">SIZE</span>
            <span class="meta-value mono">{{ formatSize(store.selectedFile.sizeBytes) }}</span>
          </div>
          <div class="meta-row">
            <span class="meta-key text-muted">TYPE</span>
            <span class="meta-value">{{ store.selectedFile.mimeType || store.selectedFile.fileType }}</span>
          </div>
          <div class="meta-row">
            <span class="meta-key text-muted">CREATED</span>
            <span class="meta-value mono">{{ formatDate(store.selectedFile.createdAt) }}</span>
          </div>
          <div class="meta-row">
            <span class="meta-key text-muted">MODIFIED</span>
            <span class="meta-value mono">{{ formatDate(store.selectedFile.modifiedAt) }}</span>
          </div>
        </div>
      </div>

      <div v-if="store.selectedFile.encrypted" class="preview-section">
        <div class="section-label">ENCRYPTION</div>
        <div class="info-card">
          <div class="info-row">
            <span class="info-key text-muted">ALGORITHM</span>
            <span class="info-badge">{{ store.selectedFile.encryptionAlgorithm?.toUpperCase() }}</span>
          </div>
          <div v-if="store.selectedFile.contextData?.keyId" class="info-row">
            <span class="info-key text-muted">KEY ID</span>
            <span class="meta-value mono">{{ String(store.selectedFile.contextData.keyId).substring(0, 16) }}..</span>
          </div>
        </div>
      </div>

      <div v-if="store.selectedFile.compressionLayers && store.selectedFile.compressionLayers[0] && store.selectedFile.compressionLayers[0] !== 'none'" class="preview-section">
        <div class="section-label">COMPRESSION</div>
        <div class="info-card">
          <div class="info-row">
            <span class="info-key text-muted">LAYER</span>
            <span class="info-badge">{{ store.selectedFile.compressionLayers[0]?.toUpperCase() }}</span>
          </div>
          <div v-if="store.selectedFile.hashBlake3" class="info-row">
            <span class="info-key text-muted">BLAKE3</span>
            <span class="meta-value mono" style="font-size:9px;word-break:break-all">{{ store.selectedFile.hashBlake3.substring(0, 20) }}..</span>
          </div>
        </div>
      </div>

      <div v-if="store.selectedFile.tags && store.selectedFile.tags.length" class="preview-section">
        <div class="section-label">TAGS</div>
        <div class="tag-list">
          <span v-for="tag in store.selectedFile.tags" :key="tag" class="tag-item">{{ tag }}</span>
        </div>
      </div>

      <div v-if="store.selectedFile.faceGroupIds && store.selectedFile.faceGroupIds.length" class="preview-section">
        <div class="section-label">FACE GROUPS</div>
        <div class="face-list">
          <div v-for="groupId in store.selectedFile.faceGroupIds" :key="groupId" class="face-item">
            <div class="avatar-sm">++</div>
            <span class="face-name">{{ getFaceGroupName(groupId) }}</span>
          </div>
        </div>
      </div>

      <div v-if="store.selectedFile.gpsLat" class="preview-section">
        <div class="section-label">LOCATION</div>
        <div class="info-card">
          <div class="info-row">
            <span class="info-key text-muted">COORDS</span>
            <span class="meta-value mono" style="font-size:9px">{{ store.selectedFile.gpsLat.toFixed(4) }}, {{ store.selectedFile.gpsLon?.toFixed(4) }}</span>
          </div>
        </div>
      </div>

      <div v-if="store.parseResult && store.parseResult.symbols.length" class="preview-section">
        <div class="section-label">SYMBOLS - {{ store.parseResult.language }}</div>
        <div class="symbol-tree">
          <div v-for="sym in store.parseResult.symbols" :key="sym.name + sym.startLine" class="symbol-row">
            <span class="symbol-kind">{{ sym.kind }}</span>
            <span class="symbol-name">{{ sym.name }}</span>
          </div>
        </div>
      </div>

      <div v-if="store.selectedFile.contextData && Object.keys(store.selectedFile.contextData).length" class="preview-section">
        <div class="section-label">CONTEXT</div>
        <div class="info-card">
          <div v-for="(value, key) in filteredContextData" :key="String(key)" class="info-row">
            <span class="info-key text-muted">{{ key }}</span>
            <span class="meta-value mono" style="font-size:9px">{{ String(value).substring(0, 24) }}</span>
          </div>
        </div>
      </div>
    </div>

    <div class="preview-actions">
      <button class="pa-btn" @click="handleEncrypt" title="ENCRYPT">{{ store.selectedFile.encrypted ? '[DEC]' : '[ENC]' }}</button>
      <button class="pa-btn" @click="handleCompress" title="COMPRESS">[CMP]</button>
      <button class="pa-btn" @click="handleStar" title="STAR">{{ store.selectedFile.isStarred ? '[*]' : '[ ]' }}</button>
      <button class="pa-btn" @click="handleCopyPath" title="COPY PATH">[CP]</button>
      <button class="pa-btn danger" @click="handleDelete" title="DELETE">[DEL]</button>
    </div>
  </aside>

  <aside v-else class="file-preview empty-preview">
    <div class="empty-content">
      <span style="font-size:24px">[=]</span>
      <span class="text-muted" style="font-size:11px">SELECT A FILE</span>
    </div>
  </aside>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useAppStore } from '@/stores/app'
import type { FileNode } from '@/types'

const store = useAppStore()

const filteredContextData = computed(() => {
  if (!store.selectedFile?.contextData) return {}
  const { keyId, ...rest } = store.selectedFile.contextData
  return rest
})

function getIcon(file: FileNode): string {
  if (file.fileType === 'folder') return '[+]'
  if (file.encrypted) return '[@]'
  if (file.compressionLayers && file.compressionLayers[0] && file.compressionLayers[0] !== 'none') return '[$]'
  if (file.mimeType?.startsWith('image/')) return '[I]'
  if (file.mimeType?.startsWith('text/') || file.mimeType?.includes('json')) return '[T]'
  return '[=]'
}

function getFaceGroupName(groupId: string): string {
  return store.faceGroups.find(fg => fg.id === groupId)?.name || groupId
}

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
  return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric', hour: '2-digit', minute: '2-digit' })
}

function handleEncrypt() {
  if (!store.selectedFile) return
  store.encryptFile(store.selectedFile.id, 'hybrid')
}

function handleCompress() {
  if (!store.selectedFile) return
  store.compressFile(store.selectedFile.id, 'zstd')
}

function handleStar() {
  if (!store.selectedFile) return
  store.toggleStar(store.selectedFile.id)
}

async function handleCopyPath() {
  if (!store.selectedFile?.path) return
  try { await navigator.clipboard.writeText(store.selectedFile.path) } catch {}
}

function handleDelete() {
  if (!store.selectedFile) return
  if (confirm('DELETE "' + store.selectedFile.name + '"?')) {
    store.deleteFile(store.selectedFile.id)
  }
}
</script>

<style scoped>
.file-preview {
  width: 340px;
  min-width: 280px;
  display: flex;
  flex-direction: column;
  background: #000;
  border-left: 2px solid #FFFFFF;
  overflow: hidden;
  z-index: 5;
  height: 100%;
}

.empty-preview {
  align-items: center;
  justify-content: center;
}

.empty-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  opacity: 0.4;
}

.preview-header {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 10px;
  border-bottom: 2px solid #FFFFFF;
  flex-shrink: 0;
}

.preview-icon {
  font-family: 'Courier New', monospace;
  font-size: 18px;
  color: #FFFFFF;
  flex-shrink: 0;
}

.preview-file-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.preview-filename {
  font-size: 12px;
  font-weight: 700;
  color: #FFFFFF;
}

.preview-path {
  font-size: 9px;
  color: rgba(255,255,255,0.5);
}

.preview-scroll {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 0 0 8px;
}

.preview-section {
  padding: 8px 10px 0;
}

.section-label {
  font-family: 'Courier New', monospace;
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.8px;
  color: rgba(255,255,255,0.5);
  margin-bottom: 6px;
}

.meta-grid {
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.meta-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 6px;
}

.meta-key {
  font-size: 10px;
  flex-shrink: 0;
  color: rgba(255,255,255,0.5) !important;
}

.meta-value {
  font-size: 10px;
  color: #FFFFFF;
  text-align: right;
}

.info-card {
  padding: 6px 8px;
  border: 2px solid #FFFFFF;
  background: #000;
}

.info-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 6px;
  padding: 2px 0;
}

.info-key {
  font-size: 10px;
  flex-shrink: 0;
  color: rgba(255,255,255,0.5) !important;
}

.info-badge {
  font-family: 'Courier New', monospace;
  font-size: 9px;
  font-weight: 700;
  border: 1px solid #FFFFFF;
  padding: 0 4px;
  color: #FFFFFF;
}

.tag-list {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.tag-item {
  font-size: 9px;
  font-family: 'Courier New', monospace;
  font-weight: 700;
  border: 2px solid #FFFFFF;
  padding: 1px 6px;
  color: #FFFFFF;
}

.face-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.face-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 3px 6px;
}

.avatar-sm {
  width: 20px;
  height: 20px;
  border: 2px solid #FFFFFF;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  font-size: 8px;
  color: #FFFFFF;
}

.face-name {
  font-size: 11px;
  color: #FFFFFF;
}

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
  font-family: 'Courier New', monospace;
  font-size: 8px;
  color: rgba(255,255,255,0.6);
  text-transform: uppercase;
  flex-shrink: 0;
  min-width: 40px;
}

.symbol-name {
  color: #FFFFFF;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.preview-actions {
  display: flex;
  gap: 2px;
  padding: 6px;
  border-top: 2px solid #FFFFFF;
  flex-shrink: 0;
}

.pa-btn {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  height: 28px;
  border: 2px solid #FFFFFF;
  background: #000;
  color: #FFFFFF;
  cursor: pointer;
  font-family: 'Courier New', monospace;
  font-size: 9px;
  font-weight: 700;
}

.pa-btn:hover {
  background: #FFFFFF;
  color: #000;
}

.pa-btn.danger:hover {
  background: #FFFFFF;
  color: #000;
}

.mono { font-family: 'Courier New', monospace; }
.text-muted { color: rgba(255,255,255,0.5) !important; }
.truncate { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
