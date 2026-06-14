<template>
  <main class="file-grid">
    <div class="file-toolbar">
      <div class="ft-left">
        <button class="view-toggle" :class="{ active: store.viewMode === 'grid' }" @click="store.viewMode = 'grid'" title="GRID (CTRL+G)">[##]</button>
        <button class="view-toggle" :class="{ active: store.viewMode === 'list' }" @click="store.viewMode = 'list'" title="LIST (CTRL+L)">[#]</button>
        <button class="view-toggle" :class="{ active: store.viewMode === 'masonry' }" @click="store.viewMode = 'masonry'" title="MASONRY (CTRL+M)">[&]</button>
        <div class="ft-div" />
        <select v-model="sortField" class="sort-select" title="SORT BY" aria-label="SORT BY">
          <option value="name">NAME</option>
          <option value="size">SIZE</option>
          <option value="date">DATE</option>
          <option value="type">TYPE</option>
        </select>
        <button class="view-toggle" @click="sortDir = sortDir === 'asc' ? 'desc' : 'asc'" :title="sortDir === 'asc' ? 'ASCENDING' : 'DESCENDING'" :aria-label="sortDir">
          {{ sortDir === 'asc' ? '^' : 'v' }}
        </button>
      </div>
      <div class="ft-center">
        <span class="ft-info">{{ sortedFiles.length }} ITEMS</span>
        <span v-if="store.isMultiSelect && selectedCount" class="ft-info" style="margin-left:8px;color:#FFFFFF;">{{ selectedCount }} SEL</span>
      </div>
      <div class="ft-right">
        <input
          v-model="filterQuery"
          class="filter-input"
          placeholder="FILTER..."
          title="FILTER FILES IN CURRENT DIRECTORY"
          aria-label="FILTER FILES"
        />
        <span v-if="store.isLoading" class="text-muted" role="status" aria-live="polite">LOADING..</span>
      </div>
    </div>

    <div v-if="store.isMultiSelect && selectedCount" class="bulk-toolbar" role="toolbar" aria-label="BULK ACTIONS">
      <span class="bulk-label">{{ selectedCount }} SELECTED</span>
      <button class="bulk-btn" @click="execBulk('encrypt')" title="ENCRYPT ALL" aria-label="ENCRYPT SELECTED">[ENC]</button>
      <button class="bulk-btn" @click="execBulk('compress')" title="COMPRESS ALL" aria-label="COMPRESS SELECTED">[CMP]</button>
      <button class="bulk-btn" @click="execBulk('star')" title="STAR ALL" aria-label="STAR SELECTED">[*]</button>
      <button class="bulk-btn danger" @click="execBulk('delete')" title="DELETE ALL" aria-label="DELETE SELECTED">[DEL]</button>
      <button class="bulk-btn" @click="clearSelection" aria-label="CLEAR SELECTION">[CLEAR]</button>
    </div>

    <div v-if="store.viewMode === 'grid'" class="grid-view" role="grid" aria-label="FILE GRID" @contextmenu.prevent="ctx.open($event, 'file_grid_bg')">
      <div
        v-for="file in sortedFiles"
        :key="file.id"
        class="file-card"
        :class="{ selected: store.selectedFileId === file.id, 'bulk-selected': store.selectedFileIds.includes(file.id) }"
        @click="handleClick(file)"
        @dblclick="handleDoubleClick(file)"
        @contextmenu.prevent="showContextMenu($event, file)"
        @touchstart="touchStartCtx($event, file)"
        @touchend="touchEndCtx($event)"
        @touchmove="touchMoveCtx($event)"
        @mouseenter="showTooltip($event, file)"
        @mousemove="moveTooltip($event)"
        @mouseleave="hideTooltip"
        :draggable="true"
        @dragstart="onDragStart($event, file)"
        @dragover.prevent
        role="gridcell"
        :aria-label="file.name"
        :aria-selected="store.selectedFileId === file.id"
      >
        <div class="file-card-select" @click.stop="toggleSelect(file.id)" aria-hidden="true">
          <div class="check-box" :class="{ checked: store.selectedFileIds.includes(file.id) }">[{{ store.selectedFileIds.includes(file.id) ? 'X' : ' ' }}]</div>
        </div>
        <div class="file-card-icon" v-if="file.thumbnailPath" aria-hidden="true">
          <img :src="file.thumbnailPath" class="file-thumb" alt="" @error="(e) => { (e.target as HTMLImageElement).style.display = 'none' }" />
        </div>
        <div class="file-card-icon" v-else aria-hidden="true">
          <span class="file-icon">{{ getIcon(file) }}</span>
        </div>
        <div class="file-card-name truncate" :title="file.name">{{ file.name }}</div>
        <div class="file-card-meta text-muted">{{ formatSize(file.sizeBytes) }}</div>
        <div class="file-card-badges" aria-hidden="true">
          <span v-if="file.encrypted" class="card-badge" title="ENCRYPTED">[E]</span>
          <span v-if="file.compressionLayers && file.compressionLayers[0] && file.compressionLayers[0] !== 'none'" class="card-badge" title="COMPRESSED">[C]</span>
          <span v-if="file.isStarred" class="card-badge" title="STARRED">[*]</span>
          <span v-if="file.gpsLat" class="card-badge" title="HAS GPS">[G]</span>
          <span v-if="file.faceGroupIds && file.faceGroupIds.length" class="card-badge" title="HAS FACES">[F]</span>
        </div>
      </div>
      <div v-if="sortedFiles.length === 0 && !store.isLoading" class="empty-grid">
        <span class="text-muted" role="status">NO FILES IN THIS DIRECTORY</span>
      </div>
    </div>

    <div v-if="store.viewMode === 'masonry'" class="masonry-view" role="grid" aria-label="MASONRY VIEW" @contextmenu.prevent="ctx.open($event, 'file_grid_bg')">
      <div
        v-for="file in sortedFiles"
        :key="file.id"
        class="masonry-item"
        :class="{ selected: store.selectedFileId === file.id, 'bulk-selected': store.selectedFileIds.includes(file.id) }"
        @click="handleClick(file)"
        @dblclick="handleDoubleClick(file)"
        @contextmenu.prevent="showContextMenu($event, file)"
        @touchstart="touchStartCtx($event, file)"
        @touchend="touchEndCtx($event)"
        @touchmove="touchMoveCtx($event)"
        @mouseenter="showTooltip($event, file)"
        @mousemove="moveTooltip($event)"
        @mouseleave="hideTooltip"
        :draggable="true"
        @dragstart="onDragStart($event, file)"
        @dragover.prevent
        :aria-label="file.name"
        :aria-selected="store.selectedFileId === file.id"
      >
        <div class="masonry-select" @click.stop="toggleSelect(file.id)" aria-hidden="true">
          <div class="check-box" :class="{ checked: store.selectedFileIds.includes(file.id) }">[{{ store.selectedFileIds.includes(file.id) ? 'X' : ' ' }}]</div>
        </div>
        <div class="masonry-content">
          <div class="masonry-icon" v-if="file.thumbnailPath" aria-hidden="true">
            <img :src="file.thumbnailPath" class="file-thumb" alt="" @error="(e) => { (e.target as HTMLImageElement).style.display = 'none' }" />
          </div>
          <div class="masonry-icon" v-else aria-hidden="true">
            <span class="file-icon">{{ getIcon(file) }}</span>
          </div>
          <div class="masonry-name truncate" :title="file.name">{{ file.name }}</div>
          <div class="masonry-meta text-muted">{{ formatSize(file.sizeBytes) }}</div>
          <div class="masonry-badges" aria-hidden="true">
            <span v-if="file.encrypted" class="card-badge" title="ENCRYPTED">[E]</span>
            <span v-if="file.compressionLayers && file.compressionLayers[0] && file.compressionLayers[0] !== 'none'" class="card-badge" title="COMPRESSED">[C]</span>
            <span v-if="file.isStarred" class="card-badge" title="STARRED">[*]</span>
          </div>
        </div>
      </div>
      <div v-if="sortedFiles.length === 0 && !store.isLoading" class="empty-grid">
        <span class="text-muted" role="status">NO FILES IN THIS DIRECTORY</span>
      </div>
    </div>

    <div v-if="store.viewMode === 'list'" class="list-view" role="grid" aria-label="FILE LIST" @contextmenu.prevent="ctx.open($event, 'file_grid_bg')">
      <div class="list-header" role="row">
        <span class="lc lc-check" @click="toggleSelectAll" role="columnheader" aria-label="SELECT ALL">
          <div class="check-box" :class="{ checked: allSelected }">[{{ allSelected ? 'X' : ' ' }}]</div>
        </span>
        <span class="lc lc-name" @click="setSort('name')" role="columnheader" :aria-label="'SORT BY NAME ' + (sortField === 'name' ? (sortDir === 'asc' ? 'ASC' : 'DESC') : '')">NAME {{ sortField === 'name' ? (sortDir === 'asc' ? '^' : 'v') : '' }}</span>
        <span class="lc lc-size" @click="setSort('size')" role="columnheader" :aria-label="'SORT BY SIZE ' + (sortField === 'size' ? (sortDir === 'asc' ? 'ASC' : 'DESC') : '')">SIZE {{ sortField === 'size' ? (sortDir === 'asc' ? '^' : 'v') : '' }}</span>
        <span class="lc lc-type" @click="setSort('type')" role="columnheader" :aria-label="'SORT BY TYPE ' + (sortField === 'type' ? (sortDir === 'asc' ? 'ASC' : 'DESC') : '')">TYPE {{ sortField === 'type' ? (sortDir === 'asc' ? '^' : 'v') : '' }}</span>
        <span class="lc lc-date" @click="setSort('date')" role="columnheader" :aria-label="'SORT BY DATE ' + (sortField === 'date' ? (sortDir === 'asc' ? 'ASC' : 'DESC') : '')">MODIFIED {{ sortField === 'date' ? (sortDir === 'asc' ? '^' : 'v') : '' }}</span>
        <span class="lc lc-status" role="columnheader">FLAGS</span>
        <span class="lc lc-hash" role="columnheader">BLAKE3</span>
      </div>
      <div
        v-for="file in sortedFiles"
        :key="file.id"
        class="list-row"
        :class="{ selected: store.selectedFileId === file.id, 'bulk-selected': store.selectedFileIds.includes(file.id) }"
        @click="handleClick(file)"
        @dblclick="handleDoubleClick(file)"
        @contextmenu.prevent="showContextMenu($event, file)"
        @touchstart="touchStartCtx($event, file)"
        @touchend="touchEndCtx($event)"
        @touchmove="touchMoveCtx($event)"
        @mouseenter="showTooltip($event, file)"
        @mousemove="moveTooltip($event)"
        @mouseleave="hideTooltip"
        :draggable="true"
        @dragstart="onDragStart($event, file)"
        @dragover.prevent
        role="row"
        :aria-label="file.name"
        :aria-selected="store.selectedFileId === file.id"
      >
        <span class="lc lc-check" @click.stop="toggleSelect(file.id)" aria-hidden="true">
          <div class="check-box" :class="{ checked: store.selectedFileIds.includes(file.id) }">[{{ store.selectedFileIds.includes(file.id) ? 'X' : ' ' }}]</div>
        </span>
        <span class="lc lc-name">
          <span v-if="file.thumbnailPath" class="thumb-sm" aria-hidden="true">
            <img :src="file.thumbnailPath" class="file-thumb-sm" alt="" @error="(e) => { (e.target as HTMLImageElement).style.display = 'none' }" />
          </span>
          <span v-else class="file-icon-sm" aria-hidden="true">{{ getIcon(file) }}</span>
          <span class="truncate">{{ file.name }}</span>
        </span>
        <span class="lc lc-size text-muted">{{ formatSize(file.sizeBytes) }}</span>
        <span class="lc lc-type text-muted">{{ file.mimeType || (file.fileType === 'folder' ? 'FOLDER' : 'FILE') }}</span>
        <span class="lc lc-date text-muted">{{ formatDate(file.modifiedAt) }}</span>
        <span class="lc lc-status" aria-hidden="true">
          <span v-if="file.encrypted" class="badge-sm">[E]</span>
          <span v-if="file.compressionLayers && file.compressionLayers[0] && file.compressionLayers[0] !== 'none'" class="badge-sm">[C]</span>
          <span v-if="file.isStarred" class="badge-sm">[*]</span>
        </span>
        <span class="lc lc-hash text-muted">{{ file.hashBlake3 ? file.hashBlake3.substring(0, 8) + '..' : '--' }}</span>
      </div>
      <div v-if="sortedFiles.length === 0 && !store.isLoading" class="empty-list">
        <span class="text-muted" role="status">NO FILES</span>
      </div>
    </div>

    <FileTooltip :file="tooltipFile" :visible="tooltipVisible" :x="tooltipX" :y="tooltipY" />

    <Teleport to="body">
      <div
        v-if="showRenameDialog"
        class="rename-overlay"
        @click.self="showRenameDialog = false"
      >
        <div class="rename-modal">
          <div class="rename-header">RENAME FILE</div>
          <input
            ref="renameInputRef"
            v-model="renameValue"
            class="rename-input"
            @keyup.enter="handleRenameConfirm"
            @keyup.escape="showRenameDialog = false"
          />
          <div class="rename-actions">
            <button class="rename-btn" @click="showRenameDialog = false">[CANCEL]</button>
            <button class="rename-btn rename-btn-primary" @click="handleRenameConfirm">[RENAME]</button>
          </div>
        </div>
      </div>
    </Teleport>


  </main>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useAppStore } from '@/stores/app'
import { useContextMenu } from '@/composables/useContextMenu'
import { useDrag } from '@/composables/useDrag'
import type { FileNode } from '@/types'
import FileTooltip from './FileTooltip.vue'

const store = useAppStore()
const ctx = useContextMenu()
const drag = useDrag()

const sortField = ref<'name' | 'size' | 'date' | 'type'>('name')
const sortDir = ref<'asc' | 'desc'>('asc')
const filterQuery = ref('')

const tooltipVisible = ref(false)
const tooltipFile = ref<FileNode | null>(null)
const tooltipX = ref(0)
const tooltipY = ref(0)
let tooltipTimer: ReturnType<typeof setTimeout> | null = null

const showRenameDialog = ref(false)
const renameValue = ref('')
const renamingFileId = ref<string | null>(null)
const renameInputRef = ref<HTMLInputElement | null>(null)

const selectedCount = computed(() => store.selectedFileIds.length)

const allSelected = computed(() =>
  sortedFiles.value.length > 0 && sortedFiles.value.every(f => store.selectedFileIds.includes(f.id))
)

const sortedFiles = computed(() => {
  let files = store.currentFolderFiles
  if (filterQuery.value.trim()) {
    const q = filterQuery.value.toLowerCase()
    files = files.filter(f => f.name.toLowerCase().includes(q))
  }
  const sorted = [...files].sort((a, b) => {
    let cmp = 0
    if (sortField.value === 'name') cmp = a.name.localeCompare(b.name)
    else if (sortField.value === 'size') cmp = a.sizeBytes - b.sizeBytes
    else if (sortField.value === 'date') cmp = new Date(a.modifiedAt).getTime() - new Date(b.modifiedAt).getTime()
    else if (sortField.value === 'type') cmp = (a.mimeType || a.fileType || '').localeCompare(b.mimeType || b.fileType || '')
    return sortDir.value === 'asc' ? cmp : -cmp
  })
  return sorted
})

function setSort(field: 'name' | 'size' | 'date' | 'type') {
  if (sortField.value === field) {
    sortDir.value = sortDir.value === 'asc' ? 'desc' : 'asc'
  } else {
    sortField.value = field
    sortDir.value = 'asc'
  }
}

function toggleSelect(fileId: string) {
  const idx = store.selectedFileIds.indexOf(fileId)
  if (idx === -1) {
    store.selectedFileIds.push(fileId)
  } else {
    store.selectedFileIds.splice(idx, 1)
  }
  store.isMultiSelect = store.selectedFileIds.length > 0
}

function toggleSelectAll() {
  if (allSelected.value) {
    store.selectedFileIds = []
  } else {
    store.selectedFileIds = sortedFiles.value.map(f => f.id)
  }
  store.isMultiSelect = store.selectedFileIds.length > 0
}

function clearSelection() {
  store.selectedFileIds = []
  store.isMultiSelect = false
}

function handleClick(file: FileNode) {
  if (store.isMultiSelect) {
    toggleSelect(file.id)
  } else {
    store.selectedFileId = file.id
  }
}

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

function showTooltip(e: MouseEvent, file: FileNode) {
  if (tooltipTimer) clearTimeout(tooltipTimer)
  tooltipTimer = setTimeout(() => {
    tooltipFile.value = file
    tooltipX.value = e.clientX
    tooltipY.value = e.clientY
    tooltipVisible.value = true
  }, 600)
}

function moveTooltip(e: MouseEvent) {
  if (tooltipVisible.value) {
    tooltipX.value = e.clientX
    tooltipY.value = e.clientY
  }
}

function hideTooltip() {
  if (tooltipTimer) clearTimeout(tooltipTimer)
  tooltipVisible.value = false
  tooltipFile.value = null
}

function onDragStart(e: DragEvent, file: FileNode) {
  e.dataTransfer?.setData('text/plain', file.id)
  e.dataTransfer!.effectAllowed = 'copy'
  if (e.dataTransfer) {
    e.dataTransfer.effectAllowed = 'copy'
  }
}

function buildFileCtxEntries(file: FileNode) {
  const ft = file.fileType || 'file'
  const mime = file.mimeType || ''
  let typeSpecific: any[] = []
  if (ft === 'folder') {
    typeSpecific = [
      { id: 'open_in_new', label: 'OPEN IN NEW TAB', icon: '[T]', action: () => {} },
      { id: 'div_f1', label: '', divider: true },
      { id: 'paste_into', label: 'PASTE INTO', icon: '[P]', action: () => {} },
      { id: 'div_f2', label: '', divider: true },
    ]
  } else if (mime.startsWith('image/')) {
    typeSpecific = [
      { id: 'rotate_cw', label: 'ROTATE CW', icon: '[R]', action: () => store.notifySuccess('ROTATE CW: ' + file.name) },
      { id: 'rotate_ccw', label: 'ROTATE CCW', icon: '[L]', action: () => store.notifySuccess('ROTATE CCW: ' + file.name) },
      { id: 'div_i1', label: '', divider: true },
    ]
  } else if (mime.startsWith('audio/')) {
    typeSpecific = [
      { id: 'play', label: 'PLAY', icon: '[P]', action: () => store.notifySuccess('PLAY: ' + file.name) },
      { id: 'div_a1', label: '', divider: true },
    ]
  } else if (mime.startsWith('video/')) {
    typeSpecific = [
      { id: 'play', label: 'PLAY', icon: '[P]', action: () => store.notifySuccess('PLAY: ' + file.name) },
      { id: 'div_v1', label: '', divider: true },
    ]
  } else if (mime.includes('zip') || mime.includes('tar') || mime.includes('gz') || mime.includes('rar') || mime.includes('7z')) {
    typeSpecific = [
      { id: 'extract', label: 'EXTRACT HERE', icon: '[X]', action: () => store.notifySuccess('EXTRACT: ' + file.name) },
      { id: 'div_ar1', label: '', divider: true },
    ]
  }

  const shared = [
    { id: 'download', label: 'DOWNLOAD', icon: '[v]', action: () => store.notifySuccess('DOWNLOAD: ' + file.name) },
    { id: 'star', label: file.isStarred ? 'UNSTAR' : 'STAR', icon: '[*]', action: () => store.toggleStar(file.id) },
    { id: 'rename', label: 'RENAME', icon: '[R]', action: () => {
      renameValue.value = file.name
      renamingFileId.value = file.id
      showRenameDialog.value = true
      setTimeout(() => renameInputRef.value?.focus(), 50)
    }},
    { id: 'duplicate', label: 'DUPLICATE', icon: '[D]', action: () => store.duplicateFileContext?.(file.id) || store.notifySuccess('DUPLICATE: ' + file.name) },
  ]
  const transform: any[] = []
  if (file.encrypted) {
    transform.push({ id: 'decrypt', label: 'DECRYPT', icon: '[@]', action: () => store.notifySuccess('DECRYPT: ' + file.name) })
  }
  if (!file.encrypted) {
    transform.push({ id: 'compress', label: 'COMPRESS', icon: '[Z]', action: () => store.compressFile(file.id, 'zstd') })
    transform.push({ id: 'encrypt', label: 'ENCRYPT', icon: '[#]', action: () => store.encryptFile(file.id, 'hybrid') })
  }
  if (file.compressionLayers?.length) {
    transform.push({ id: 'decompress', label: 'DECOMPRESS', icon: '[$]', action: () => store.notifySuccess('DECOMPRESS: ' + file.name) })
  }

  const base = [
    { id: 'open', label: 'OPEN', icon: '[>]', action: () => { store.selectFile(file.id) } },
    { id: 'preview', label: 'PREVIEW', icon: '[=]', action: () => { store.selectedFileId = file.id } },
    { id: 'div0', label: '', divider: true },
  ]

  const meta = [
    { id: 'permissions', label: 'PERMISSIONS', icon: '[!]', action: () => { store.selectedFileId = file.id; store.showPermissionsPanel = true } },
    { id: 'properties', label: 'PROPERTIES', icon: '[i]', action: () => store.notifySuccess('PROPS: ' + file.name + ' | SIZE: ' + formatSize(file.sizeBytes) + ' | ' + (file.mimeType || '')) },
  ]
  const deleteAction = { id: 'delete', label: 'DELETE', icon: '[X]', action: () => store.deleteFile(file.id) }

  return [...base, ...typeSpecific, ...shared, { id: 'div_t1', label: '', divider: true }, ...transform, { id: 'div_m1', label: '', divider: true }, ...meta, { id: 'div_d1', label: '', divider: true }, deleteAction]
}

let longPressTimer: ReturnType<typeof setTimeout> | null = null
let longPressFileId: string | null = null

function touchStartCtx(e: TouchEvent, file: FileNode) {
  longPressTimer = setTimeout(() => {
    longPressFileId = file.id
    showContextMenu(e as unknown as MouseEvent, file)
    longPressTimer = null
  }, 600)
}

function touchEndCtx(e: TouchEvent) {
  if (longPressTimer) {
    clearTimeout(longPressTimer)
    longPressTimer = null
  }
}

function touchMoveCtx(e: TouchEvent) {
  if (longPressTimer) {
    clearTimeout(longPressTimer)
    longPressTimer = null
  }
}

function showContextMenu(e: MouseEvent, file: FileNode) {
  const entries = buildFileCtxEntries(file)
  ctx.replaceEntries('file_grid_item', entries)
  ctx.open(e, 'file_grid_item', {
    select: () => store.selectFile(file.id),
    preview: () => { store.selectedFileId = file.id },
    download: () => store.notifySuccess('DOWNLOAD: ' + file.name),
    star: () => store.toggleStar(file.id),
    rename: () => {
      renameValue.value = file.name
      renamingFileId.value = file.id
      showRenameDialog.value = true
      setTimeout(() => renameInputRef.value?.focus(), 50)
    },
    compress: () => store.compressFile(file.id, 'zstd'),
    encrypt: () => store.encryptFile(file.id, 'hybrid'),
    decrypt: () => store.notifySuccess('DECRYPT: ' + file.name),
    decompress: () => store.notifySuccess('DECOMPRESS: ' + file.name),
    permissions: () => { store.selectedFileId = file.id; store.showPermissionsPanel = true },
    properties: () => store.notifySuccess('PROPS: ' + file.name + ' | SIZE: ' + formatSize(file.sizeBytes) + ' | ' + file.mimeType || ''),
    delete: () => store.deleteFile(file.id),
    duplicate: () => store.duplicateFileContext?.(file.id) || store.notifySuccess('DUPLICATE: ' + file.name),
    rotate: (dir: string) => store.notifySuccess(`ROTATE ${dir}: ` + file.name),
    play: () => store.notifySuccess('PLAY: ' + file.name),
    extract: () => store.notifySuccess('EXTRACT: ' + file.name),
    pasteInto: () => {},
    openNew: () => {},
  })
}

async function execBulk(action: string) {
  const ids = [...store.selectedFileIds]
  for (const id of ids) {
    try {
      switch (action) {
        case 'encrypt': await store.encryptFile(id, 'hybrid'); break
        case 'compress': await store.compressFile(id, 'zstd'); break
        case 'star': store.toggleStar(id); break
        case 'delete': await store.deleteFile(id); break
      }
    } catch {}
  }
  clearSelection()
}

async function handleRenameConfirm() {
  if (renamingFileId.value && renameValue.value.trim()) {
    await store.renameFile(renamingFileId.value, renameValue.value.trim())
  }
  showRenameDialog.value = false
  renamingFileId.value = null
}
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

.ft-div {
  width: 1px;
  height: 14px;
  background: rgba(255,255,255,0.3);
  margin: 0 4px;
}

.ft-info {
  font-family: 'Courier New', monospace;
  font-size: 10px;
  color: rgba(255,255,255,0.5);
}

.sort-select {
  background: #000;
  border: 2px solid #FFFFFF;
  color: #FFFFFF;
  font-family: 'Courier New', monospace;
  font-size: 9px;
  padding: 2px 4px;
  cursor: pointer;
}

.filter-input {
  background: #000;
  border: 2px solid #FFFFFF;
  color: #FFFFFF;
  font-family: 'Courier New', monospace;
  font-size: 9px;
  padding: 2px 6px;
  width: 100px;
}

.filter-input::placeholder {
  color: rgba(255,255,255,0.3);
}

.bulk-toolbar {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  background: #FFFFFF;
  border-bottom: 2px solid #000000;
  flex-shrink: 0;
}

.bulk-label {
  font-family: 'Courier New', monospace;
  font-size: 10px;
  font-weight: 700;
  color: #000000;
  margin-right: 8px;
}

.bulk-btn {
  padding: 2px 8px;
  font-family: 'Courier New', monospace;
  font-size: 9px;
  font-weight: 700;
  cursor: pointer;
  border: 2px solid #000000;
  background: #FFFFFF;
  color: #000000;
}

.bulk-btn:hover {
  background: #000000;
  color: #FFFFFF;
}

.bulk-btn.danger:hover {
  background: #000000;
  color: #FFFFFF;
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

.file-card.bulk-selected {
  border-width: 3px;
  border-color: #FFFFFF;
  background: rgba(255,255,255,0.08);
}

.file-card-select {
  position: absolute;
  top: 4px;
  left: 4px;
  z-index: 2;
}

.check-box {
  font-family: 'Courier New', monospace;
  font-size: 8px;
  color: #FFFFFF;
  border: 1px solid #FFFFFF;
  padding: 0 1px;
  line-height: 12px;
  cursor: pointer;
}

.check-box.checked {
  background: #FFFFFF;
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

.file-thumb {
  max-width: 60px;
  max-height: 60px;
  border: 1px solid #FFFFFF;
}

.thumb-sm {
  width: 18px;
  height: 18px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}

.file-thumb-sm {
  max-width: 18px;
  max-height: 18px;
  border: 1px solid #FFFFFF;
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

.masonry-view {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 10px;
  columns: 140px;
  column-gap: 8px;
}

.masonry-item {
  display: inline-block;
  width: 100%;
  margin-bottom: 8px;
  break-inside: avoid;
  cursor: pointer;
  border: 2px solid #FFFFFF;
  background: #000;
  position: relative;
}

.masonry-item:hover {
  background: #FFFFFF;
}

.masonry-item:hover .masonry-name,
.masonry-item:hover .masonry-meta {
  color: #000;
}

.masonry-item.selected {
  background: #FFFFFF;
}

.masonry-item.selected .masonry-name,
.masonry-item.selected .masonry-meta,
.masonry-item.selected .masonry-icon .file-icon {
  color: #000;
}

.masonry-item.bulk-selected {
  border-width: 3px;
}

.masonry-select {
  position: absolute;
  top: 4px;
  left: 4px;
  z-index: 2;
}

.masonry-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 16px 10px 10px;
}

.masonry-icon {
  margin-bottom: 8px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.masonry-name {
  font-size: 10px;
  font-weight: 600;
  color: #FFFFFF;
  text-align: center;
  width: 100%;
}

.masonry-meta {
  font-size: 9px;
  font-family: 'Courier New', monospace;
  margin-top: 4px;
}

.masonry-badges {
  display: flex;
  gap: 2px;
  margin-top: 6px;
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

.list-header .lc {
  cursor: pointer;
}

.list-header .lc:hover {
  color: #FFFFFF;
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

.list-row.bulk-selected {
  background: rgba(255,255,255,0.08);
}

.lc {
  display: flex;
  align-items: center;
  gap: 4px;
  overflow: hidden;
  white-space: nowrap;
}

.lc-check { flex: 0 0 24px; justify-content: center; }
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

.context-menu {
  position: fixed;
  z-index: 1000;
  background: #000;
  border: 2px solid #FFFFFF;
  min-width: 180px;
  font-family: 'Courier New', monospace;
  font-size: 10px;
  color: #FFFFFF;
  box-shadow: 4px 4px 0 rgba(255,255,255,0.15);
}

.context-menu-item {
  position: relative;
  padding: 6px 12px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.context-menu-item:hover {
  background: #FFFFFF;
  color: #000;
}

.context-menu-item.danger:hover {
  background: #FFFFFF;
  color: #000;
}

.context-menu-item.disabled {
  opacity: 0.3;
  cursor: default;
}

.context-menu-item:hover .submenu {
  display: block;
}

.submenu {
  display: none;
  position: absolute;
  left: 100%;
  top: -2px;
  background: #000;
  border: 2px solid #FFFFFF;
  min-width: 220px;
  z-index: 1001;
  box-shadow: 4px 4px 0 rgba(255,255,255,0.15);
}

.submenu-right {
  left: auto;
  right: 100%;
}

.context-menu-divider {
  height: 1px;
  background: rgba(255,255,255,0.2);
  margin: 2px 0;
}

.rename-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0,0,0,0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.rename-modal {
  background: #000;
  border: 2px solid #FFFFFF;
  padding: 16px;
  width: 300px;
  font-family: 'Courier New', monospace;
  color: #FFFFFF;
  box-shadow: 4px 4px 0 rgba(255,255,255,0.15);
}

.rename-header {
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 1px;
  margin-bottom: 10px;
}

.rename-input {
  width: 100%;
  background: #000;
  border: 2px solid #FFFFFF;
  color: #FFFFFF;
  font-family: 'Courier New', monospace;
  font-size: 11px;
  padding: 6px 8px;
  margin-bottom: 10px;
}

.rename-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.rename-btn {
  background: transparent;
  border: 2px solid #FFFFFF;
  color: #FFFFFF;
  padding: 4px 12px;
  font-family: 'Courier New', monospace;
  font-size: 10px;
  font-weight: 700;
  cursor: pointer;
}

.rename-btn:hover {
  background: #FFFFFF;
  color: #000;
}

.rename-btn-primary {
  background: #FFFFFF;
  color: #000;
}

.rename-btn-primary:hover {
  background: #000;
  color: #FFFFFF;
}

.text-muted { color: rgba(255,255,255,0.5) !important; }
.truncate { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
