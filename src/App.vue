<script setup lang="ts">
import { ref, provide, onMounted, watch, computed, nextTick } from 'vue'
import { useAppStore } from '@/stores/app'
import { isWebMode } from '@/composables/useTauri'
import { useKeyboardShortcuts, getGlobalShortcuts } from '@/composables/useKeyboardShortcuts'
import { useShortcuts } from '@/composables/useShortcuts'
import { useContextMenu } from '@/composables/useContextMenu'
import { useDrag } from '@/composables/useDrag'
import { useSwipe } from '@/composables/useSwipe'
import { useTouchConfig, type TouchAction } from '@/composables/useTouchConfig'
import { defaultKpl, defaultKpd } from '@/keymaps'
import { ShortcutsKey } from '@/composables/shortcutsKey'
import CanvasEngine from '@/components/CanvasEngine.vue'
import TopBar from '@/components/TopBar.vue'
import Sidebar from '@/components/Sidebar.vue'
import FileGrid from '@/components/FileGrid.vue'
import FilePreview from '@/components/FilePreview.vue'
import StatusBar from '@/components/StatusBar.vue'
import EncryptionPanel from '@/components/EncryptionPanel.vue'
import CompressionPanel from '@/components/CompressionPanel.vue'
import CollectionsPanel from '@/components/CollectionsPanel.vue'
import FaceGroupingPanel from '@/components/FaceGroupingPanel.vue'
import MapView from '@/components/MapView.vue'
import CodeIntelligencePanel from '@/components/CodeIntelligencePanel.vue'
import UserManagementPanel from '@/components/UserManagementPanel.vue'
import WebDashboardPanel from '@/components/WebDashboardPanel.vue'
import DashboardOverlay from '@/components/DashboardOverlay.vue'
import SyncPanel from '@/components/SyncPanel.vue'
import LandingPage from '@/components/LandingPage.vue'
import NotificationStack from '@/components/NotificationStack.vue'
import CommandPalette from '@/components/CommandPalette.vue'
import KeyboardShortcutsHelp from '@/components/KeyboardShortcutsHelp.vue'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import LoadingSpinner from '@/components/LoadingSpinner.vue'
import SettingsPage from '@/components/SettingsPage.vue'
import LoginPopup from '@/components/LoginPopup.vue'
import FileUploadDialog from '@/components/FileUploadDialog.vue'
import StorageDashboard from '@/components/StorageDashboard.vue'
import FilePermissionsPanel from '@/components/FilePermissionsPanel.vue'
import MobileNav from '@/components/MobileNav.vue'
import ContextMenu from '@/components/ContextMenu.vue'
import type { PanelType } from '@/types'

const store = useAppStore()

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
    toggle_sidebar: () => { store.sidebarCollapsed = !store.sidebarCollapsed },
    toggle_palette: () => { store.commandPaletteOpen = !store.commandPaletteOpen },
    toggle_help: () => { store.showShortcutsHelp = !store.showShortcutsHelp },
    focus_search: () => { store.currentPanel = 'search' },
    go_back: () => navigateInHistory(-1),
    go_forward: () => navigateInHistory(1),
    go_home: () => { store.currentPanel = 'landing' },
    prev_panel: () => {},
    next_panel: () => {},
    open_trash: () => { store.currentPanel = 'trash'; store.fetchTrashItems() },
    open_activity: () => { store.currentPanel = 'activity'; store.fetchAuditLog() },
    open_collections: () => { store.currentPanel = 'collections' },
    open_faces: () => { store.currentPanel = 'faces' },
    open_map: () => { store.currentPanel = 'map' },
    open_code: () => { store.currentPanel = 'code' },
    open_settings: () => { store.currentPanel = 'settings' },
    open_storage: () => { store.currentPanel = 'storage' },
    escape: () => {
      store.selectedFile = null
      store.createFolderPromptOpen = false
      store.showEncryptionPanel = false
      store.showCompressionPanel = false
      store.showPermissionsPanel = false
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

const recentFiles = computed(() =>
  [...store.files]
    .sort((a, b) => new Date(b.modifiedAt).getTime() - new Date(a.modifiedAt).getTime())
    .slice(0, 20)
)

const filteredSearchResults = computed(() => {
  const results = store.searchResults
  if (searchTypeFilter.value === 'all') return results
  return results.filter(r => {
    if (searchTypeFilter.value === 'folder') return r.matchType === 'folder'
    if (searchTypeFilter.value === 'image') return r.matchType === 'image'
    if (searchTypeFilter.value === 'text') return r.matchType === 'text'
    return true
  })
})

const showRightPanel = computed(() => {
  return store.selectedFileId !== null
    || store.showEncryptionPanel
    || store.showCompressionPanel
    || store.showPermissionsPanel
})

const confirmVisible = ref(false)
const confirmMessage = ref('')
const confirmTitle = ref('CONFIRM')

const searchTypeFilter = ref('all')
const searchCurrentDir = ref(false)
const recentSearches = ref<string[]>(() => {
  try { return JSON.parse(localStorage.getItem('cybermanju_recent_searches') || '[]') as string[] } catch { return [] }
})

function saveRecentSearch(query: string) {
  if (!query.trim()) return
  const arr = recentSearches.value.filter(s => s !== query)
  arr.unshift(query)
  if (arr.length > 10) arr.pop()
  recentSearches.value = arr
  localStorage.setItem('cybermanju_recent_searches', JSON.stringify(arr))
}

function escapeHtml(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
}

function highlightTerms(text: string, query: string): string {
  if (!query.trim()) return escapeHtml(text)
  const escaped = escapeHtml(query).replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  return escapeHtml(text).replace(new RegExp(`(${escaped})`, 'gi'), '<mark>$1</mark>')
}

watch(() => store.searchResults, (results) => {
  if (results.length > 0 && store.searchQuery.trim()) {
    saveRecentSearch(store.searchQuery)
  }
})

const newFolderName = ref('')
const folderInputRef = ref<HTMLInputElement | null>(null)
const showUploadDialog = ref(false)

const mainAreaRef = ref<HTMLElement | null>(null)

useSwipe(mainAreaRef, touchConfig.getSwipeOptions())

watch(shortcutOverrides, (v) => {
  localStorage.setItem('cybermanju_keybindings', JSON.stringify(v))
}, { deep: true })

shortcuts.on('toggle_sidebar', () => { store.sidebarCollapsed = !store.sidebarCollapsed })
shortcuts.on('toggle_palette', () => { store.commandPaletteOpen = !store.commandPaletteOpen })
shortcuts.on('toggle_help', () => { store.showShortcutsHelp = !store.showShortcutsHelp })
shortcuts.on('focus_search', () => { store.currentPanel = 'search' })
shortcuts.on('new_folder', () => { store.createFolderPromptOpen = true })
shortcuts.on('upload_file', () => { showUploadDialog.value = true })
shortcuts.on('refresh', () => { store.fetchFiles() })
shortcuts.on('escape', () => {
  store.selectedFile = null
  store.createFolderPromptOpen = false
  store.showEncryptionPanel = false
  store.showCompressionPanel = false
  store.showPermissionsPanel = false
})
shortcuts.on('go_back', () => { navigateInHistory(-1) })
shortcuts.on('go_forward', () => { navigateInHistory(1) })
shortcuts.on('open_trash', () => { store.currentPanel = 'trash'; store.fetchTrashItems() })
shortcuts.on('open_activity', () => { store.currentPanel = 'activity'; store.fetchAuditLog() })
shortcuts.on('open_collections', () => { store.currentPanel = 'collections' })
shortcuts.on('open_faces', () => { store.currentPanel = 'faces' })
shortcuts.on('open_map', () => { store.currentPanel = 'map' })
shortcuts.on('open_code', () => { store.currentPanel = 'code' })
shortcuts.on('open_settings', () => { store.currentPanel = 'settings' })
shortcuts.on('open_storage', () => { store.currentPanel = 'storage' })
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
      { id: 'sort_name', label: 'NAME', icon: '[N]', action: () => { store.sortBy = 'name' } },
      { id: 'sort_date', label: 'DATE', icon: '[D]', action: () => { store.sortBy = 'date' } },
      { id: 'sort_size', label: 'SIZE', icon: '[S]', action: () => { store.sortBy = 'size' } },
      { id: 'sort_type', label: 'TYPE', icon: '[T]', action: () => { store.sortBy = 'type' } },
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
      { id: 'go_trash', label: 'TRASH', icon: '[T]', action: () => { store.currentPanel = 'trash'; store.fetchTrashItems() } },
      { id: 'go_recent', label: 'RECENT', icon: '[R]', action: () => { store.currentPanel = 'recent' } },
      { id: 'go_favorites', label: 'FAVORITES', icon: '[*]', action: () => { store.currentPanel = 'favorites' } },
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
  { id: 'show_trash', label: 'SHOW TRASH', icon: '[T]', action: () => { store.currentPanel = 'trash'; store.fetchTrashItems() } },
  { id: 'show_storage', label: 'STORAGE DASHBOARD', icon: '[@]', action: () => { store.currentPanel = 'storage' } },
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



onMounted(() => {
  store.initialize()
  if (isWebMode() && store.currentPanel === 'files') {
    store.currentPanel = 'landing'
  }
})
</script>

<template>
  <div class="cybermanju-shell">
    <CanvasEngine :enabled="store.matrixRainEnabled" />

    <TopBar />

    <div class="main-area" ref="mainAreaRef">
      <Sidebar />

      <main class="center-content">
        <div v-if="store.lastError" class="error-banner" @click="store.clearError()">
          <span class="error-icon">[!]</span>
          <span class="error-text">{{ store.lastError }}</span>
          <span class="error-dismiss">X</span>
        </div>

        <div class="view-toolbar">
          <div class="toolbar-left">
            <button class="tb-nav-btn" @click="store.currentPanel = 'files'" title="GO TO FILES">[#]</button>
            <button class="tb-nav-btn" @click="goBack" :disabled="pathHistoryIndex <= 0" title="BACK">[&lt;]</button>
            <button class="tb-nav-btn" @click="goForward" :disabled="pathHistoryIndex >= pathHistory.length - 1" title="FORWARD">[&gt;]</button>
            <button class="tb-nav-btn" @click="store.fetchFiles()" title="REFRESH">[R]</button>
            <span class="breadcrumb">
              <span class="bc-root" @click="store.currentPath = '/'; store.fetchFiles('/')">/~</span>
              <template v-for="(part, idx) in store.currentPath.split('/').filter(Boolean)" :key="idx">
                <span class="bc-sep">/</span>
                <span
                  class="bc-part"
                  :class="{ 'bc-active': idx === store.currentPath.split('/').filter(Boolean).length - 1 }"
                  @click="navigateToPath(store.currentPath.split('/').filter(Boolean).slice(0, Number(idx) + 1).join('/'))"
                >{{ part }}</span>
              </template>
            </span>
          </div>
          <div class="toolbar-right">
            <button
              class="view-btn"
              :class="{ active: store.viewMode === 'grid' }"
              @click="store.viewMode = 'grid'"
              title="GRID VIEW (CTRL+G)"
            >
              <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><rect x="1" y="1" width="6" height="6"/><rect x="9" y="1" width="6" height="6"/><rect x="1" y="9" width="6" height="6"/><rect x="9" y="9" width="6" height="6"/></svg>
            </button>
            <button
              class="view-btn"
              :class="{ active: store.viewMode === 'list' }"
              @click="store.viewMode = 'list'"
              title="LIST VIEW (CTRL+L)"
            >
              <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><rect x="1" y="2" width="14" height="2.5"/><rect x="1" y="6.75" width="14" height="2.5"/><rect x="1" y="11.5" width="14" height="2.5"/></svg>
            </button>
            <button
              class="view-btn"
              :class="{ active: store.viewMode === 'masonry' }"
              @click="store.viewMode = 'masonry'"
              title="MASONRY VIEW (CTRL+M)"
            >
              <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><rect x="1" y="1" width="6" height="4"/><rect x="9" y="1" width="6" height="8"/><rect x="1" y="7" width="6" height="8"/><rect x="9" y="11" width="6" height="4"/></svg>
            </button>
            <div class="tb-divider" />
            <button class="tb-nav-btn" @click="store.createFolderPromptOpen = true" title="NEW FOLDER (CTRL+N)">[+]</button>
            <button class="tb-nav-btn" @click="showUploadDialog = true" title="UPLOAD FILES">[^]</button>
            <div class="tb-divider" />
            <span class="file-count">
              {{ store.currentFolderFiles.length }} ITEMS
              <span v-if="store.encryptedFiles.length" class="count-badge">{{ store.encryptedFiles.length }} ENC</span>
              <span v-if="store.compressedFiles.length" class="count-badge">{{ store.compressedFiles.length }} CMP</span>
            </span>
          </div>
        </div>

        <div class="content-panels">
          <Transition name="panel-fade" mode="out-in">
            <LandingPage
              v-if="store.currentPanel === 'landing'"
              key="landing"
              @open-app="store.currentPanel = 'files'"
            />
            <FileGrid
              v-else-if="store.currentPanel === 'files'"
              key="files"
            />
            <DashboardOverlay
              v-else-if="store.currentPanel === 'webdash'"
              key="webdash"
              @close="store.currentPanel = 'files'"
            />
            <div v-else-if="store.currentPanel === 'search'" key="search" class="panel-search">
              <div class="bw-card" style="padding: 12px; margin-bottom: 12px;">
                <div class="bw-title">TANTIVY SEARCH</div>
                <div class="search-controls">
                  <p class="text-muted" style="flex:1;">"{{ store.searchQuery }}" - {{ filteredSearchResults.length }} RESULTS</p>
                  <select v-model="searchTypeFilter" class="bw-select-sm" title="FILTER BY TYPE">
                    <option value="all">ALL</option>
                    <option value="image">IMAGES</option>
                    <option value="text">TEXT</option>
                    <option value="folder">FOLDERS</option>
                    <option value="file">FILES</option>
                  </select>
                  <label class="search-current-dir" title="SEARCH ONLY IN CURRENT DIRECTORY">
                    <input type="checkbox" v-model="searchCurrentDir" class="bw-checkbox" />
                    <span class="text-muted" style="font-size:9px;">DIR</span>
                  </label>
                </div>
              </div>

              <div v-if="recentSearches.length > 0 && !store.searchQuery" class="recent-searches">
                <div class="bw-title" style="padding: 0 12px; margin-bottom: 6px;">RECENT SEARCHES</div>
                <div v-for="sq in recentSearches" :key="sq" class="recent-search-item" @click="store.searchQuery = sq; store.searchFiles(sq)">
                  <span class="text-muted">[R]</span>
                  <span>{{ sq }}</span>
                </div>
              </div>

              <div v-if="filteredSearchResults.length" class="search-results-list">
                <div
                  v-for="result in filteredSearchResults"
                  :key="result.fileId"
                  class="search-result-item bw-card"
                  @click="store.selectFile(result.fileId); store.currentPanel = 'files'"
                >
                  <div class="search-match-type">{{ result.matchType?.toUpperCase() || 'SEARCH' }}</div>
                  <div class="search-result-body">
                    <div class="search-result-name" v-html="highlightTerms(result.fileName, store.searchQuery)"></div>
                    <div class="search-result-snippet text-muted" v-if="result.snippet" v-html="highlightTerms(result.snippet, store.searchQuery)"></div>
                  </div>
                  <div class="search-result-score">{{ result.score.toFixed(3) }}</div>
                </div>
                <button
                  v-if="filteredSearchResults.length < store.searchTotalResults"
                  class="load-more-btn"
                  @click="store.loadMoreSearchResults()"
                  :disabled="store.isSearching"
                >[LOAD MORE] ({{ store.searchTotalResults - filteredSearchResults.length }} MORE)</button>
              </div>
              <div v-else-if="store.searchQuery && !store.isSearching" class="empty-state">
                <p class="text-muted">NO RESULTS FOR "{{ store.searchQuery }}"</p>
              </div>
              <div v-else-if="!store.searchQuery" class="empty-state">
                <p class="text-muted">TYPE IN SEARCH BAR FOR TANTIVY BM25 SEARCH</p>
              </div>
            </div>
            <CollectionsPanel v-else-if="store.currentPanel === 'collections'" key="collections" />
            <FaceGroupingPanel v-else-if="store.currentPanel === 'faces'" key="faces" />
            <MapView v-else-if="store.currentPanel === 'map'" key="map" />
            <CodeIntelligencePanel v-else-if="store.currentPanel === 'code'" key="code" />
            <UserManagementPanel v-else-if="store.currentPanel === 'users'" key="users" />
            <WebDashboardPanel v-else-if="store.currentPanel === 'dashboard'" key="dashboard" />
            <SyncPanel v-else-if="store.currentPanel === 'sync'" key="sync" />
            <SettingsPage v-else-if="store.currentPanel === 'settings'" key="settings" />
            <div v-else-if="store.currentPanel === 'trash'" key="trash" class="panel-page">
              <div class="bw-card" style="padding: 16px;">
                <div class="trash-header">
                  <div class="bw-title">TRASH</div>
                  <div class="trash-actions">
                    <button class="bw-btn bw-btn-sm" @click="store.fetchTrashItems()" title="REFRESH TRASH">[R]</button>
                    <button class="bw-btn bw-btn-sm bw-btn-danger" @click="store.emptyTrash()" title="EMPTY TRASH">[EMPTY]</button>
                  </div>
                </div>
                <p class="text-muted" style="margin-bottom:8px;font-size:10px;">DELETED FILES CAN BE RESTORED FROM HERE.</p>
                <div v-if="store.trashItems.length === 0" class="empty-state" style="height:80px;">
                  <p class="text-muted">NO FILES IN TRASH</p>
                </div>
                <div v-else class="trash-list">
                  <div v-for="item in store.trashItems" :key="item.id" class="trash-item">
                    <span class="trash-icon">{{ item.originalFile.fileType === 'folder' ? '[+]' : '[=]' }}</span>
                    <div class="trash-info">
                      <span class="trash-name truncate">{{ item.originalFile.name }}</span>
                      <span class="trash-date text-muted">{{ new Date(item.deletedAt).toLocaleDateString() }}</span>
                    </div>
                    <div class="trash-actions">
                      <button class="trash-action-btn" @click="store.restoreTrashItem(item.originalFile.id)" title="RESTORE">[RST]</button>
                      <button class="trash-action-btn danger" @click="store.deleteFromTrash(item.originalFile.id)" title="DELETE PERMANENTLY">[DEL]</button>
                    </div>
                  </div>
                </div>
              </div>
            </div>
            <div v-else-if="store.currentPanel === 'activity'" key="activity" class="panel-page">
              <div class="bw-card" style="padding: 16px;">
                <div class="trash-header">
                  <div class="bw-title">ACTIVITY LOG</div>
                  <button class="bw-btn bw-btn-sm" @click="store.fetchAuditLog()" title="REFRESH">[R]</button>
                </div>
                <p class="text-muted" style="margin-bottom:8px;font-size:10px;">FILE OPERATIONS TIMELINE.</p>
                <div v-if="store.auditLog.length === 0" class="empty-state" style="height:80px;">
                  <p class="text-muted">NO RECENT ACTIVITY</p>
                </div>
                <div v-else class="activity-list">
                  <div v-for="entry in store.auditLog" :key="entry.id" class="activity-item">
                    <span class="activity-action">{{ entry.action.toUpperCase() }}</span>
                    <span class="activity-entity text-muted">{{ entry.entityType }}</span>
                    <span class="activity-date text-muted">{{ new Date(entry.timestamp).toLocaleString() }}</span>
                    <span v-if="entry.details && Object.keys(entry.details).length" class="activity-detail text-muted">{{ JSON.stringify(entry.details).substring(0, 40) }}</span>
                  </div>
                </div>
              </div>
            </div>
            <div v-else-if="store.currentPanel === 'favorites'" key="favorites" class="panel-page">
              <div class="bw-card" style="padding: 16px;">
                <div class="bw-title">FAVORITES</div>
                <div v-if="store.starredFiles.length === 0" class="empty-state" style="height:100px;">
                  <p class="text-muted">NO STARRED FILES</p>
                </div>
                <div v-else class="fav-list">
                  <div v-for="f in store.starredFiles" :key="f.id" class="fav-item" @click="store.selectFile(f.id)">
                    <span class="fav-icon">{{ f.fileType === 'folder' ? '[+]' : '[=]' }}</span>
                    <span class="fav-name truncate">{{ f.name }}</span>
                  </div>
                </div>
              </div>
            </div>
            <div v-else-if="store.currentPanel === 'recent'" key="recent" class="panel-page">
              <div class="bw-card" style="padding: 16px;">
                <div class="bw-title">RECENT FILES</div>
                <div v-if="store.files.length === 0" class="empty-state" style="height:100px;">
                  <p class="text-muted">NO FILES YET</p>
                </div>
                <div v-else class="recent-list">
                  <div
                    v-for="f in recentFiles"
                    :key="f.id"
                    class="recent-item"
                    @click="store.selectFile(f.id)"
                  >
                    <span class="recent-icon">{{ f.fileType === 'folder' ? '[+]' : '[=]' }}</span>
                    <div class="recent-info">
                      <span class="recent-name truncate">{{ f.name }}</span>
                      <span class="recent-date text-muted">{{ new Date(f.modifiedAt).toLocaleDateString() }}</span>
                    </div>
                  </div>
                </div>
              </div>
            </div>
            <div v-else-if="store.currentPanel === 'accounts'" key="accounts" class="panel-page">
              <div class="bw-card" style="padding: 16px;">
                <div class="bw-title">MULTI-ACCOUNT MANAGER</div>
                <div class="accounts-list">
                  <div
                    v-for="account in store.accounts"
                    :key="account.id"
                    class="account-item bw-card"
                    :class="{ active: account.isActive }"
                    @click="store.switchAccount(account.id)"
                  >
                    <div class="bw-dot" :class="{ 'bw-dot-on': account.isActive }" />
                    <div class="account-info">
                      <div class="account-name">{{ account.name }}</div>
                      <div class="account-meta text-muted">{{ account.accountType }} {{ account.path }}</div>
                    </div>
                    <div v-if="account.isActive" class="active-badge">ACTIVE</div>
                  </div>
                </div>
              </div>
            </div>
            <div v-else-if="store.currentPanel === 'loose-groups'" key="loose" class="panel-page">
              <div class="bw-card" style="padding: 16px;">
                <div class="bw-title">LOOSE FILE GROUPING</div>
                <div class="loose-groups-list">
                  <div v-for="group in store.looseGroups" :key="group.id" class="loose-group-item bw-card">
                    <div class="lg-info">
                      <div class="lg-name">{{ group.name }}</div>
                      <div class="lg-count text-muted">{{ group.fileIds.length }} FILES</div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
            <StorageDashboard v-else-if="store.currentPanel === 'storage'" key="storage" />
            <div v-else-if="store.currentPanel === 'style'" key="style" class="panel-page">
              <div class="bw-card" style="padding: 16px;">
                <div class="bw-title">STYLE-BASED ORGANIZATION</div>
                <p class="text-muted" style="margin-bottom: 12px;">FILES ORGANIZED BY VISUAL STYLE (CLIP MODEL)</p>
                <div class="style-tags">
                  <span v-for="tag in [...new Set(store.files.flatMap(f => f.tags || []))]" :key="tag" class="bw-btn" style="padding: 4px 8px;" @click="store.searchQuery = tag; store.currentPanel = 'search'; store.searchFiles(tag)">
                    {{ tag }}
                  </span>
                </div>
              </div>
            </div>
            <div v-else key="loading" class="panel-page">
              <LoadingSpinner size="lg" label="LOADING..." />
            </div>
          </Transition>
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
      </main>

      <Transition name="slide-right">
        <aside v-if="showRightPanel" class="right-panel">
          <EncryptionPanel v-if="store.showEncryptionPanel" @close="store.showEncryptionPanel = false" />
          <CompressionPanel v-else-if="store.showCompressionPanel" @close="store.showCompressionPanel = false" />
          <FilePermissionsPanel v-else-if="store.showPermissionsPanel" @close="store.showPermissionsPanel = false" />
          <FilePreview v-else />
        </aside>
      </Transition>
    </div>

    <StatusBar />

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
  </div>
</template>

<style scoped>
.cybermanju-shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
  width: 100vw;
  background: #000;
  color: #FFFFFF;
  overflow: hidden;
  position: relative;
}

.main-area {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.center-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-width: 0;
}

.error-banner {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  background: #FFFFFF;
  border-bottom: 2px solid #000000;
  color: #000000;
  font-family: 'Courier New', monospace;
  font-size: 10px;
  font-weight: 600;
  cursor: pointer;
  flex-shrink: 0;
}

.error-icon { font-size: 12px; flex-shrink: 0; }
.error-text { flex: 1; }
.error-dismiss { font-weight: 700; cursor: pointer; }

.view-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 12px;
  border-bottom: 2px solid #FFFFFF;
  background: #000;
  min-height: 34px;
  flex-shrink: 0;
}

.toolbar-left, .toolbar-right {
  display: flex;
  align-items: center;
  gap: 6px;
}

.tb-nav-btn {
  background: transparent;
  border: 2px solid transparent;
  color: rgba(255,255,255,0.5);
  padding: 2px 6px;
  cursor: pointer;
  font-family: 'Courier New', monospace;
  font-size: 11px;
  font-weight: 700;
}

.tb-nav-btn:hover {
  color: #FFFFFF;
  border-color: #FFFFFF;
}

.breadcrumb {
  font-family: 'Courier New', monospace;
  font-size: 11px;
  color: rgba(255,255,255,0.6);
}

.bc-root { color: #FFFFFF; cursor: pointer; }
.bc-root:hover { text-decoration: underline; }
.bc-sep { opacity: 0.4; margin: 0 2px; }
.bc-part { color: #FFFFFF; cursor: pointer; }
.bc-part:hover { text-decoration: underline; }

.tb-divider {
  width: 1px;
  height: 14px;
  background: rgba(255,255,255,0.3);
}

.file-count {
  font-family: 'Courier New', monospace;
  font-size: 10px;
  color: rgba(255,255,255,0.6);
}

.count-badge {
  margin-left: 8px;
  color: #FFFFFF;
  border: 1px solid #FFFFFF;
  padding: 0 4px;
  font-size: 9px;
}

.view-btn {
  background: transparent;
  border: 2px solid transparent;
  color: rgba(255,255,255,0.4);
  padding: 3px 6px;
  cursor: pointer;
  display: flex;
  align-items: center;
}

.view-btn:hover {
  color: #FFFFFF;
  border-color: #FFFFFF;
}

.view-btn.active {
  color: #000;
  background: #FFFFFF;
  border-color: #FFFFFF;
}

.content-panels {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
}

.right-panel {
  width: 380px;
  min-width: 380px;
  border-left: 2px solid #FFFFFF;
  background: #000;
  overflow-y: auto;
  overflow-x: hidden;
}

.search-results-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 0 12px 12px;
}

.search-result-item {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 10px !important;
  cursor: pointer;
}

.search-result-item:hover {
  transform: translate(2px, 2px);
  box-shadow: 1px 1px 0 #000 !important;
}

.search-match-type {
  font-size: 9px;
  font-weight: 700;
  padding: 2px 6px;
  border: 2px solid #000;
  background: #FFFFFF;
  color: #000;
  white-space: nowrap;
}

.search-result-body {
  flex: 1;
  min-width: 0;
}

.search-result-name {
  font-family: 'Courier New', monospace;
  font-weight: 700;
  color: #000;
  margin-bottom: 2px;
  font-size: 12px;
}

.search-result-snippet {
  font-size: 10px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: rgba(0,0,0,0.5);
}

.search-result-score {
  font-family: 'Courier New', monospace;
  font-size: 10px;
  color: #000;
  white-space: nowrap;
}

.load-more-btn {
  width: 100%;
  padding: 8px;
  background: #000;
  border: 2px solid #000;
  color: #000;
  font-family: 'Courier New', monospace;
  font-size: 10px;
  font-weight: 700;
  cursor: pointer;
  text-align: center;
}

.load-more-btn:hover {
  background: #000;
  color: #FFF;
  border-color: #FFF;
}

.load-more-btn:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

.search-controls {
  display: flex;
  align-items: center;
  gap: 8px;
}

.bw-select-sm {
  background: #000;
  border: 2px solid #FFFFFF;
  color: #FFFFFF;
  font-family: 'Courier New', monospace;
  font-size: 9px;
  padding: 2px 4px;
  cursor: pointer;
  appearance: none;
}

.search-current-dir {
  display: flex;
  align-items: center;
  gap: 4px;
  cursor: pointer;
}

.bw-checkbox {
  appearance: none;
  width: 12px;
  height: 12px;
  border: 1px solid #FFFFFF;
  background: #000;
  cursor: pointer;
}

.bw-checkbox:checked {
  background: #FFFFFF;
}

.recent-searches {
  padding: 0 12px 12px;
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.recent-search-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 6px;
  font-size: 10px;
  cursor: pointer;
  border: 2px solid transparent;
  font-family: 'Courier New', monospace;
  color: #000;
}

.recent-search-item:hover {
  border-color: #000;
}

.recent-search-item .text-muted {
  color: rgba(0,0,0,0.4) !important;
}

.search-result-name :deep(mark) {
  background: #000;
  color: #FFF;
  padding: 0 2px;
}

.search-result-snippet :deep(mark) {
  background: rgba(0,0,0,0.1);
  color: #000;
  padding: 0 2px;
}

.accounts-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.account-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px !important;
  cursor: pointer;
}

.account-item:hover {
  transform: translate(2px, 2px);
  box-shadow: 1px 1px 0 #000 !important;
}

.account-item.active {
  border-color: #000 !important;
  border-width: 3px !important;
}

.account-info {
  flex: 1;
  min-width: 0;
}

.account-name {
  font-weight: 700;
  font-size: 13px;
  color: #000;
}

.account-meta {
  font-size: 10px;
  margin-top: 2px;
  color: rgba(0,0,0,0.5) !important;
}

.active-badge {
  font-size: 9px;
  font-weight: 700;
  padding: 2px 8px;
  background: #000;
  color: #FFFFFF;
  border: 2px solid #000;
  letter-spacing: 0.5px;
}

.loose-groups-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.loose-group-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px !important;
}

.lg-info {
  flex: 1;
  min-width: 0;
}

.lg-name {
  font-weight: 700;
  color: #000;
}

.lg-count {
  font-size: 10px;
  font-family: 'Courier New', monospace;
  color: rgba(0,0,0,0.5) !important;
}

.style-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.panel-page {
  padding: 12px;
}

.empty-state {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 160px;
}

.bw-title {
  font-family: 'Courier New', monospace;
  font-size: 11px;
  font-weight: 700;
  color: #000;
  margin-bottom: 12px;
  letter-spacing: 1px;
}

.overlay-thin {
  position: fixed;
  inset: 0;
  background: rgba(0,0,0,0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.mini-modal {
  background: #FFFFFF;
  border: 2px solid #000000;
  box-shadow: 4px 4px 0 #000000;
  padding: 16px;
  width: 300px;
  font-family: 'Courier New', monospace;
}

.mini-header {
  font-size: 12px;
  font-weight: 800;
  color: #000000;
  margin-bottom: 10px;
  letter-spacing: 1px;
}

.mini-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.fav-list, .recent-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.fav-item, .recent-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  cursor: pointer;
  border: 2px solid transparent;
}

.fav-item:hover, .recent-item:hover {
  border-color: #000000;
}

.fav-icon, .recent-icon {
  font-size: 11px;
  flex-shrink: 0;
}

.fav-name, .recent-name {
  font-size: 11px;
  font-weight: 600;
  color: #000;
  flex: 1;
}

.recent-info {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.recent-date {
  font-size: 9px;
  color: rgba(0,0,0,0.5) !important;
}

.fav-meta {
  font-size: 9px;
  color: rgba(0,0,0,0.5) !important;
}

@media (max-width: 768px) {
  .right-panel {
    position: fixed;
    right: 0;
    top: 48px;
    bottom: 24px;
    width: 100vw;
    min-width: 100vw;
    z-index: 50;
  }

  .view-toolbar {
    flex-wrap: wrap;
    gap: 4px;
    min-height: auto;
    padding: 4px 6px;
  }

  .toolbar-right {
    flex-wrap: wrap;
  }

  .tb-nav-btn, .view-btn {
    min-width: 36px;
    min-height: 36px;
    font-size: 13px;
  }

  .breadcrumb {
    font-size: 9px;
    max-width: 120px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
}

@media (min-width: 769px) and (max-width: 1024px) {
  .breadcrumb {
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
}

/* Panel transition */
.panel-fade-enter-active,
.panel-fade-leave-active {
  transition: opacity 0.15s ease;
}
.panel-fade-enter-from,
.panel-fade-leave-to {
  opacity: 0;
}

/* Slide right transition */
.slide-right-enter-active,
.slide-right-leave-active {
  transition: transform 0.2s ease, opacity 0.2s ease;
}
.slide-right-enter-from {
  transform: translateX(40px);
  opacity: 0;
}
.slide-right-leave-to {
  transform: translateX(40px);
  opacity: 0;
}

.trash-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.trash-actions {
  display: flex;
  gap: 4px;
}

.bw-btn-sm {
  padding: 2px 6px !important;
  font-size: 9px !important;
}

.bw-btn-danger {
  border-color: #FFFFFF !important;
}

.bw-btn-danger:hover {
  background: #FFFFFF !important;
  color: #000 !important;
}

.trash-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
  max-height: 400px;
  overflow-y: auto;
}

.trash-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border: 2px solid transparent;
  cursor: default;
}

.trash-item:hover {
  border-color: #000;
}

.trash-icon { font-size: 11px; flex-shrink: 0; }

.trash-info {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.trash-name { font-size: 11px; font-weight: 600; color: #000; }
.trash-date { font-size: 9px; }

.trash-action-btn {
  background: transparent;
  border: 2px solid #000;
  color: #000;
  padding: 1px 4px;
  font-family: 'Courier New', monospace;
  font-size: 8px;
  font-weight: 700;
  cursor: pointer;
}

.trash-action-btn:hover {
  background: #000;
  color: #FFF;
}

.trash-action-btn.danger:hover {
  background: #000;
  color: #FFF;
}

.activity-list {
  display: flex;
  flex-direction: column;
  gap: 3px;
  max-height: 400px;
  overflow-y: auto;
}

.activity-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 6px;
  font-size: 9px;
  border-bottom: 1px solid rgba(0,0,0,0.1);
}

.activity-action {
  font-weight: 700;
  color: #000;
  flex-shrink: 0;
  min-width: 60px;
}

.activity-entity {
  flex-shrink: 0;
  min-width: 40px;
}

.activity-date {
  flex-shrink: 0;
}

.activity-detail {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  text-align: right;
}

.text-muted { color: rgba(255,255,255,0.5) !important; }
.text-muted-inverse { color: rgba(0,0,0,0.5) !important; }
.truncate { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
