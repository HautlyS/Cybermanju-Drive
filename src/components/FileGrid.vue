<template>
  <main class="file-grid">
    <div class="file-toolbar">
      <div class="ft-left">
        <button class="view-toggle" :class="{ active: store.viewMode === 'grid' }" @click="store.viewMode = 'grid'" title="GRID">[##]</button>
        <button class="view-toggle" :class="{ active: store.viewMode === 'list' }" @click="store.viewMode = 'list'" title="LIST">[#]</button>
      </div>
      <div class="ft-center">
        <span class="ft-info">{{ displayFiles.length }} ITEMS</span>
      </div>
      <div class="ft-right">
        <span v-if="store.isLoading" class="text-muted">LOADING..</span>
      </div>
    </div>

    <div v-if="store.viewMode === 'grid'" class="grid-view">
      <div
        v-for="file in displayFiles"
        :key="file.id"
        class="file-card"
        :class="{ selected: store.selectedFileId === file.id }"
        @click="store.selectedFileId = file.id"
        @dblclick="handleDoubleClick(file)"
        @contextmenu.prevent="showContextMenu($event, file)"
      >
        <div class="file-card-icon">
          <span class="file-icon">{{ getIcon(file) }}</span>
        </div>
        <div class="file-card-name truncate" :title="file.name">{{ file.name }}</div>
        <div class="file-card-meta text-muted">{{ formatSize(file.sizeBytes) }}</div>
        <div class="file-card-badges">
          <span v-if="file.encrypted" class="card-badge" title="ENCRYPTED">[E]</span>
          <span v-if="file.compressionLayers && file.compressionLayers[0] && file.compressionLayers[0] !== 'none'" class="card-badge" title="COMPRESSED">[C]</span>
          <span v-if="file.isStarred" class="card-badge" title="STARRED">[*]</span>
          <span v-if="file.gpsLat" class="card-badge" title="HAS GPS">[G]</span>
          <span v-if="file.faceGroupIds && file.faceGroupIds.length" class="card-badge" title="HAS FACES">[F]</span>
        </div>
      </div>
      <div v-if="displayFiles.length === 0 && !store.isLoading" class="empty-grid">
        <span class="text-muted">NO FILES IN THIS DIRECTORY</span>
      </div>
    </div>

    <div v-if="store.viewMode === 'list'" class="list-view">
      <div class="list-header">
        <span class="lc lc-name">NAME</span>
        <span class="lc lc-size">SIZE</span>
        <span class="lc lc-type">TYPE</span>
        <span class="lc lc-date">MODIFIED</span>
        <span class="lc lc-status">FLAGS</span>
        <span class="lc lc-hash">BLAKE3</span>
      </div>
      <div
        v-for="file in displayFiles"
        :key="file.id"
        class="list-row"
        :class="{ selected: store.selectedFileId === file.id }"
        @click="store.selectedFileId = file.id"
        @dblclick="handleDoubleClick(file)"
        @contextmenu.prevent="showContextMenu($event, file)"
      >
        <span class="lc lc-name">
          <span class="file-icon-sm">{{ getIcon(file) }}</span>
          <span class="truncate">{{ file.name }}</span>
        </span>
        <span class="lc lc-size text-muted">{{ formatSize(file.sizeBytes) }}</span>
        <span class="lc lc-type text-muted">{{ file.mimeType || (file.fileType === 'folder' ? 'FOLDER' : 'FILE') }}</span>
        <span class="lc lc-date text-muted">{{ formatDate(file.modifiedAt) }}</span>
        <span class="lc lc-status">
          <span v-if="file.encrypted" class="badge-sm">[E]</span>
          <span v-if="file.compressionLayers && file.compressionLayers[0] && file.compressionLayers[0] !== 'none'" class="badge-sm">[C]</span>
          <span v-if="file.isStarred" class="badge-sm">[*]</span>
        </span>
        <span class="lc lc-hash text-muted">{{ file.hashBlake3 ? file.hashBlake3.substring(0, 8) + '..' : '--' }}</span>
      </div>
      <div v-if="displayFiles.length === 0 && !store.isLoading" class="empty-list">
        <span class="text-muted">NO FILES</span>
      </div>
    </div>

    <Teleport to="body">
      <div
        v-if="contextMenu.visible"
        class="context-menu"
        :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }"
        @click="contextMenu.visible = false"
      >
        <div class="context-menu-item" @click="handleContextAction('open')">OPEN</div>
        <div class="context-menu-item" @click="handleContextAction('preview')">PREVIEW</div>
        <div class="context-menu-divider" />
        <div class="context-menu-item" @click="handleContextAction('encrypt')">ENCRYPT</div>
        <div class="context-menu-item" @click="handleContextAction('compress')">COMPRESS</div>
        <div class="context-menu-item" @click="handleContextAction('star')">STAR</div>
        <div class="context-menu-divider" />
        <div class="context-menu-item" @click="handleContextAction('duplicate')">DUPLICATE CTX</div>
        <div class="context-menu-divider" />
        <div class="context-menu-item danger" @click="handleContextAction('delete')">DELETE</div>
      </div>
    </Teleport>
  </main>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useAppStore } from '@/stores/app'
import type { FileNode } from '@/types'

const store = useAppStore()

const displayFiles = computed(() => store.currentFolderFiles)

const contextMenu = ref({ visible: false, x: 0, y: 0, file: null as FileNode | null })

function getIcon(file: FileNode): string {
  if (file.fileType === 'folder') return '[+]'
  if (file.encrypted) return '[@]'
  if (file.compressionLayers && file.compressionLayers[0] && file.compressionLayers[0] !== 'none') return '[$]'
  if (file.mimeType?.startsWith('image/')) return '[I]'
  if (file.mimeType?.startsWith('text/') || file.mimeType?.includes('json') || file.mimeType?.includes('xml')) return '[T]'
  return '[=]'
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
  return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })
}

function handleDoubleClick(file: FileNode) {
  store.selectFile(file.id)
}

function showContextMenu(e: MouseEvent, file: FileNode) {
  contextMenu.value = { visible: true, x: e.clientX, y: e.clientY, file }
}

function handleContextAction(action: string) {
  const file = contextMenu.value.file
  if (!file) return
  switch (action) {
    case 'open': handleDoubleClick(file); break
    case 'preview': store.selectedFileId = file.id; break
    case 'encrypt': store.encryptFile(file.id, 'hybrid'); break
    case 'compress': store.compressFile(file.id, 'zstd'); break
    case 'star': store.toggleStar(file.id); break
    case 'duplicate': store.duplicateFileContext(file.id); break
    case 'delete': store.deleteFile(file.id); break
  }
  contextMenu.value.visible = false
}

function closeContextMenu() {
  contextMenu.value.visible = false
}

onMounted(() => document.addEventListener('click', closeContextMenu))
onUnmounted(() => document.removeEventListener('click', closeContextMenu))
</script>

<style scoped>
.file-grid {
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: #000;
  position: relative;
  height: 100%;
}

.file-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 32px;
  padding: 0 8px;
  border-bottom: 2px solid #FFFFFF;
  background: #000;
  flex-shrink: 0;
}

.ft-left, .ft-right {
  display: flex;
  align-items: center;
  gap: 4px;
}

.ft-info {
  font-family: 'Courier New', monospace;
  font-size: 10px;
  color: rgba(255,255,255,0.5);
}

.view-toggle {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 2px 6px;
  cursor: pointer;
  color: rgba(255,255,255,0.4);
  font-family: 'Courier New', monospace;
  font-size: 10px;
  font-weight: 700;
  border: 2px solid transparent;
}

.view-toggle:hover {
  color: #FFFFFF;
  border-color: #FFFFFF;
}

.view-toggle.active {
  color: #000;
  background: #FFFFFF;
  border-color: #FFFFFF;
}

.grid-view {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 10px;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
  gap: 8px;
  align-content: start;
}

.file-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 12px 8px 8px;
  cursor: pointer;
  border: 2px solid #FFFFFF;
  background: #000;
  min-height: 110px;
  position: relative;
}

.file-card:hover {
  background: #FFFFFF;
}

.file-card:hover .file-card-name,
.file-card:hover .file-card-meta {
  color: #000;
}

.file-card.selected {
  background: #FFFFFF;
}

.file-card.selected .file-card-name,
.file-card.selected .file-card-meta,
.file-card.selected .file-icon {
  color: #000;
}

.file-card-icon {
  margin-bottom: 6px;
  height: 30px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.file-icon {
  font-family: 'Courier New', monospace;
  font-size: 18px;
  color: #FFFFFF;
}

.file-card-name {
  font-size: 10px;
  font-weight: 600;
  color: #FFFFFF;
  text-align: center;
  width: 100%;
}

.file-card-meta {
  font-size: 9px;
  font-family: 'Courier New', monospace;
  margin-top: 2px;
}

.file-card-badges {
  display: flex;
  gap: 2px;
  margin-top: 4px;
  position: absolute;
  top: 4px;
  right: 6px;
}

.card-badge {
  font-family: 'Courier New', monospace;
  font-size: 8px;
  font-weight: 700;
  color: #FFFFFF;
  border: 1px solid #FFFFFF;
  padding: 0 2px;
}

.empty-grid {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 60px 20px;
  font-size: 11px;
}

.list-view {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
}

.list-header {
  display: flex;
  align-items: center;
  height: 28px;
  padding: 0 10px;
  background: #000;
  border-bottom: 2px solid #FFFFFF;
  font-size: 9px;
  font-family: 'Courier New', monospace;
  font-weight: 700;
  letter-spacing: 0.5px;
  color: rgba(255,255,255,0.5);
  position: sticky;
  top: 0;
  z-index: 2;
}

.list-row {
  display: flex;
  align-items: center;
  height: 30px;
  padding: 0 10px;
  border-bottom: 1px solid rgba(255,255,255,0.1);
  cursor: pointer;
  font-size: 11px;
  color: #FFFFFF;
}

.list-row:hover {
  background: rgba(255,255,255,0.1);
}

.list-row.selected {
  background: #FFFFFF;
  color: #000;
}

.list-row.selected .text-muted {
  color: #000 !important;
  opacity: 0.6;
}

.lc {
  display: flex;
  align-items: center;
  gap: 4px;
  overflow: hidden;
  white-space: nowrap;
}

.lc-name { flex: 3; min-width: 0; }
.lc-size { flex: 1; min-width: 50px; justify-content: flex-end; }
.lc-type { flex: 1.2; min-width: 60px; }
.lc-date { flex: 1.2; min-width: 80px; }
.lc-status { flex: 0.8; min-width: 50px; gap: 2px; }
.lc-hash { flex: 1.2; min-width: 80px; font-size: 9px; font-family: 'Courier New', monospace; }

.file-icon-sm {
  font-family: 'Courier New', monospace;
  font-size: 11px;
  color: #FFFFFF;
  flex-shrink: 0;
  width: 18px;
  text-align: center;
}

.badge-sm {
  font-size: 8px;
  font-family: 'Courier New', monospace;
  font-weight: 700;
  border: 1px solid #FFFFFF;
  padding: 0 2px;
  color: #FFFFFF;
}

.empty-list {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 40px;
  font-size: 11px;
}

.text-muted { color: rgba(255,255,255,0.5) !important; }
.truncate { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
