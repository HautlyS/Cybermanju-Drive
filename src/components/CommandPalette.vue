<template>
  <Teleport to="body">
    <div
      v-if="store.commandPaletteOpen"
      class="cp-overlay"
      @click.self="close"
    >
      <div ref="cpRef" class="cp-modal" role="dialog" aria-label="Command palette">
        <div class="cp-header">
          <span class="cp-prompt">&gt;</span>
          <input
            ref="inputRef"
            v-model="query"
            class="cp-input"
            placeholder="TYPE COMMAND..."
            aria-label="SEARCH COMMANDS"
            @keydown="handleKeydown"
            @input="filterCommands"
          />
        </div>
        <div class="cp-results">
          <div
            v-for="(group, gi) in filteredGroups"
            :key="gi"
            class="cp-group"
          >
            <div class="cp-group-label">{{ group.label }}</div>
            <div
              v-for="(cmd, ci) in group.items"
              :key="cmd.id"
              class="cp-item"
              :class="{ active: activeIndex === getGlobalIndex(gi, ci) }"
              @click="execute(cmd)"
              @mouseenter="activeIndex = getGlobalIndex(gi, ci)"
            >
              <span class="cp-item-icon">{{ cmd.icon }}</span>
              <span class="cp-item-label">{{ cmd.label }}</span>
              <span v-if="cmd.shortcut" class="cp-item-shortcut">{{ cmd.shortcut }}</span>
            </div>
          </div>
          <div v-if="allCommandsFiltered.length === 0" class="cp-empty text-muted">
            NO MATCHING COMMANDS
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, toRef, computed, nextTick, watch } from 'vue'
import { useAppStore } from '@/stores/app'
import type { PanelType } from '@/types'
import { useFocusTrap } from '@/composables/useFocusTrap'

const store = useAppStore()
const cpRef = ref<HTMLElement | null>(null)
useFocusTrap(cpRef, toRef(store, 'commandPaletteOpen'))

interface Command {
  id: string
  label: string
  icon: string
  shortcut?: string
  action: () => void
}

interface CommandGroup {
  label: string
  items: Command[]
}

const query = ref('')
const activeIndex = ref(0)
const inputRef = ref<HTMLInputElement | null>(null)

const commands = computed<CommandGroup[]>(() => [
  {
    label: 'NAVIGATION',
    items: [
      { id: 'nav-files', label: 'Go to Files', icon: '[#]', shortcut: '', action: () => { store.currentPanel = 'files' } },
      { id: 'nav-search', label: 'Go to Search', icon: '[S]', action: () => { store.currentPanel = 'search' } },
      { id: 'nav-collections', label: 'Go to Collections', icon: '[*]', action: () => { store.currentPanel = 'collections' } },
      { id: 'nav-faces', label: 'Go to People (Faces)', icon: '[+]', action: () => { store.currentPanel = 'faces' } },
      { id: 'nav-map', label: 'Go to Map', icon: '[@]', action: () => { store.currentPanel = 'map'; store.fetchGeoFiles() } },
      { id: 'nav-code', label: 'Go to Code Intelligence', icon: '[T]', action: () => { store.currentPanel = 'code' } },
      { id: 'nav-sync', label: 'Go to Sync', icon: '[~]', action: () => { store.currentPanel = 'sync' } },
      { id: 'nav-users', label: 'Go to User Management', icon: '[!]', action: () => { store.currentPanel = 'users' } },
      { id: 'nav-settings', label: 'Go to Settings', icon: '[@]', action: () => { store.currentPanel = 'settings' as PanelType } },
      { id: 'nav-trash', label: 'Go to Trash', icon: '[%]', action: () => { store.currentPanel = 'trash' as PanelType } },
      { id: 'nav-favorites', label: 'Go to Favorites', icon: '[*]', action: () => { store.currentPanel = 'favorites' as PanelType } },
      { id: 'nav-recent', label: 'Go to Recent Files', icon: '[T]', action: () => { store.currentPanel = 'recent' as PanelType } },
      { id: 'nav-activity', label: 'Go to Activity Log', icon: '[~]', action: () => { store.currentPanel = 'activity' as PanelType } },
    ],
  },
  {
    label: 'ACTIONS',
    items: [
      { id: 'act-new-folder', label: 'New Folder', icon: '[+]', shortcut: 'Ctrl+N', action: () => { store.createFolderPromptOpen = true } },
      { id: 'act-encrypt', label: 'Encrypt Selected File', icon: '[@]', shortcut: 'Ctrl+E', action: () => { if (store.selectedFileId) store.showEncryptionPanel = true } },
      { id: 'act-compress', label: 'Compress Selected File', icon: '[$]', shortcut: 'Ctrl+Shift+C', action: () => { if (store.selectedFileId) store.showCompressionPanel = true } },
      { id: 'act-batch-detect', label: 'Batch Face Detection', icon: '[+]', action: () => { store.detectFacesBatch() } },
      { id: 'act-refresh', label: 'Refresh Files', icon: '[R]', action: () => { store.fetchFiles() } },
    ],
  },
  {
    label: 'VIEW',
    items: [
      { id: 'view-grid', label: 'Grid View', icon: '[#]', shortcut: 'Ctrl+G', action: () => { store.viewMode = 'grid' } },
      { id: 'view-list', label: 'List View', icon: '[#]', shortcut: 'Ctrl+L', action: () => { store.viewMode = 'list' } },
      { id: 'view-masonry', label: 'Masonry View', icon: '[#]', shortcut: 'Ctrl+M', action: () => { store.viewMode = 'masonry' } },
      { id: 'view-toggle-sidebar', label: 'Toggle Sidebar', icon: '[#]', shortcut: 'Ctrl+B', action: () => { store.sidebarCollapsed = !store.sidebarCollapsed } },
      { id: 'view-toggle-matrix', label: 'Toggle Matrix Rain', icon: '[~]', action: () => { store.matrixRainEnabled = !store.matrixRainEnabled } },
    ],
  },
  {
    label: 'PANELS',
    items: [
      { id: 'panel-encryption', label: 'Toggle Encryption Panel', icon: '[@]', shortcut: 'Ctrl+E', action: () => { store.showEncryptionPanel = !store.showEncryptionPanel } },
      { id: 'panel-compression', label: 'Toggle Compression Panel', icon: '[$]', shortcut: 'Ctrl+Shift+C', action: () => { store.showCompressionPanel = !store.showCompressionPanel } },
    ],
  },
])

function filterCommands() {
  activeIndex.value = 0
}

interface Cmd { id: string; label: string; icon: string; shortcut?: string; action: () => void }
interface CmdGroup { label: string; items: Cmd[] }

const filteredGroups = computed(() => {
  const q = query.value.toLowerCase().trim()
  if (!q) return commands.value as CmdGroup[]
  return (commands.value as CmdGroup[])
    .map((g: CmdGroup) => ({
      ...g,
      items: g.items.filter((c: Cmd) => c.label.toLowerCase().includes(q) || c.id.toLowerCase().includes(q)),
    }))
    .filter((g: CmdGroup) => g.items.length > 0)
})

const allCommandsFiltered = computed(() => {
  return filteredGroups.value.flatMap((g: CmdGroup) => g.items)
})

function getGlobalIndex(gi: number, ci: number): number {
  let idx = 0
  const groups = filteredGroups.value as CmdGroup[]
  for (let g = 0; g < gi; g++) {
    idx += groups[g].items.length
  }
  return idx + ci
}

function handleKeydown(e: KeyboardEvent) {
  const total = allCommandsFiltered.value.length
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    activeIndex.value = (activeIndex.value + 1) % Math.max(total, 1)
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    activeIndex.value = (activeIndex.value - 1 + Math.max(total, 1)) % Math.max(total, 1)
  } else if (e.key === 'Enter') {
    e.preventDefault()
    const cmd = allCommandsFiltered.value[activeIndex.value]
    if (cmd) execute(cmd)
  } else if (e.key === 'Escape') {
    close()
  }
}

function execute(cmd: Command) {
  cmd.action()
  close()
}

function close() {
  store.commandPaletteOpen = false
  query.value = ''
}

watch(() => store.commandPaletteOpen, async (v: boolean) => {
  if (v) {
    await nextTick()
    inputRef.value?.focus()
    query.value = ''
    activeIndex.value = 0
  }
})
</script>

<style scoped>
.cp-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.85);
  display: flex;
  align-items: flex-start;
  justify-content: center;
  padding-top: 120px;
  z-index: 10000;
}

.cp-modal {
  width: 480px;
  max-width: 90vw;
  max-height: 400px;
  background: #FFFFFF;
  border: 2px solid #000000;
  box-shadow: 4px 4px 0 #000000;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.cp-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-bottom: 2px solid #000000;
  background: #000000;
}

.cp-prompt {
  color: #FFFFFF;
  font-family: 'Courier New', monospace;
  font-size: 13px;
  font-weight: 700;
}

.cp-input {
  flex: 1;
  background: transparent;
  border: none;
  color: #FFFFFF;
  font-family: 'Courier New', monospace;
  font-size: 12px;
  outline: none;
}

.cp-input::placeholder {
  color: rgba(255, 255, 255, 0.4);
}

.cp-results {
  flex: 1;
  overflow-y: auto;
  padding: 4px 0;
}

.cp-group {
  padding: 4px 0;
}

.cp-group-label {
  padding: 4px 12px;
  font-family: 'Courier New', monospace;
  font-size: 9px;
  font-weight: 700;
  color: rgba(0, 0, 0, 0.4);
  letter-spacing: 1px;
}

.cp-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  cursor: pointer;
  font-family: 'Courier New', monospace;
  font-size: 11px;
  color: #000000;
}

.cp-item:hover,
.cp-item.active {
  background: #000000;
  color: #FFFFFF;
}

.cp-item-icon {
  font-size: 11px;
  width: 20px;
  text-align: center;
  flex-shrink: 0;
}

.cp-item-label {
  flex: 1;
}

.cp-item-shortcut {
  font-size: 9px;
  color: rgba(0, 0, 0, 0.4);
  margin-left: 12px;
}

.cp-item:hover .cp-item-shortcut,
.cp-item.active .cp-item-shortcut {
  color: rgba(255, 255, 255, 0.5);
}

.cp-empty {
  padding: 24px;
  text-align: center;
  font-size: 10px;
}

.text-muted { opacity: 0.5; }
</style>
