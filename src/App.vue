<script setup lang="ts">
import { ref, provide, onMounted, watch, nextTick } from 'vue'
import { useAppStore } from '@/stores/app'
import { useKeyboardShortcuts, getGlobalShortcuts } from '@/composables/useKeyboardShortcuts'
import { useShortcuts } from '@/composables/useShortcuts'
import { useContextMenu } from '@/composables/useContextMenu'
import { useDrag } from '@/composables/useDrag'
import { useSwipe } from '@/composables/useSwipe'
import { useTouchConfig, type TouchAction } from '@/composables/useTouchConfig'
import { useWindowManager } from '@/composables/useWindowManager'
import { defaultKpl, defaultKpd } from '@/keymaps'
import { ShortcutsKey } from '@/composables/shortcutsKey'
import DesktopShell from '@/components/DesktopShell.vue'
import LandingPage from '@/components/LandingPage.vue'
import CanvasEngine from '@/components/CanvasEngine.vue'
import NotificationStack from '@/components/NotificationStack.vue'
import CommandPalette from '@/components/CommandPalette.vue'
import KeyboardShortcutsHelp from '@/components/KeyboardShortcutsHelp.vue'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import LoadingSpinner from '@/components/LoadingSpinner.vue'
import LoginPopup from '@/components/LoginPopup.vue'
import FileUploadDialog from '@/components/FileUploadDialog.vue'
import MobileNav from '@/components/MobileNav.vue'
import ContextMenu from '@/components/ContextMenu.vue'
import type { PanelType } from '@/types'

const store = useAppStore()
const wm = useWindowManager()

useKeyboardShortcuts(getGlobalShortcuts(store))

const shortcutOverrides = ref<Record<string, string>>({})
try {
  const saved = localStorage.getItem('cybermanju_keybindings')
  if (saved) shortcutOverrides.value = JSON.parse(saved)
} catch {}

const shortcuts = useShortcuts(defaultKpl, defaultKpd, undefined, shortcutOverrides)
provide(ShortcutsKey, shortcuts)

const ctx = useContextMenu()
const drag = useDrag()

const touchConfig = useTouchConfig({ autoDetect: true })
touchConfig.onAction((action: TouchAction) => {
  const actionMap: Record<string, () => void> = {
    toggle_sidebar: () => {},
    toggle_palette: () => { store.commandPaletteOpen = !store.commandPaletteOpen },
    toggle_help: () => { store.showShortcutsHelp = !store.showShortcutsHelp },
    focus_search: () => { store.currentPanel = 'search' },
    go_back: () => navigateInHistory(-1),
    go_forward: () => navigateInHistory(1),
    go_home: () => { store.currentPanel = 'landing' },
    prev_panel: () => {},
    next_panel: () => {},
    open_trash: () => { wm.open('trash'); store.fetchTrashItems() },
    open_activity: () => { wm.open('activity'); store.fetchAuditLog() },
    open_collections: () => { wm.open('collections') },
    open_faces: () => { wm.open('faces') },
    open_map: () => { wm.open('map') },
    open_code: () => { wm.open('code') },
    open_settings: () => { wm.open('settings') },
    open_storage: () => { wm.open('storage') },
    escape: () => {
      store.selectedFileId = null
      store.createFolderPromptOpen = false
    },
    new_folder: () => { store.createFolderPromptOpen = true },
    refresh: () => { store.fetchFiles() },
    toggle_fullscreen: () => {
      if (!document.fullscreenElement) document.documentElement.requestFullscreen()
      else document.exitFullscreen()
    },
    context_menu: () => {},
    select_item: () => {},
    zoom_in: () => {},
    zoom_out: () => {},
    none: () => {},
    scroll_up: () => window.scrollBy(0, -100),
    scroll_down: () => window.scrollBy(0, 100),
  }
  actionMap[action]?.()
})

const searchTypeFilter = ref('all')
const searchCurrentDir = ref(false)
const recentSearches = ref<string[]>((() => {
  try { return JSON.parse(localStorage.getItem('cybermanju_recent_searches') || '[]') as string[] } catch { return [] }
})())

function saveRecentSearch(query: string) {
  if (!query.trim()) return
  const arr = recentSearches.value.filter(s => s !== query)
  arr.unshift(query)
  if (arr.length > 10) arr.pop()
  recentSearches.value = arr
  localStorage.setItem('cybermanju_recent_searches', JSON.stringify(arr))
}

watch(() => store.searchResults, (results) => {
  if (results.length > 0 && store.searchQuery.trim()) {
    saveRecentSearch(store.searchQuery)
  }
})

const confirmVisible = ref(false)
const confirmMessage = ref('')
const confirmTitle = ref('CONFIRM')

const newFolderName = ref('')
const folderInputRef = ref<HTMLInputElement | null>(null)
const showUploadDialog = ref(false)

const mainAreaRef = ref<HTMLElement | null>(null)

useSwipe(mainAreaRef, touchConfig.getSwipeOptions())

watch(shortcutOverrides, (v) => {
  localStorage.setItem('cybermanju_keybindings', JSON.stringify(v))
}, { deep: true })

shortcuts.on('toggle_sidebar', () => {})
shortcuts.on('toggle_palette', () => { store.commandPaletteOpen = !store.commandPaletteOpen })
shortcuts.on('toggle_help', () => { store.showShortcutsHelp = !store.showShortcutsHelp })
shortcuts.on('focus_search', () => { store.currentPanel = 'search' })
shortcuts.on('new_folder', () => { store.createFolderPromptOpen = true })
shortcuts.on('upload_file', () => { showUploadDialog.value = true })
shortcuts.on('refresh', () => { store.fetchFiles() })
shortcuts.on('escape', () => {
  store.selectedFileId = null
  store.createFolderPromptOpen = false
})
shortcuts.on('go_back', () => { navigateInHistory(-1) })
shortcuts.on('go_forward', () => { navigateInHistory(1) })
shortcuts.on('open_trash', () => { wm.open('trash'); store.fetchTrashItems() })
shortcuts.on('open_activity', () => { wm.open('activity'); store.fetchAuditLog() })
shortcuts.on('open_collections', () => { wm.open('collections') })
shortcuts.on('open_faces', () => { wm.open('faces') })
shortcuts.on('open_map', () => { wm.open('map') })
shortcuts.on('open_code', () => { wm.open('code') })
shortcuts.on('open_settings', () => { wm.open('settings') })
shortcuts.on('open_storage', () => { wm.open('storage') })
shortcuts.on('go_home', () => { store.currentPanel = 'landing' })

function fileTypeContextMenu(file: any) {
  const base = [
    { id: 'open', label: 'OPEN', icon: '[>]', shortcut: shortcuts.getShortcut('open'), action: () => file?.select?.() },
    { id: 'preview', label: 'PREVIEW', icon: '[=]', shortcut: shortcuts.getShortcut('preview'), action: () => file?.preview?.() },
    { id: 'div0', label: '', divider: true },
  ]
  const typeActions: Record<string, any[]> = {
    image: [
      { id: 'rotate_cw', label: 'ROTATE CW', icon: '[R]', shortcut: shortcuts.getShortcut('rotate_cw'), action: () => file?.rotate?.('cw') },
      { id: 'rotate_ccw', label: 'ROTATE CCW', icon: '[L]', shortcut: shortcuts.getShortcut('rotate_ccw'), action: () => file?.rotate?.('ccw') },
      { id: 'div_i1', label: '', divider: true },
    ],
    audio: [
      { id: 'play', label: 'PLAY', icon: '[P]', action: () => file?.play?.() },
      { id: 'div_a1', label: '', divider: true },
    ],
    video: [
      { id: 'play', label: 'PLAY', icon: '[P]', action: () => file?.play?.() },
      { id: 'div_v1', label: '', divider: true },
    ],
    archive: [
      { id: 'extract', label: 'EXTRACT HERE', icon: '[X]', action: () => file?.extract?.() },
      { id: 'div_ar1', label: '', divider: true },
    ],
    folder: [
      { id: 'open_in_new', label: 'OPEN IN NEW TAB', icon: '[T]', action: () => file?.openNew?.() },
      { id: 'div_f1', label: '', divider: true },
      { id: 'paste_into', label: 'PASTE INTO', icon: '[P]', action: () => file?.pasteInto?.() },
      { id: 'div_f2', label: '', divider: true },
    ],
  }
  const ft = file?.fileType || 'file'
  const typeSpecific = typeActions[ft] || []
  const download = { id: 'download', label: 'DOWNLOAD', icon: '[v]', shortcut: shortcuts.getShortcut('download'), action: () => file?.download?.() }
  const star = { id: 'star', label: file?.isStarred ? 'UNSTAR' : 'STAR', icon: '[*]', shortcut: shortcuts.getShortcut('star_file'), action: () => file?.star?.() }
  const rename = { id: 'rename', label: 'RENAME', icon: '[R]', shortcut: shortcuts.getShortcut('rename'), action: () => file?.rename?.() }
  const duplicate = { id: 'duplicate', label: 'DUPLICATE', icon: '[D]', shortcut: shortcuts.getShortcut('duplicate'), action: () => file?.duplicate?.() }
  const compress = { id: 'compress', label: 'COMPRESS', icon: '[Z]', shortcut: shortcuts.getShortcut('compress'), action: () => file?.compress?.() }
  const encrypt = { id: 'encrypt', label: 'ENCRYPT', icon: '[#]', shortcut: shortcuts.getShortcut('encrypt'), action: () => file?.encrypt?.() }
  const decrypt = { id: 'decrypt', label: 'DECRYPT', icon: '[@]', action: () => file?.decrypt?.() }
  const decompress = { id: 'decompress', label: 'DECOMPRESS', icon: '[$]', action: () => file?.decompress?.() }
  const permissions = { id: 'permissions', label: 'PERMISSIONS', icon: '[!]', shortcut: shortcuts.getShortcut('show_permissions'), action: () => file?.permissions?.() }
  const properties = { id: 'properties', label: 'PROPERTIES', icon: '[i]', shortcut: shortcuts.getShortcut('file_properties'), action: () => file?.properties?.() }
  const deleteAction = { id: 'delete', label: 'DELETE', icon: '[X]', shortcut: shortcuts.getShortcut('delete'), action: () => file?.delete?.() }
  const transformDivider = { id: 'div_t1', label: '', divider: true }
  const metaDivider = { id: 'div_m1', label: '', divider: true }
  const dangerDivider = { id: 'div_d1', label: '', divider: true }
  return [
    ...base,
    ...typeSpecific,
    download,
    star,
    rename,
    duplicate,
    metaDivider,
    file?.encrypted ? decrypt : null,
    file?.compressionLayers?.length ? decompress : null,
    !file?.encrypted ? compress : null,
    !file?.encrypted ? encrypt : null,
    transformDivider,
    permissions,
    properties,
    dangerDivider,
    deleteAction,
  ].filter(Boolean) as any[]
}

ctx.registerContext('file_grid_item', [
  { id: 'open', label: 'OPEN', icon: '[>]', shortcut: shortcuts.getShortcut('open'), action: (d) => d?.select?.() },
  { id: 'preview', label: 'PREVIEW', icon: '[=]', shortcut: shortcuts.getShortcut('preview'), action: (d) => d?.preview?.() },
  { id: 'div0', label: '', divider: true },
  { id: 'download', label: 'DOWNLOAD', icon: '[v]', shortcut: shortcuts.getShortcut('download'), action: (d) => d?.download?.() },
  { id: 'star', label: 'STAR', icon: '[*]', shortcut: shortcuts.getShortcut('star_file'), action: (d) => d?.star?.() },
  { id: 'rename', label: 'RENAME', icon: '[R]', shortcut: shortcuts.getShortcut('rename'), action: (d) => d?.rename?.() },
  { id: 'duplicate', label: 'DUPLICATE', icon: '[D]', shortcut: shortcuts.getShortcut('duplicate'), action: (d) => d?.duplicate?.() },
  { id: 'div1', label: '', divider: true },
  { id: 'compress', label: 'COMPRESS', icon: '[Z]', shortcut: shortcuts.getShortcut('compress'), action: (d) => d?.compress?.() },
  { id: 'encrypt', label: 'ENCRYPT', icon: '[#]', shortcut: shortcuts.getShortcut('encrypt'), action: (d) => d?.encrypt?.() },
  { id: 'decrypt', label: 'DECRYPT', icon: '[@]', action: (d) => d?.decrypt?.() },
  { id: 'div2', label: '', divider: true },
  { id: 'permissions', label: 'PERMISSIONS', icon: '[!]', shortcut: shortcuts.getShortcut('show_permissions'), action: (d) => d?.permissions?.() },
  { id: 'properties', label: 'PROPERTIES', icon: '[i]', shortcut: shortcuts.getShortcut('file_properties'), action: (d) => d?.properties?.() },
  { id: 'div3', label: '', divider: true },
  { id: 'delete', label: 'DELETE', icon: '[X]', shortcut: shortcuts.getShortcut('delete'), action: (d) => d?.delete?.() },
])

ctx.registerContext('file_grid_bg', [
  { id: 'new_folder', label: 'NEW FOLDER', icon: '[+]', shortcut: shortcuts.getShortcut('new_folder'), action: () => { store.createFolderPromptOpen = true } },
  { id: 'paste', label: 'PASTE', icon: '[P]', shortcut: shortcuts.getShortcut('paste'), action: () => {} },
  { id: 'div1', label: '', divider: true },
  {
    id: 'sort', label: 'SORT BY', icon: '[S]', submenu: [
      { id: 'sort_name', label: 'NAME', icon: '[N]', action: () => { (store as any).sortBy = 'name' } },
      { id: 'sort_date', label: 'DATE', icon: '[D]', action: () => { (store as any).sortBy = 'date' } },
      { id: 'sort_size', label: 'SIZE', icon: '[S]', action: () => { (store as any).sortBy = 'size' } },
      { id: 'sort_type', label: 'TYPE', icon: '[T]', action: () => { (store as any).sortBy = 'type' } },
    ]
  },
  {
    id: 'view', label: 'VIEW MODE', icon: '[V]', submenu: [
      { id: 'view_grid', label: 'GRID', icon: '[#]', action: () => { store.currentPanel = 'files'; store.viewMode = 'grid' } },
      { id: 'view_list', label: 'LIST', icon: '[@]', action: () => { store.currentPanel = 'files'; store.viewMode = 'list' } },
      { id: 'view_masonry', label: 'MASONRY', icon: '[*]', action: () => { store.currentPanel = 'files'; store.viewMode = 'masonry' } },
    ]
  },
  { id: 'div2', label: '', divider: true },
  { id: 'select_all', label: 'SELECT ALL', icon: '[A]', shortcut: shortcuts.getShortcut('select_all'), action: () => { store.selectedFileIds = [...store.files.map(f => f.id)] } },
  { id: 'deselect', label: 'DESELECT', icon: '[C]', shortcut: shortcuts.getShortcut('deselect'), action: () => { store.selectedFileIds = [] } },
  { id: 'div3', label: '', divider: true },
  {
    id: 'go_to', label: 'GO TO', icon: '[G]', submenu: [
      { id: 'go_home', label: 'HOME', icon: '[H]', action: () => { store.currentPanel = 'landing' } },
      { id: 'go_trash', label: 'TRASH', icon: '[T]', action: () => { wm.open('trash'); store.fetchTrashItems() } },
      { id: 'go_recent', label: 'RECENT', icon: '[R]', action: () => { wm.open('recent') } },
      { id: 'go_favorites', label: 'FAVORITES', icon: '[*]', action: () => { wm.open('favorites') } },
    ]
  },
  { id: 'div4', label: '', divider: true },
  { id: 'refresh', label: 'REFRESH', icon: '[R]', shortcut: shortcuts.getShortcut('refresh'), action: () => store.fetchFiles() },
])

ctx.registerContext('sidebar_node', [
  { id: 'open', label: 'OPEN', icon: '[>]', action: (d) => d?.select?.() },
  { id: 'open_new_tab', label: 'OPEN IN NEW TAB', icon: '[T]', action: (d) => d?.openNewTab?.() },
  { id: 'div1', label: '', divider: true },
  { id: 'new_subfolder', label: 'NEW SUBFOLDER', icon: '[+]', action: (d) => d?.newSubfolder?.() },
  { id: 'rename', label: 'RENAME', icon: '[R]', action: (d) => d?.rename?.() },
  { id: 'duplicate', label: 'DUPLICATE', icon: '[D]', action: (d) => d?.duplicate?.() },
  { id: 'div2', label: '', divider: true },
  { id: 'paste_into', label: 'PASTE INTO', icon: '[P]', action: (d) => d?.pasteInto?.() },
  { id: 'div3', label: '', divider: true },
  { id: 'expand', label: 'EXPAND ALL', icon: '[+]', action: (d) => d?.expandAll?.() },
  { id: 'collapse', label: 'COLLAPSE ALL', icon: '[-]', action: (d) => d?.collapseAll?.() },
  { id: 'div4', label: '', divider: true },
  { id: 'delete', label: 'DELETE', icon: '[X]', action: (d) => d?.delete?.() },
])

ctx.registerContext('sidebar_bg', [
  { id: 'new_folder', label: 'NEW FOLDER', icon: '[+]', shortcut: shortcuts.getShortcut('new_folder'), action: () => { store.createFolderPromptOpen = true } },
  { id: 'refresh', label: 'REFRESH', icon: '[R]', shortcut: shortcuts.getShortcut('refresh'), action: () => store.fetchFiles() },
  { id: 'div1', label: '', divider: true },
  { id: 'expand', label: 'EXPAND ALL', icon: '[+]', action: () => {} },
  { id: 'collapse', label: 'COLLAPSE ALL', icon: '[-]', action: () => {} },
  { id: 'div2', label: '', divider: true },
  { id: 'show_trash', label: 'SHOW TRASH', icon: '[T]', action: () => { wm.open('trash'); store.fetchTrashItems() } },
  { id: 'show_storage', label: 'STORAGE DASHBOARD', icon: '[@]', action: () => { wm.open('storage') } },
])

ctx.registerContext('search_result', [
  { id: 'open', label: 'OPEN', icon: '[>]', action: (d) => d?.select?.() },
  { id: 'preview', label: 'PREVIEW', icon: '[=]', action: (d) => d?.preview?.() },
  { id: 'download', label: 'DOWNLOAD', icon: '[v]', action: (d) => d?.download?.() },
  { id: 'star', label: 'STAR', icon: '[*]', action: (d) => d?.star?.() },
  { id: 'div1', label: '', divider: true },
  { id: 'copy_path', label: 'COPY PATH', icon: '[C]', action: (d) => d?.copyPath?.() },
  { id: 'show_in_folder', label: 'SHOW IN FOLDER', icon: '[F]', action: (d) => d?.showInFolder?.() },
  { id: 'div2', label: '', divider: true },
  { id: 'properties', label: 'PROPERTIES', icon: '[i]', action: (d) => d?.properties?.() },
])

ctx.registerContext('collection_item', [
  { id: 'open', label: 'OPEN COLLECTION', icon: '[>]', action: (d) => d?.open?.() },
  { id: 'rename', label: 'RENAME', icon: '[R]', action: (d) => d?.rename?.() },
  { id: 'div1', label: '', divider: true },
  { id: 'add_files', label: 'ADD FILES', icon: '[+]', action: (d) => d?.addFiles?.() },
  { id: 'remove_files', label: 'REMOVE FILES', icon: '[-]', action: (d) => d?.removeFiles?.() },
  { id: 'div2', label: '', divider: true },
  { id: 'share', label: 'SHARE', icon: '[@]', action: (d) => d?.share?.() },
  { id: 'delete', label: 'DELETE COLLECTION', icon: '[X]', action: (d) => d?.delete?.() },
])

ctx.registerContext('face_item', [
  { id: 'rename', label: 'RENAME', icon: '[R]', action: (d) => d?.rename?.() },
  { id: 'merge', label: 'MERGE WITH...', icon: '[+]', action: (d) => d?.merge?.() },
  { id: 'div1', label: '', divider: true },
  { id: 'show_files', label: 'SHOW FILES', icon: '[F]', action: (d) => d?.showFiles?.() },
  { id: 'delete', label: 'DELETE GROUP', icon: '[X]', action: (d) => d?.delete?.() },
])

ctx.registerContext('trash_item', [
  { id: 'restore', label: 'RESTORE', icon: '[R]', action: (d) => d?.restore?.() },
  { id: 'div1', label: '', divider: true },
  { id: 'delete_perm', label: 'DELETE PERMANENTLY', icon: '[X]', action: (d) => d?.deletePermanently?.() },
])

ctx.registerContext('activity_item', [
  { id: 'copy_details', label: 'COPY DETAILS', icon: '[C]', action: (d) => d?.copyDetails?.() },
  { id: 'show_file', label: 'SHOW FILE', icon: '[F]', action: (d) => d?.showFile?.() },
])

ctx.registerContext('favorite_item', [
  { id: 'open', label: 'OPEN', icon: '[>]', action: (d) => d?.open?.() },
  { id: 'unstar', label: 'UNSTAR', icon: '[*]', action: (d) => d?.unstar?.() },
  { id: 'div1', label: '', divider: true },
  { id: 'show_in_folder', label: 'SHOW IN FOLDER', icon: '[F]', action: (d) => d?.showInFolder?.() },
])

ctx.registerContext('recent_item', [
  { id: 'open', label: 'OPEN', icon: '[>]', action: (d) => d?.open?.() },
  { id: 'star', label: 'STAR', icon: '[*]', action: (d) => d?.star?.() },
  { id: 'div1', label: '', divider: true },
  { id: 'show_in_folder', label: 'SHOW IN FOLDER', icon: '[F]', action: (d) => d?.showInFolder?.() },
])

const pathHistory = ref<string[]>([])
const pathHistoryIndex = ref(-1)

function navigateToPath(path: string) {
  const fullPath = '/' + path
  if (pathHistoryIndex.value < pathHistory.value.length - 1) {
    pathHistory.value = pathHistory.value.slice(0, pathHistoryIndex.value + 1)
  }
  pathHistory.value.push(fullPath)
  pathHistoryIndex.value = pathHistory.value.length - 1
  store.currentPath = fullPath
  store.fetchFiles(fullPath)
}

function goBack() {
  if (pathHistoryIndex.value > 0) {
    pathHistoryIndex.value--
    const path = pathHistory.value[pathHistoryIndex.value]
    store.currentPath = path
    store.fetchFiles(path)
  }
}

function goForward() {
  if (pathHistoryIndex.value < pathHistory.value.length - 1) {
    pathHistoryIndex.value++
    const path = pathHistory.value[pathHistoryIndex.value]
    store.currentPath = path
    store.fetchFiles(path)
  }
}

function navigateInHistory(dir: number) {
  if (dir < 0) goBack()
  else goForward()
}

watch(() => store.createFolderPromptOpen, async (v: boolean) => {
  if (v) {
    newFolderName.value = ''
    await nextTick()
    folderInputRef.value?.focus()
  }
})

async function handleCreateFolder() {
  if (!newFolderName.value.trim()) return
  await store.createFolder(newFolderName.value.trim(), store.selectedFileId || '')
  newFolderName.value = ''
  store.createFolderPromptOpen = false
}

function handleUpload() {
  showUploadDialog.value = true
}

onMounted(() => {
  store.currentPanel = 'landing'
  store.initialize()
  window.addEventListener('cybermanju:upload', handleUpload)
})
</script>

<template>
  <div class="cybermanju-shell">
    <LandingPage
      v-if="store.currentPanel === 'landing'"
      @open-app="store.currentPanel = 'files'; wm.open('files')"
    />
    <template v-else>
      <DesktopShell>
        <template #wallpaper>
          <CanvasEngine :enabled="store.matrixRainEnabled" />
        </template>
      </DesktopShell>

      <div v-if="store.lastError" class="error-banner" @click="store.clearError()">
      <span class="error-icon">[!]</span>
      <span class="error-text">{{ store.lastError }}</span>
      <span class="error-dismiss">X</span>
    </div>

    <Teleport to="body">
      <div v-if="store.createFolderPromptOpen" class="overlay-thin" @click.self="store.createFolderPromptOpen = false">
        <div class="mini-modal">
          <div class="mini-header">NEW FOLDER</div>
          <input
            ref="folderInputRef"
            v-model="newFolderName"
            class="bw-input"
            style="width:100%;margin-bottom:8px;"
            placeholder="FOLDER NAME"
            @keyup.enter="handleCreateFolder"
          />
          <div class="mini-actions">
            <button class="bw-btn" @click="store.createFolderPromptOpen = false">[CANCEL]</button>
            <button class="bw-btn bw-btn-inverse" @click="handleCreateFolder">[CREATE]</button>
          </div>
        </div>
      </div>
    </Teleport>

    <NotificationStack />
    <CommandPalette />
    <KeyboardShortcutsHelp />
    <ConfirmDialog
      :visible="confirmVisible"
      :title="confirmTitle"
      :message="confirmMessage"
      @confirm="confirmVisible = false"
      @cancel="confirmVisible = false"
      @update:visible="confirmVisible = $event"
    />
    <LoginPopup />
    <FileUploadDialog
      :visible="showUploadDialog"
      @close="showUploadDialog = false"
    />
    <MobileNav />
    <ContextMenu />

    </template>
    </div>
</template>

<style scoped>
.cybermanju-shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
  width: 100vw;
  background: #0a0a0a;
  color: #e0e0e0;
  overflow: hidden;
  position: relative;
}

.error-banner {
  position: fixed;
  bottom: 60px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 16px;
  background: #1a1a1a;
  border: 1px solid #ff5f57;
  border-radius: 8px;
  color: #ff5f57;
  font-family: 'Courier New', monospace;
  font-size: 10px;
  font-weight: 600;
  cursor: pointer;
  z-index: 9999;
  box-shadow: 0 4px 16px rgba(0,0,0,0.5);
}

.error-icon { font-size: 12px; flex-shrink: 0; }
.error-text { flex: 1; }
.error-dismiss { font-weight: 700; cursor: pointer; margin-left: 8px; }

.overlay-thin {
  position: fixed;
  inset: 0;
  background: rgba(0,0,0,0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10000;
}

.mini-modal {
  background: #1a1a1a;
  border: 1px solid #2a2a2a;
  border-radius: 10px;
  box-shadow: 0 8px 32px rgba(0,0,0,0.6);
  padding: 20px;
  width: 300px;
  font-family: 'Courier New', monospace;
}

.mini-header {
  font-size: 12px;
  font-weight: 800;
  color: #e0e0e0;
  margin-bottom: 12px;
  letter-spacing: 1px;
}

.mini-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.text-muted { color: rgba(255,255,255,0.5) !important; }
.truncate { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
