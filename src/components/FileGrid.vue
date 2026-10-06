<template>
  <main class="file-grid">
    <div class="file-toolbar">
      <div class="ft-left">
        <button class="view-toggle" :class="{ active: store.viewMode === 'grid' }" @click="store.viewMode = 'grid'" title="GRID (CTRL+G)" aria-label="GRID VIEW"><AppIcon name="solar:grid-3x3-bold" :size="13" /></button>
        <button class="view-toggle" :class="{ active: store.viewMode === 'list' }" @click="store.viewMode = 'list'" title="LIST (CTRL+L)" aria-label="LIST VIEW"><AppIcon name="solar:list-bold" :size="13" /></button>
        <button class="view-toggle" :class="{ active: store.viewMode === 'masonry' }" @click="store.viewMode = 'masonry'" title="MASONRY (CTRL+M)" aria-label="MASONRY VIEW"><AppIcon name="solar:columns-3-bold" :size="13" /></button>
        <div class="ft-div" />
        <select v-model="sortField" class="sort-select" title="SORT BY" aria-label="SORT BY">
          <option value="name">NAME</option>
          <option value="size">SIZE</option>
          <option value="date">DATE</option>
          <option value="type">TYPE</option>
        </select>
        <button class="view-toggle" @click="sortDir = sortDir === 'asc' ? 'desc' : 'asc'" :title="sortDir === 'asc' ? 'ASCENDING' : 'DESCENDING'" :aria-label="sortDir">
          <AppIcon :name="sortDir === 'asc' ? 'solar:alt-arrow-up-bold' : 'solar:alt-arrow-down-bold'" :size="13" />
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
      <button class="bulk-btn" @click="execBulk('encrypt')" title="ENCRYPT ALL" aria-label="ENCRYPT SELECTED"><AppIcon name="solar:lock-bold" :size="13" /></button>
      <button class="bulk-btn" @click="execBulk('compress')" title="COMPRESS ALL" aria-label="COMPRESS SELECTED"><AppIcon name="solar:archive-bold" :size="13" /></button>
      <button class="bulk-btn" @click="execBulk('star')" title="STAR ALL" aria-label="STAR SELECTED"><AppIcon name="solar:star-bold" :size="13" /></button>
      <button class="bulk-btn danger" @click="execBulk('delete')" title="DELETE ALL" aria-label="DELETE SELECTED"><AppIcon name="solar:trash-bin-trash-bold" :size="13" /></button>
      <button class="bulk-btn" @click="clearSelection" aria-label="CLEAR SELECTION" title="CLEAR SELECTION"><AppIcon name="solar:close-square-bold" :size="13" /></button>
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
          <span class="file-icon"><AppIcon :name="getIcon(file)" :size="18" /></span>
        </div>
        <div class="file-card-name truncate" :title="file.name">{{ file.name }}</div>
        <div class="file-card-meta text-muted">{{ humanBytes(file.sizeBytes) }}</div>
        <div class="file-card-badges" aria-hidden="true">
          <span v-if="file.encrypted" class="card-badge" title="ENCRYPTED" aria-label="ENCRYPTED"><AppIcon name="solar:lock-bold" :size="10" /></span>
          <span v-if="file.compressionLayers && file.compressionLayers[0] && file.compressionLayers[0] !== 'none'" class="card-badge" title="COMPRESSED" aria-label="COMPRESSED"><AppIcon name="solar:archive-bold" :size="10" /></span>
          <span v-if="file.isStarred" class="card-badge" title="STARRED" aria-label="STARRED"><AppIcon name="solar:star-bold" :size="10" /></span>
          <span v-if="file.gpsLat" class="card-badge" title="HAS GPS" aria-label="HAS GPS"><AppIcon name="solar:map-point-bold" :size="10" /></span>
          <span v-if="file.faceGroupIds && file.faceGroupIds.length" class="card-badge" title="HAS FACES" aria-label="HAS FACES"><AppIcon name="solar:face-scan-circle-bold" :size="10" /></span>
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
            <span class="file-icon"><AppIcon :name="getIcon(file)" :size="18" /></span>
          </div>
          <div class="masonry-name truncate" :title="file.name">{{ file.name }}</div>
          <div class="masonry-meta text-muted">{{ humanBytes(file.sizeBytes) }}</div>
          <div class="masonry-badges" aria-hidden="true">
            <span v-if="file.encrypted" class="card-badge" title="ENCRYPTED" aria-label="ENCRYPTED"><AppIcon name="solar:lock-bold" :size="10" /></span>
            <span v-if="file.compressionLayers && file.compressionLayers[0] && file.compressionLayers[0] !== 'none'" class="card-badge" title="COMPRESSED" aria-label="COMPRESSED"><AppIcon name="solar:archive-bold" :size="10" /></span>
            <span v-if="file.isStarred" class="card-badge" title="STARRED" aria-label="STARRED"><AppIcon name="solar:star-bold" :size="10" /></span>
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
          <span v-else class="file-icon-sm" aria-hidden="true"><AppIcon :name="getIcon(file)" :size="14" /></span>
          <span class="truncate">{{ file.name }}</span>
        </span>
        <span class="lc lc-size text-muted">{{ humanBytes(file.sizeBytes) }}</span>
        <span class="lc lc-type text-muted">{{ file.mimeType || (file.fileType === 'folder' ? 'FOLDER' : 'FILE') }}</span>
        <span class="lc lc-date text-muted">{{ formatDate(file.modifiedAt) }}</span>
        <span class="lc lc-status" aria-hidden="true">
          <span v-if="file.encrypted" class="badge-sm" title="ENCRYPTED" aria-label="ENCRYPTED"><AppIcon name="solar:lock-bold" :size="10" /></span>
          <span v-if="file.compressionLayers && file.compressionLayers[0] && file.compressionLayers[0] !== 'none'" class="badge-sm" title="COMPRESSED" aria-label="COMPRESSED"><AppIcon name="solar:archive-bold" :size="10" /></span>
          <span v-if="file.isStarred" class="badge-sm" title="STARRED" aria-label="STARRED"><AppIcon name="solar:star-bold" :size="10" /></span>
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
            <button class="rename-btn" @click="showRenameDialog = false"><AppIcon name="solar:close-bold" :size="13" /> CANCEL</button>
            <button class="rename-btn rename-btn-primary" @click="handleRenameConfirm"><AppIcon name="solar:pen-bold" :size="13" /> RENAME</button>
          </div>
        </div>
      </div>
    </Teleport>


  </main>
</template>

<script setup lang="ts">
import AppIcon from '@/components/AppIcon.vue'
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useAppStore } from '@/stores/app'
import { humanBytes } from '@/utils/format'
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
  if (file.fileType === 'folder') return 'solar:folder-bold'
  if (file.encrypted) return 'solar:lock-bold'
  if (file.compressionLayers && file.compressionLayers[0] && file.compressionLayers[0] !== 'none') return 'solar:archive-bold'
  if (file.mimeType?.startsWith('image/')) return 'solar:gallery-bold'
  if (file.mimeType?.startsWith('text/') || file.mimeType?.includes('json') || file.mimeType?.includes('xml')) return 'solar:file-text-bold'
  return 'solar:file-bold'
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
      { id: 'open_in_new', label: 'OPEN IN NEW TAB', icon: 'solar:square-arrow-right-up-bold', action: () => {} },
      { id: 'div_f1', label: '', divider: true },
      { id: 'paste_into', label: 'PASTE INTO', icon: 'solar:clipboard-paste-bold', action: () => {} },
      { id: 'div_f2', label: '', divider: true },
    ]
  } else if (mime.startsWith('image/')) {
    typeSpecific = [
      { id: 'rotate_cw', label: 'ROTATE CW', icon: 'solar:undo-right-round-bold', action: () => store.notifySuccess('ROTATE CW: ' + file.name) },
      { id: 'rotate_ccw', label: 'ROTATE CCW', icon: 'solar:undo-left-round-bold', action: () => store.notifySuccess('ROTATE CCW: ' + file.name) },
      { id: 'div_i1', label: '', divider: true },
    ]
  } else if (mime.startsWith('audio/')) {
    typeSpecific = [
      { id: 'play', label: 'PLAY', icon: 'solar:play-bold', action: () => store.notifySuccess('PLAY: ' + file.name) },
      { id: 'div_a1', label: '', divider: true },
    ]
  } else if (mime.startsWith('video/')) {
    typeSpecific = [
      { id: 'play', label: 'PLAY', icon: 'solar:play-bold', action: () => store.notifySuccess('PLAY: ' + file.name) },
      { id: 'div_v1', label: '', divider: true },
    ]
  } else if (mime.includes('zip') || mime.includes('tar') || mime.includes('gz') || mime.includes('rar') || mime.includes('7z')) {
    typeSpecific = [
      { id: 'extract', label: 'EXTRACT HERE', icon: 'solar:box-bold', action: () => store.notifySuccess('EXTRACT: ' + file.name) },
      { id: 'div_ar1', label: '', divider: true },
    ]
  }

  const shared = [
    { id: 'download', label: 'DOWNLOAD', icon: 'solar:download-bold', action: () => store.notifySuccess('DOWNLOAD: ' + file.name) },
    { id: 'star', label: file.isStarred ? 'UNSTAR' : 'STAR', icon: 'solar:star-bold', action: () => store.toggleStar(file.id) },
    { id: 'rename', label: 'RENAME', icon: 'solar:pen-bold', action: () => {
      renameValue.value = file.name
      renamingFileId.value = file.id
      showRenameDialog.value = true
      setTimeout(() => renameInputRef.value?.focus(), 50)
    }},
    { id: 'duplicate', label: 'DUPLICATE', icon: 'solar:copy-add-bold', action: () => store.duplicateFileContext?.(file.id) || store.notifySuccess('DUPLICATE: ' + file.name) },
  ]
  const transform: any[] = []
  if (file.encrypted) {
    transform.push({ id: 'decrypt', label: 'DECRYPT', icon: 'solar:lock-unlocked-bold', action: () => store.notifySuccess('DECRYPT: ' + file.name) })
  }
  if (!file.encrypted) {
    transform.push({ id: 'compress', label: 'COMPRESS', icon: 'solar:archive-bold', action: () => store.compressFile(file.id, 'zstd') })
    transform.push({ id: 'encrypt', label: 'ENCRYPT', icon: 'solar:lock-bold', action: () => store.encryptFile(file.id, 'hybrid') })
  }
  if (file.compressionLayers?.length) {
    transform.push({ id: 'decompress', label: 'DECOMPRESS', icon: 'solar:archive-up-bold', action: () => store.notifySuccess('DECOMPRESS: ' + file.name) })
  }

  const base = [
    { id: 'open', label: 'OPEN', icon: 'solar:folder-open-bold', action: () => { store.selectFile(file.id) } },
    { id: 'preview', label: 'PREVIEW', icon: 'solar:eye-bold', action: () => { store.selectedFileId = file.id } },
    { id: 'div0', label: '', divider: true },
  ]

  const meta = [
    { id: 'permissions', label: 'PERMISSIONS', icon: 'solar:key-bold', action: () => { store.selectedFileId = file.id; store.showPermissionsPanel = true } },
    { id: 'properties', label: 'PROPERTIES', icon: 'solar:info-circle-bold', action: () => store.notifySuccess('PROPS: ' + file.name + ' | SIZE: ' + humanBytes(file.sizeBytes) + ' | ' + (file.mimeType || '')) },
  ]
  const deleteAction = { id: 'delete', label: 'DELETE', icon: 'solar:trash-bin-trash-bold', action: () => store.deleteFile(file.id) }

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
    properties: () => store.notifySuccess('PROPS: ' + file.name + ' | SIZE: ' + humanBytes(file.sizeBytes) + ' | ' + file.mimeType || ''),
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
  background: var(--ui-surface);
  position: relative;
  height: 100%;
}

.file-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 32px;
  padding: 0 8px;
  border-bottom: 1px solid var(--ui-border);
  background: var(--ui-surface);
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
  background: color-mix(in srgb, var(--ui-text) 30%, transparent);
  margin: 0 4px;
}

.ft-info {
  font-family: var(--ui-font);
  font-size: 10px;
  color: color-mix(in srgb, var(--ui-text) 50%, transparent);
}

.sort-select {
  background: var(--ui-surface);
  border: 1px solid var(--ui-border);
  color: var(--ui-text);
  font-family: var(--ui-font);
  font-size: 9px;
  padding: 2px 4px;
  cursor: pointer;
}

.filter-input {
  background: var(--ui-surface);
  border: 1px solid var(--ui-border);
  color: var(--ui-text);
  font-family: var(--ui-font);
  font-size: 9px;
  padding: 2px 6px;
  width: 100px;
}

.filter-input::placeholder {
  color: color-mix(in srgb, var(--ui-text) 35%, transparent);
}

.bulk-toolbar {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  background: var(--ui-glass-2);
  border-bottom: 1px solid var(--ui-border);
  flex-shrink: 0;
}

.bulk-label {
  font-family: var(--ui-font);
  font-size: 10px;
  font-weight: 700;
  color: var(--ui-text);
  margin-right: 8px;
}

.bulk-btn {
  padding: 2px 8px;
  font-family: var(--ui-font);
  font-size: 9px;
  font-weight: 700;
  cursor: pointer;
  border: 1px solid var(--ui-border);
  background: var(--ui-glass-2);
  color: var(--ui-text);
}

.bulk-btn:hover {
  background: var(--ui-surface);
  color: var(--ui-text);
}

.bulk-btn.danger:hover {
  background: var(--ui-surface);
  color: var(--ui-text);
}

.view-toggle {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 2px 6px;
  cursor: pointer;
  color: color-mix(in srgb, var(--ui-text) 40%, transparent);
  font-family: var(--ui-font);
  font-size: 10px;
  font-weight: 700;
  border: 2px solid transparent;
}

.view-toggle:hover {
  color: var(--ui-text);
  border-color: var(--ui-border-strong);
}

.view-toggle.active {
  color: var(--ui-text);
  background: var(--ui-glass-2);
  border-color: var(--ui-border-strong);
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
  border: 1px solid var(--ui-border);
  background: var(--ui-surface);
  min-height: 110px;
  position: relative;
}

.file-card:hover {
  background: var(--ui-glass-2);
}

.file-card:hover .file-card-name,
.file-card:hover .file-card-meta {
  color: var(--ui-text);
}

.file-card.selected {
  background: var(--ui-glass-2);
}

.file-card.selected .file-card-name,
.file-card.selected .file-card-meta,
.file-card.selected .file-icon {
  color: var(--ui-text);
}

.file-card.bulk-selected {
  border-width: 3px;
  border-color: var(--ui-border-strong);
  background: color-mix(in srgb, var(--ui-text) 6%, transparent);
}

.file-card-select {
  position: absolute;
  top: 4px;
  left: 4px;
  z-index: 2;
}

.check-box {
  font-family: var(--ui-font);
  font-size: 8px;
  color: var(--ui-text);
  border: 1px solid var(--ui-border-strong);
  padding: 0 1px;
  line-height: 12px;
  cursor: pointer;
}

.check-box.checked {
  background: var(--ui-glass-2);
  color: var(--ui-text);
}

.file-card-icon {
  margin-bottom: 6px;
  height: 30px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.file-icon {
  font-family: var(--ui-font);
  font-size: 18px;
  color: var(--ui-text);
}

.file-thumb {
  max-width: 60px;
  max-height: 60px;
  border: 1px solid var(--ui-border-strong);
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
  border: 1px solid var(--ui-border-strong);
}

.file-card-name {
  font-size: 10px;
  font-weight: 600;
  color: var(--ui-text);
  text-align: center;
  width: 100%;
}

.file-card-meta {
  font-size: 9px;
  font-family: var(--ui-font);
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
  font-family: var(--ui-font);
  font-size: 8px;
  font-weight: 700;
  color: var(--ui-text);
  border: 1px solid var(--ui-border-strong);
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
  border: 1px solid var(--ui-border);
  background: var(--ui-surface);
  position: relative;
}

.masonry-item:hover {
  background: var(--ui-glass-2);
}

.masonry-item:hover .masonry-name,
.masonry-item:hover .masonry-meta {
  color: var(--ui-text);
}

.masonry-item.selected {
  background: var(--ui-glass-2);
}

.masonry-item.selected .masonry-name,
.masonry-item.selected .masonry-meta,
.masonry-item.selected .masonry-icon .file-icon {
  color: var(--ui-text);
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
  color: var(--ui-text);
  text-align: center;
  width: 100%;
}

.masonry-meta {
  font-size: 9px;
  font-family: var(--ui-font);
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
  background: var(--ui-surface);
  border-bottom: 1px solid var(--ui-border);
  font-size: 9px;
  font-family: var(--ui-font);
  font-weight: 700;
  letter-spacing: 0.5px;
  color: color-mix(in srgb, var(--ui-text) 50%, transparent);
  position: sticky;
  top: 0;
  z-index: 2;
}

.list-header .lc {
  cursor: pointer;
}

.list-header .lc:hover {
  color: var(--ui-text);
}

.list-row {
  display: flex;
  align-items: center;
  height: 30px;
  padding: 0 10px;
  border-bottom: 1px solid var(--ui-border);
  cursor: pointer;
  font-size: 11px;
  color: var(--ui-text);
}

.list-row:hover {
  background: color-mix(in srgb, var(--ui-text) 10%, transparent);
}

.list-row.selected {
  background: var(--ui-glass-2);
  color: var(--ui-text);
}

.list-row.selected .text-muted {
  color: var(--ui-text) !important;
  opacity: 0.6;
}

.list-row.bulk-selected {
  background: color-mix(in srgb, var(--ui-text) 6%, transparent);
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
.lc-hash { flex: 1.2; min-width: 80px; font-size: 9px; font-family: var(--ui-font); }

.file-icon-sm {
  font-family: var(--ui-font);
  font-size: 11px;
  color: var(--ui-text);
  flex-shrink: 0;
  width: 18px;
  text-align: center;
}

.badge-sm {
  font-size: 8px;
  font-family: var(--ui-font);
  font-weight: 700;
  border: 1px solid var(--ui-border-strong);
  padding: 0 2px;
  color: var(--ui-text);
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
  background: var(--ui-surface);
  border: 1px solid var(--ui-border);
  min-width: 180px;
  font-family: var(--ui-font);
  font-size: 10px;
  color: var(--ui-text);
  box-shadow: 4px 4px 0 var(--ui-border);
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
  background: var(--ui-glass-2);
  color: var(--ui-text);
}

.context-menu-item.danger:hover {
  background: var(--ui-glass-2);
  color: var(--ui-text);
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
  background: var(--ui-surface);
  border: 1px solid var(--ui-border);
  min-width: 220px;
  z-index: 1001;
  box-shadow: 4px 4px 0 var(--ui-border);
}

.submenu-right {
  left: auto;
  right: 100%;
}

.context-menu-divider {
  height: 1px;
  background: color-mix(in srgb, var(--ui-text) 20%, transparent);
  margin: 2px 0;
}

.rename-overlay {
  position: fixed;
  inset: 0;
  background: color-mix(in srgb, var(--ui-bg-deep) 62%, transparent);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.rename-modal {background: var(--ui-surface);
  border: 1px solid var(--ui-border);
  padding: 16px;
  width: 300px;
  font-family: var(--ui-font);
  color: var(--ui-text);
  box-shadow: 4px 4px 0 var(--ui-border);
  border-radius: var(--ui-radius-lg);
  backdrop-filter: blur(var(--ui-blur)) saturate(var(--ui-saturate));
  -webkit-backdrop-filter: blur(var(--ui-blur)) saturate(var(--ui-saturate));
}

.rename-header {
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 1px;
  margin-bottom: 10px;
}

.rename-input {
  width: 100%;
  background: var(--ui-surface);
  border: 1px solid var(--ui-border);
  color: var(--ui-text);
  font-family: var(--ui-font);
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
  border: 1px solid var(--ui-border);
  color: var(--ui-text);
  padding: 4px 12px;
  font-family: var(--ui-font);
  font-size: 10px;
  font-weight: 700;
  cursor: pointer;
}

.rename-btn:hover {
  background: var(--ui-glass-2);
  color: var(--ui-text);
}

.rename-btn-primary {
  background: var(--ui-glass-2);
  color: var(--ui-text);
}

.rename-btn-primary:hover {
  background: var(--ui-surface);
  color: var(--ui-text);
}

.text-muted { color: color-mix(in srgb, var(--ui-text) 50%, transparent) !important; }
.truncate { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
