<template>
  <header class="top-menu-bar">
    <div class="tmb-left">
      <div class="app-logo" @click="store.currentPanel = 'landing'">
        <span class="logo-brand">CYBERMANJU</span>
        <span class="logo-drive">DRIVE</span>
      </div>
      <nav class="menu-items" ref="menuRef">
        <div
          v-for="item in menuStructure"
          :key="item.id"
          class="menu-item"
          @click="toggleMenu(item.id)"
          @mouseenter="hoverMenu(item.id)"
        >
          <span class="menu-label">{{ item.label }}</span>
          <div v-if="openMenu === item.id" class="menu-dropdown">
            <template v-for="sub in item.children" :key="sub.id">
              <div
                v-if="sub.divider"
                class="menu-divider"
              />
              <div
                v-else
                class="menu-dropdown-item"
                @click.stop="executeMenuItem(sub)"
              >
                <span class="mdi-icon">{{ sub.icon || '' }}</span>
                <span class="mdi-label">{{ sub.label }}</span>
                <span v-if="sub.shortcut" class="mdi-shortcut">{{ sub.shortcut }}</span>
                <span v-if="sub.checked" class="mdi-check">[x]</span>
              </div>
            </template>
          </div>
        </div>
      </nav>
    </div>

    <div class="tmb-center">
      <div class="search-wrap" :class="{ searching: store.isSearching }">
        <span class="search-prompt">&gt;</span>
        <input
          v-model="store.searchQuery"
          class="search-input"
          type="text"
          placeholder="TANTIVY_SEARCH..."
          @keyup.enter="handleSearch"
        />
        <span v-if="store.isSearching" class="search-cursor">_</span>
      </div>
    </div>

    <div class="tmb-right">
      <div class="sys-tray">
        <button
          class="tray-icon"
          :class="{ active: store.encryptionStatus.isEncrypted }"
          @click="wm.open('encryption')"
          title="Encryption: {{ store.encryptionStatus.isEncrypted ? 'ON' : 'OFF' }}"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
            <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
          </svg>
        </button>

        <button
          class="tray-icon"
          :class="{ active: store.compressedFiles.length > 0 }"
          @click="wm.open('compression')"
          title="Compression: {{ store.compressedFiles.length }} files"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/>
            <polyline points="3.27 6.96 12 12.01 20.73 6.96"/>
            <line x1="12" y1="22.08" x2="12" y2="12"/>
          </svg>
        </button>

        <button
          class="tray-icon"
          :class="{ active: store.activeAccount }"
          @click="wm.open('accounts')"
          :title="store.activeAccount?.name || 'No account'"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/>
            <circle cx="12" cy="7" r="4"/>
          </svg>
        </button>

        <button
          class="tray-icon"
          :class="{ active: store.matrixRainEnabled }"
          @click="store.matrixRainEnabled = !store.matrixRainEnabled"
          title="Toggle background effects"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polygon points="22 3 22 15 16 15 16 21 8 21 8 15 2 15 2 3"/>
          </svg>
        </button>

        <button
          class="tray-icon"
          :class="{ active: openWindowCountValue > 0 }"
          @click="store.commandPaletteOpen = true"
          title="Command Palette"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="4" y1="4" x2="10" y2="4"/><line x1="4" y1="10" x2="15" y2="10"/>
            <line x1="4" y1="16" x2="20" y2="16"/>
          </svg>
        </button>

        <button
          class="tray-icon"
          @click="store.showLoginPopup = true"
          :title="store.currentUser ? store.currentUser.username : 'Login'"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4"/>
            <polyline points="10 17 15 12 10 7"/>
            <line x1="15" y1="12" x2="3" y2="12"/>
          </svg>
        </button>
      </div>

      <div class="tmb-separator" />

      <div class="clock" @click="openDateInfo">
        <span class="clock-time">{{ timeStr }}</span>
        <span class="clock-date">{{ dateStr }}</span>
      </div>
    </div>
  </header>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useAppStore } from '@/stores/app'
import { useWindowManager } from '@/composables/useWindowManager'

const store = useAppStore()
const wm = useWindowManager()
const openWindowCountValue = computed(() => wm.windows.value.filter(w => !w.minimized).length)

const timeStr = ref('')
const dateStr = ref('')
let clockTimer: ReturnType<typeof setInterval> | null = null

function updateClock() {
  const now = new Date()
  timeStr.value = now.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
  dateStr.value = now.toLocaleDateString([], { month: 'short', day: 'numeric' })
}

onMounted(() => {
  updateClock()
  clockTimer = setInterval(updateClock, 1000)
})

onUnmounted(() => {
  if (clockTimer) clearInterval(clockTimer)
})

const openMenu = ref<string | null>(null)
const menuRef = ref<HTMLElement | null>(null)

interface MenuItem {
  id: string
  label?: string
  icon?: string
  shortcut?: string
  checked?: boolean
  divider?: true
  action?: () => void
}

interface MenuGroup {
  id: string
  label: string
  children: MenuItem[]
}

const menuStructure = computed(() => { const m: MenuGroup[] = [
  {
    id: 'file',
    label: 'File',
    children: [
      { id: 'new-folder', label: 'New Folder', icon: '[+]', shortcut: 'Ctrl+N', action: () => { store.createFolderPromptOpen = true } },
      { id: 'upload', label: 'Upload Files', icon: '[^]', action: () => { window.dispatchEvent(new CustomEvent('cybermanju:upload')) } },
      { id: 'div1', divider: true },
      { id: 'open-terminal', label: 'Open Terminal', icon: '[>]', action: () => { store.currentPanel = 'landing' } },
      { id: 'div2', divider: true },
      { id: 'settings', label: 'Settings', icon: '[@]', shortcut: 'Ctrl+,', action: () => { wm.open('settings') } },
      { id: 'quit', label: 'Quit', icon: '[X]', action: () => {} },
    ],
  },
  {
    id: 'edit',
    label: 'Edit',
    children: [
      { id: 'cut', label: 'Cut', icon: '[-]', shortcut: 'Ctrl+X', action: () => {} },
      { id: 'copy', label: 'Copy', icon: '[C]', shortcut: 'Ctrl+C', action: () => {} },
      { id: 'paste', label: 'Paste', icon: '[P]', shortcut: 'Ctrl+V', action: () => {} },
      { id: 'div1', divider: true },
      { id: 'select-all', label: 'Select All', icon: '[A]', shortcut: 'Ctrl+A', action: () => { store.selectedFileIds = store.files.map(f => f.id) } },
      { id: 'deselect', label: 'Deselect', icon: '[C]', action: () => { store.selectedFileIds = [] } },
    ],
  },
  {
    id: 'view',
    label: 'View',
    children: [
      { id: 'file-browser', label: 'File Browser', icon: '[#]', shortcut: 'Ctrl+1', action: () => { wm.open('files') } },
      { id: 'collections', label: 'Collections', icon: '[*]', action: () => { wm.open('collections') } },
      { id: 'people', label: 'People (Faces)', icon: '[+]', action: () => { wm.open('faces') } },
      { id: 'map', label: 'Map View', icon: '[@]', action: () => { wm.open('map') } },
      { id: 'code', label: 'Code Intelligence', icon: '[T]', action: () => { wm.open('code') } },
      { id: 'div1', divider: true },
      { id: 'search', label: 'Search', icon: '[S]', shortcut: 'Ctrl+F', action: () => { store.searchQuery = ''; store.currentPanel = 'search' } },
      { id: 'storage', label: 'Storage Dashboard', icon: '[$]', action: () => { wm.open('storage') } },
      { id: 'sync-panel', label: 'Sync Panel', icon: '[~]', action: () => { wm.open('sync') } },
      { id: 'div2', divider: true },
      { id: 'minimize-all', label: 'Minimize All', icon: '[-]', action: () => wm.minimizeAll() },
      { id: 'close-all', label: 'Close All Windows', icon: '[X]', action: () => wm.closeAll() },
    ],
  },
  {
    id: 'tools',
    label: 'Tools',
    children: [
      { id: 'trash', label: 'Trash', icon: '[%]', action: () => { wm.open('trash'); store.fetchTrashItems() } },
      { id: 'activity', label: 'Activity Log', icon: '[~]', action: () => { wm.open('activity'); store.fetchAuditLog() } },
      { id: 'favorites', label: 'Favorites', icon: '[*]', action: () => { wm.open('favorites') } },
      { id: 'recent', label: 'Recent Files', icon: '[T]', action: () => { wm.open('recent') } },
      { id: 'div1', divider: true },
      { id: 'accounts', label: 'Account Manager', icon: '[@]', action: () => { wm.open('accounts') } },
      { id: 'users', label: 'User Management', icon: '[!]', action: () => { wm.open('users'); store.fetchUsers() } },
      { id: 'div2', divider: true },
      { id: 'command-palette', label: 'Command Palette', icon: '[K]', shortcut: 'Ctrl+K', action: () => { store.commandPaletteOpen = true } },
      { id: 'keyboard-shortcuts', label: 'Keyboard Shortcuts', icon: '[?]', action: () => { store.showShortcutsHelp = true } },
    ],
  },
  {
    id: 'help',
    label: 'Help',
    children: [
      { id: 'about', label: 'About Cybermanju Drive', icon: '[i]', action: () => {} },
      { id: 'docs', label: 'Documentation', icon: '[D]', action: () => { window.open('https://github.com/hautlythird211/Cybermanju-Drive', '_blank') } },
      { id: 'div1', divider: true },
      { id: 'matrix', label: 'Toggle Matrix Rain', icon: '[~]', checked: store.matrixRainEnabled, action: () => { store.matrixRainEnabled = !store.matrixRainEnabled } },
    ],
  },
]; return m; })

function toggleMenu(id: string) {
  openMenu.value = openMenu.value === id ? null : id
}

function hoverMenu(id: string) {
  if (openMenu.value !== null) {
    openMenu.value = id
  }
}

function executeMenuItem(item: any) {
  openMenu.value = null
  item.action?.()
}

function handleSearch() {
  if (store.searchQuery.trim()) {
    store.searchFiles(store.searchQuery)
    wm.open('search')
  }
}

function openDateInfo() {
  wm.open('settings')
}

function handleClickOutside(e: MouseEvent) {
  if (menuRef.value && !menuRef.value.contains(e.target as Node)) {
    openMenu.value = null
  }
}

onMounted(() => {
  document.addEventListener('click', handleClickOutside)
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
})
</script>

<style scoped>
.top-menu-bar {
  display: flex;
  align-items: center;
  height: 32px;
  padding: 0 8px;
  background: #111;
  border-bottom: 1px solid #222;
  z-index: 100;
  position: relative;
  gap: 8px;
  -webkit-app-region: drag;
  user-select: none;
}

.tmb-left {
  display: flex;
  align-items: center;
  gap: 4px;
  -webkit-app-region: no-drag;
}

.app-logo {
  display: flex;
  align-items: baseline;
  gap: 3px;
  padding: 0 8px;
  cursor: pointer;
  border-right: 1px solid #222;
  margin-right: 4px;
}

.app-logo:hover .logo-brand {
  color: #00ff41;
}

.logo-brand {
  font-family: 'Courier New', monospace;
  font-size: 11px;
  font-weight: 800;
  letter-spacing: 1.5px;
  color: #e0e0e0;
  transition: color 0.15s;
}

.logo-drive {
  font-family: 'Courier New', monospace;
  font-size: 10px;
  font-weight: 600;
  color: #555;
  letter-spacing: 0.5px;
}

.menu-items {
  display: flex;
  align-items: center;
  gap: 0;
}

.menu-item {
  position: relative;
  padding: 4px 10px;
  cursor: pointer;
  border-radius: 4px;
}

.menu-item:hover {
  background: #1a1a1a;
}

.menu-label {
  font-family: 'Courier New', monospace;
  font-size: 11px;
  color: #aaa;
  letter-spacing: 0.2px;
  font-weight: 500;
}

.menu-item:hover .menu-label {
  color: #e0e0e0;
}

.menu-dropdown {
  position: absolute;
  top: 100%;
  left: 0;
  min-width: 220px;
  background: #181818;
  border: 1px solid #2a2a2a;
  border-radius: 6px;
  padding: 4px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.6);
  z-index: 200;
}

.menu-dropdown-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  font-family: 'Courier New', monospace;
  font-size: 11px;
  color: #ccc;
  cursor: pointer;
  border-radius: 4px;
  transition: background 0.1s;
}

.menu-dropdown-item:hover {
  background: #222;
  color: #fff;
}

.mdi-icon {
  width: 18px;
  text-align: center;
  font-size: 10px;
  color: #666;
}

.mdi-label {
  flex: 1;
}

.mdi-shortcut {
  font-size: 9px;
  color: #555;
  margin-left: auto;
}

.mdi-check {
  color: #00ff41;
  font-size: 9px;
}

.menu-divider {
  height: 1px;
  background: #2a2a2a;
  margin: 4px 6px;
}

.tmb-center {
  flex: 1;
  display: flex;
  justify-content: center;
  max-width: 360px;
  margin: 0 auto;
}

.search-wrap {
  position: relative;
  display: flex;
  align-items: center;
  width: 100%;
  background: #1a1a1a;
  border: 1px solid #2a2a2a;
  border-radius: 6px;
  padding: 0 8px;
  height: 22px;
  transition: border-color 0.15s;
}

.search-wrap:focus-within {
  border-color: #00ff41;
}

.search-prompt {
  color: #555;
  font-family: 'Courier New', monospace;
  font-size: 10px;
  margin-right: 4px;
}

.search-input {
  flex: 1;
  background: transparent;
  border: none;
  color: #e0e0e0;
  font-family: 'Courier New', monospace;
  font-size: 10px;
  height: 100%;
  outline: none;
}

.search-input::placeholder {
  color: #444;
}

.search-cursor {
  color: #00ff41;
  animation: blink 0.8s step-end infinite;
  font-size: 10px;
}

@keyframes blink {
  50% { opacity: 0; }
}

.tmb-right {
  display: flex;
  align-items: center;
  gap: 6px;
  -webkit-app-region: no-drag;
}

.sys-tray {
  display: flex;
  align-items: center;
  gap: 2px;
}

.tray-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 22px;
  color: #666;
  cursor: pointer;
  border-radius: 4px;
  transition: all 0.1s;
  background: transparent;
  border: none;
}

.tray-icon:hover {
  color: #e0e0e0;
  background: #1a1a1a;
}

.tray-icon.active {
  color: #00ff41;
}

.tmb-separator {
  width: 1px;
  height: 16px;
  background: #222;
  flex-shrink: 0;
}

.clock {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  padding: 0 6px;
  cursor: pointer;
  border-radius: 4px;
  transition: background 0.1s;
}

.clock:hover {
  background: #1a1a1a;
}

.clock-time {
  font-family: 'Courier New', monospace;
  font-size: 10px;
  font-weight: 600;
  color: #ccc;
  line-height: 1.2;
}

.clock-date {
  font-family: 'Courier New', monospace;
  font-size: 8px;
  color: #555;
  line-height: 1.2;
}
</style>
