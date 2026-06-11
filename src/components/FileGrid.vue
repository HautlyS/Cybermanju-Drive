<template>
  <main class="file-grid-area content">
    <!-- Toolbar -->
    <div class="file-toolbar panel-top">
      <div class="toolbar-left">
        <button
          class="view-toggle"
          :class="{ active: store.viewMode === 'grid' }"
          @click="store.viewMode = 'grid'"
          title="Grid view"
        >
          <LayoutGrid :size="15" />
        </button>
        <button
          class="view-toggle"
          :class="{ active: store.viewMode === 'list' }"
          @click="store.viewMode = 'list'"
          title="List view"
        >
          <List :size="15" />
        </button>
      </div>
      <div class="toolbar-center">
        <span class="toolbar-info text-secondary mono">
          {{ displayFiles.length }} items
        </span>
      </div>
      <div class="toolbar-right">
        <span v-if="store.isLoading" class="text-muted">Loading...</span>
      </div>
    </div>

    <!-- Grid View -->
    <div v-if="store.viewMode === 'grid'" class="grid-view">
      <div
        v-for="file in displayFiles"
        :key="file.id"
        class="file-card neobrutalism-card"
        :class="{ 'file-card-selected': store.selectedFileId === file.id }"
        @click="store.selectedFileId = file.id"
        @dblclick="handleDoubleClick(file)"
        @contextmenu.prevent="showContextMenu($event, file)"
      >
        <!-- File icon -->
        <div class="file-card-icon">
          <component :is="getFileIcon(file)" :size="28" :class="getIconColor(file)" />
        </div>

        <!-- File name -->
        <div class="file-card-name truncate" :title="file.name">
          {{ file.name }}
        </div>

        <!-- File meta -->
        <div class="file-card-meta text-muted">
          {{ formatSize(file.sizeBytes) }}
        </div>

        <!-- Badges -->
        <div class="file-card-badges">
          <span v-if="file.encrypted" class="card-badge" title="Encrypted">
            🔒
          </span>
          <span v-if="file.compressionLayers && file.compressionLayers[0] && file.compressionLayers[0] !== 'none'" class="card-badge" title="Compressed">
            📦
          </span>
          <span v-if="file.isStarred" class="card-badge" title="Starred">
            ⭐
          </span>
          <span v-if="file.gpsLat" class="card-badge" title="Has GPS">
            📍
          </span>
          <span v-if="file.faceGroupIds && file.faceGroupIds.length > 0" class="card-badge" title="Has face groups">
            👤
          </span>
        </div>
      </div>

      <!-- Empty state -->
      <div v-if="displayFiles.length === 0 && !store.isLoading" class="empty-grid">
        <Folder :size="40" class="text-muted" />
        <span class="text-muted">No files in this directory</span>
      </div>
    </div>

    <!-- List View -->
    <div v-if="store.viewMode === 'list'" class="list-view">
      <div class="list-header">
        <span class="list-col list-col-name">Name</span>
        <span class="list-col list-col-size">Size</span>
        <span class="list-col list-col-type">Type</span>
        <span class="list-col list-col-date">Modified</span>
        <span class="list-col list-col-status">Status</span>
        <span class="list-col list-col-hash">BLAKE3</span>
      </div>

      <div
        v-for="file in displayFiles"
        :key="file.id"
        class="list-row"
        :class="{ 'list-row-selected': store.selectedFileId === file.id }"
        @click="store.selectedFileId = file.id"
        @dblclick="handleDoubleClick(file)"
        @contextmenu.prevent="showContextMenu($event, file)"
      >
        <span class="list-col list-col-name">
          <component :is="getFileIcon(file)" :size="14" :class="getIconColor(file)" style="flex-shrink:0" />
          <span class="truncate">{{ file.name }}</span>
        </span>
        <span class="list-col list-col-size mono text-secondary">{{ formatSize(file.sizeBytes) }}</span>
        <span class="list-col list-col-type text-muted">{{ file.mimeType || (file.fileType === 'folder' ? 'folder' : 'file') }}</span>
        <span class="list-col list-col-date text-muted mono">{{ formatDate(file.modifiedAt) }}</span>
        <span class="list-col list-col-status">
          <span v-if="file.encrypted" class="badge badge-green" :style="{ borderColor: getEncryptionColor(file) }">🔒</span>
          <span v-if="file.compressionLayers && file.compressionLayers[0] && file.compressionLayers[0] !== 'none'" class="badge badge-gold">📦</span>
          <span v-if="file.isStarred" class="badge badge-gold">⭐</span>
          <span v-if="file.gpsLat" class="badge badge-cyan">📍</span>
        </span>
        <span class="list-col list-col-hash mono text-muted">
          {{ file.hashBlake3 ? file.hashBlake3.substring(0, 10) + '…' : '—' }}
        </span>
      </div>

      <!-- Empty state -->
      <div v-if="displayFiles.length === 0 && !store.isLoading" class="empty-list">
        <span class="text-muted">No files in this directory</span>
      </div>
    </div>

    <!-- Context Menu -->
    <Teleport to="body">
      <div
        v-if="contextMenu.visible"
        class="context-menu"
        :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }"
        @click="contextMenu.visible = false"
      >
        <div class="context-menu-item" @click="handleContextAction('open')">
          <File :size="13" /> Open
        </div>
        <div class="context-menu-item" @click="handleContextAction('preview')">
          <ExternalLink :size="13" /> Preview
        </div>
        <div class="context-menu-divider" />
        <div class="context-menu-item" @click="handleContextAction('encrypt')">
          <Lock :size="13" class="text-neon" /> Encrypt
        </div>
        <div class="context-menu-item" @click="handleContextAction('compress')">
          <Archive :size="13" class="text-gold" /> Compress
        </div>
        <div class="context-menu-item" @click="handleContextAction('star')">
          <Star :size="13" class="text-gold" /> Star
        </div>
        <div class="context-menu-divider" />
        <div class="context-menu-item" @click="handleContextAction('duplicate')">
          <Copy :size="13" class="text-cyan" /> Duplicate Context
        </div>
        <div class="context-menu-divider" />
        <div class="context-menu-item danger" @click="handleContextAction('delete')">
          <Trash2 :size="13" /> Delete
        </div>
      </div>
    </Teleport>
  </main>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, markRaw } from 'vue'
import {
  Folder, File, Image, Lock, Archive, Star, MapPin, Users,
  Code, ChevronRight, Grid, List, LayoutGrid, Copy, Trash2, ExternalLink,
} from 'lucide-vue-next'
import { useAppStore } from '@/stores/app'
import { ENCRYPTION_INFO } from '@/types'
import type { FileNode, EncryptionAlgo } from '@/types'

const store = useAppStore()

const displayFiles = computed(() => {
  return store.currentFolderFiles
})

const contextMenu = ref({ visible: false, x: 0, y: 0, file: null as FileNode | null })

function getFileIcon(file: FileNode) {
  if (file.fileType === 'folder') return markRaw(Folder)
  if (file.encrypted) return markRaw(Lock)
  if (file.compressionLayers && file.compressionLayers[0] && file.compressionLayers[0] !== 'none') return markRaw(Archive)
  if (file.mimeType?.startsWith('image/')) return markRaw(Image)
  if (file.mimeType?.startsWith('text/') || file.mimeType?.includes('json') || file.mimeType?.includes('xml')) return markRaw(Code)
  return markRaw(File)
}

function getIconColor(file: FileNode): string {
  if (file.fileType === 'folder') return 'text-gold'
  if (file.encrypted) return 'text-neon'
  if (file.compressionLayers && file.compressionLayers[0] && file.compressionLayers[0] !== 'none') return 'text-gold'
  if (file.mimeType?.startsWith('image/')) return 'text-pink'
  if (file.mimeType?.startsWith('text/') || file.mimeType?.includes('json')) return 'text-cyan'
  return 'text-secondary'
}

function getEncryptionColor(file: FileNode): string {
  if (!file.encryptionAlgorithm) return 'var(--cyber-matrix-green)'
  return ENCRYPTION_INFO[file.encryptionAlgorithm as EncryptionAlgo]?.color || 'var(--cyber-matrix-green)'
}

function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const k = 1024
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + units[i]
}

function formatDate(dateStr: string): string {
  if (!dateStr) return '—'
  const d = new Date(dateStr)
  return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })
}

function handleDoubleClick(file: FileNode) {
  if (file.fileType === 'folder') {
    store.selectFile(file.id)
  }
  // File preview would open in the preview panel
  store.selectFile(file.id)
}

function showContextMenu(e: MouseEvent, file: FileNode) {
  contextMenu.value = {
    visible: true,
    x: e.clientX,
    y: e.clientY,
    file,
  }
}

function handleContextAction(action: string) {
  const file = contextMenu.value.file
  if (!file) return

  switch (action) {
    case 'open':
      handleDoubleClick(file)
      break
    case 'preview':
      store.selectedFileId = file.id
      break
    case 'encrypt':
      store.encryptFile(file.id, 'hybrid')
      break
    case 'compress':
      store.compressFile(file.id, 'zstd')
      break
    case 'star':
      store.toggleStar(file.id)
      break
    case 'duplicate':
      store.duplicateFileContext(file.id)
      break
    case 'delete':
      store.deleteFile(file.id)
      break
  }

  contextMenu.value.visible = false
}

function closeContextMenu() {
  contextMenu.value.visible = false
}

onMounted(() => {
  document.addEventListener('click', closeContextMenu)
})

onUnmounted(() => {
  document.removeEventListener('click', closeContextMenu)
})
</script>

<style scoped>
.file-grid-area {
  grid-area: content;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--cyber-bg-deep);
  position: relative;
}

/* Toolbar */
.file-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 38px;
  padding: 0 10px;
  border-bottom: 2px solid var(--cyber-bg-hover);
  background: var(--cyber-bg-panel);
  flex-shrink: 0;
}

.toolbar-left, .toolbar-right {
  display: flex;
  align-items: center;
  gap: 4px;
}

.toolbar-info {
  font-size: 11px;
}

.view-toggle {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 2px;
  cursor: pointer;
  color: var(--cyber-text-muted);
  transition: all 0.1s;
}

.view-toggle:hover {
  background: var(--cyber-bg-hover);
  color: var(--cyber-text-primary);
}

.view-toggle.active {
  background: var(--cyber-bg-card);
  color: var(--cyber-matrix-green);
  border: 2px solid rgba(0, 255, 65, 0.3);
}

/* Grid View */
.grid-view {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 12px;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
  gap: 10px;
  align-content: start;
}

.file-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 16px 10px 10px;
  cursor: pointer;
  background: var(--cyber-bg-card);
  border: 3px solid #000000;
  border-radius: 2px;
  box-shadow: 4px 4px 0 #000000;
  transition: all 0.15s ease;
  min-height: 130px;
  position: relative;
}

.file-card:hover {
  box-shadow: 6px 6px 0 #000000;
  transform: translate(-1px, -1px);
}

.file-card-selected {
  border-color: var(--cyber-matrix-green);
  box-shadow: 4px 4px 0 var(--cyber-matrix-green), 0 0 10px rgba(0, 255, 65, 0.15);
}

.file-card-icon {
  margin-bottom: 8px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.file-card-name {
  font-size: 11px;
  font-weight: 600;
  color: var(--cyber-text-primary);
  text-align: center;
  width: 100%;
}

.file-card-meta {
  font-size: 10px;
  font-family: monospace;
  margin-top: 2px;
}

.file-card-badges {
  display: flex;
  gap: 2px;
  margin-top: 6px;
  position: absolute;
  top: 4px;
  right: 6px;
}

.card-badge {
  font-size: 10px;
  line-height: 1;
}

/* List View */
.list-view {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
}

.list-header {
  display: flex;
  align-items: center;
  height: 32px;
  padding: 0 12px;
  background: var(--cyber-bg-panel);
  border-bottom: 2px solid var(--cyber-bg-hover);
  font-size: 10px;
  font-family: monospace;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--cyber-text-muted);
  position: sticky;
  top: 0;
  z-index: 2;
}

.list-row {
  display: flex;
  align-items: center;
  height: 34px;
  padding: 0 12px;
  border-bottom: 1px solid rgba(37, 37, 64, 0.5);
  cursor: pointer;
  transition: background 0.1s;
  font-size: 12px;
}

.list-row:hover {
  background: var(--cyber-bg-hover);
}

.list-row-selected {
  background: rgba(0, 255, 65, 0.06);
  border-left: 3px solid var(--cyber-matrix-green);
}

.list-col {
  display: flex;
  align-items: center;
  gap: 6px;
  overflow: hidden;
  white-space: nowrap;
}

.list-col-name {
  flex: 3;
  min-width: 0;
}

.list-col-size {
  flex: 1;
  min-width: 60px;
  justify-content: flex-end;
}

.list-col-type {
  flex: 1.2;
  min-width: 80px;
}

.list-col-date {
  flex: 1.2;
  min-width: 90px;
}

.list-col-status {
  flex: 0.8;
  min-width: 60px;
  gap: 3px;
}

.list-col-hash {
  flex: 1.5;
  min-width: 100px;
  font-size: 10px;
}

/* Empty states */
.empty-grid, .empty-list {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 60px 20px;
  font-size: 13px;
}

/* Utilities */
.mono { font-family: 'Courier New', monospace; }
.text-neon { color: var(--cyber-matrix-green); }
.text-gold { color: var(--cyber-saffron-gold); }
.text-cyan { color: var(--cyber-cyber-blue); }
.text-pink { color: var(--cyber-lotus-pink); }
.text-purple { color: var(--cyber-cyber-purple); }
.text-secondary { color: var(--cyber-text-secondary); }
.text-muted { color: var(--cyber-text-muted); }
.truncate { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

.badge {
  display: inline-flex;
  align-items: center;
  padding: 0 3px;
  font-size: 9px;
  border-radius: 2px;
  border: 1px solid;
  line-height: 16px;
}

.badge-green {
  color: var(--cyber-matrix-green);
  border-color: var(--cyber-matrix-green);
  background: rgba(0, 255, 65, 0.08);
}

.badge-gold {
  color: var(--cyber-saffron-gold);
  border-color: var(--cyber-saffron-gold);
  background: rgba(255, 184, 0, 0.08);
}

.badge-cyan {
  color: var(--cyber-cyber-blue);
  border-color: var(--cyber-cyber-blue);
  background: rgba(0, 212, 255, 0.08);
}
</style>