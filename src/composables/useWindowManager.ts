import { ref, computed, markRaw, type Component } from 'vue'
import type { PanelType } from '@/types'
import { MODULE_METADATA } from '@/types'
import FileGrid from '@/components/FileGrid.vue'
import CollectionsPanel from '@/components/CollectionsPanel.vue'
import FaceGroupingPanel from '@/components/FaceGroupingPanel.vue'
import MapView from '@/components/MapView.vue'
import CodeIntelligencePanel from '@/components/CodeIntelligencePanel.vue'
import UserManagementPanel from '@/components/UserManagementPanel.vue'
import WebDashboardPanel from '@/components/WebDashboardPanel.vue'
import SyncPanel from '@/components/SyncPanel.vue'
import SettingsPage from '@/components/SettingsPage.vue'
import StorageDashboard from '@/components/StorageDashboard.vue'
import EncryptionPanel from '@/components/EncryptionPanel.vue'
import CompressionPanel from '@/components/CompressionPanel.vue'
import FilePermissionsPanel from '@/components/FilePermissionsPanel.vue'
import FilePreview from '@/components/FilePreview.vue'
import WindowContent from '@/components/WindowContent.vue'

export interface WindowState {
  id: string
  panelType: PanelType
  title: string
  icon: string
  x: number
  y: number
  width: number
  height: number
  minimized: boolean
  zIndex: number
  component: Component | null
  props?: Record<string, unknown>
}

type SizeMap = { [K in PanelType]?: { width: number; height: number } } & {
  permissions?: { width: number; height: number }
}
const defaultSizes: SizeMap = {
  files: { width: 900, height: 580 },
  search: { width: 600, height: 480 },
  collections: { width: 500, height: 420 },
  faces: { width: 600, height: 460 },
  map: { width: 720, height: 520 },
  code: { width: 650, height: 500 },
  users: { width: 520, height: 460 },
  sync: { width: 580, height: 440 },
  settings: { width: 560, height: 520 },
  trash: { width: 500, height: 400 },
  activity: { width: 540, height: 400 },
  favorites: { width: 420, height: 360 },
  recent: { width: 420, height: 360 },
  accounts: { width: 480, height: 400 },
  'loose-groups': { width: 440, height: 380 },
  style: { width: 440, height: 360 },
  storage: { width: 580, height: 480 },
  dashboard: { width: 600, height: 460 },
  webdash: { width: 640, height: 500 },
  encryption: { width: 480, height: 420 },
  compression: { width: 480, height: 420 },
  permissions: { width: 440, height: 380 },
  preview: { width: 480, height: 540 },
}

const inlinePanels: PanelType[] = [
  'search', 'trash', 'activity', 'favorites', 'recent',
  'accounts', 'loose-groups', 'style'
]

const panelComponentMap: Record<string, Component> = {
  files: FileGrid,
  collections: CollectionsPanel,
  faces: FaceGroupingPanel,
  map: MapView,
  code: CodeIntelligencePanel,
  users: UserManagementPanel,
  dashboard: WebDashboardPanel,
  sync: SyncPanel,
  settings: SettingsPage,
  storage: StorageDashboard,
  encryption: EncryptionPanel,
  compression: CompressionPanel,
  permissions: FilePermissionsPanel,
  preview: FilePreview,
  webdash: WebDashboardPanel,
}

function getComponent(panelType: PanelType): Component | null {
  if (panelComponentMap[panelType]) return markRaw(panelComponentMap[panelType])
  if (inlinePanels.includes(panelType)) return markRaw(WindowContent)
  return null
}

let windowCounter = 0

const windows = ref<WindowState[]>([])
const nextZIndex = ref(10)
const windowFocusHistory = ref<string[]>([])

function cascadePosition(index: number): { x: number; y: number } {
  const offset = 30 + (index % 10) * 28
  return { x: offset, y: offset }
}

export function useWindowManager() {
  const activeWindow = computed(() => {
    if (windowFocusHistory.value.length === 0) return null
    const id = windowFocusHistory.value[windowFocusHistory.value.length - 1]
    return windows.value.find(w => w.id === id) || null
  })

  function open(panelType: PanelType, props?: Record<string, unknown>) {
    const existing = windows.value.find(
      w => w.panelType === panelType && !w.minimized
    )
    if (existing) {
      focus(existing.id)
      return existing.id
    }

    const meta = MODULE_METADATA[panelType] || { label: panelType.toUpperCase(), icon: '[*]' }
    const size = defaultSizes[panelType] || { width: 520, height: 440 }
    const pos = cascadePosition(windows.value.length)
    const id = `win-${++windowCounter}`
    const comp = getComponent(panelType)

    const resolvedProps = { ...(props || {}) }
    if (inlinePanels.includes(panelType)) {
      resolvedProps.panelType = panelType
    }

    const win: WindowState = {
      id,
      panelType,
      title: meta.label,
      icon: meta.icon,
      x: pos.x,
      y: pos.y,
      width: size.width,
      height: size.height,
      minimized: false,
      zIndex: nextZIndex.value++,
      component: comp,
      props: resolvedProps,
    }
    windows.value.push(win)
    windowFocusHistory.value = windowFocusHistory.value.filter(w => w !== id)
    windowFocusHistory.value.push(id)
    return id
  }

  function close(id: string) {
    windows.value = windows.value.filter(w => w.id !== id)
    windowFocusHistory.value = windowFocusHistory.value.filter(w => w !== id)
  }

  function minimize(id: string) {
    const win = windows.value.find(w => w.id === id)
    if (win) {
      win.minimized = true
      windowFocusHistory.value = windowFocusHistory.value.filter(w => w !== id)
    }
  }

  function restore(id: string) {
    const win = windows.value.find(w => w.id === id)
    if (win) {
      win.minimized = false
      focus(id)
    }
  }

  function focus(id: string) {
    const win = windows.value.find(w => w.id === id)
    if (win) {
      win.zIndex = nextZIndex.value++
      windowFocusHistory.value = windowFocusHistory.value.filter(w => w !== id)
      windowFocusHistory.value.push(id)
    }
  }

  function toggle(panelType: PanelType, props?: Record<string, unknown>) {
    const existing = windows.value.find(
      w => w.panelType === panelType
    )
    if (existing) {
      if (existing.minimized) {
        restore(existing.id)
      } else {
        close(existing.id)
      }
    } else {
      open(panelType, props)
    }
  }

  function closeAll() {
    windows.value = []
    windowFocusHistory.value = []
  }

  function minimizeAll() {
    windows.value.forEach(w => { w.minimized = true })
    windowFocusHistory.value = []
  }

  function updatePosition(id: string, x: number, y: number) {
    const win = windows.value.find(w => w.id === id)
    if (win) {
      win.x = x
      win.y = y
    }
  }

  function updateSize(id: string, width: number, height: number) {
    const win = windows.value.find(w => w.id === id)
    if (win) {
      win.width = Math.max(320, width)
      win.height = Math.max(240, height)
    }
  }

  const openWindowCount = computed(() =>
    windows.value.filter(w => !w.minimized).length
  )

  const isOpen = (panelType: PanelType) =>
    windows.value.some(w => w.panelType === panelType)

  return {
    windows,
    activeWindow,
    nextZIndex,
    open,
    close,
    minimize,
    restore,
    focus,
    toggle,
    closeAll,
    minimizeAll,
    updatePosition,
    updateSize,
    openWindowCount,
    isOpen,
    inlinePanels,
  }
}

export type WindowManager = ReturnType<typeof useWindowManager>
