<template>
  <div class="desktop-shell">
    <TopMenuBar />

    <div class="desktop-area" ref="desktopRef">
      <div class="desktop-wallpaper">
        <slot name="wallpaper" />
      </div>

      <div class="desktop-workspace">
        <div class="desktop-icons">
          <div class="desktop-shortcut" @dblclick="wm.open('files')">
            <div class="shortcut-icon">[#]</div>
            <div class="shortcut-label">Files</div>
          </div>
          <div class="desktop-shortcut" @dblclick="wm.open('collections')">
            <div class="shortcut-icon">[*]</div>
            <div class="shortcut-label">Collections</div>
          </div>
          <div class="desktop-shortcut" @dblclick="wm.open('map')">
            <div class="shortcut-icon">[@]</div>
            <div class="shortcut-label">Map</div>
          </div>
          <div class="desktop-shortcut" @dblclick="wm.open('code')">
            <div class="shortcut-icon">[T]</div>
            <div class="shortcut-label">Code</div>
          </div>
          <div class="desktop-shortcut" @dblclick="wm.open('settings')">
            <div class="shortcut-icon">[@]</div>
            <div class="shortcut-label">Settings</div>
          </div>
          <div class="desktop-shortcut" @dblclick="store.currentPanel = 'landing'">
            <div class="shortcut-icon">[>]</div>
            <div class="shortcut-label">Terminal</div>
          </div>
        </div>

        <AppWindow
          v-for="win in visibleWindows"
          :key="win.id"
          :win="win"
          :focused="win.id === focusedWindowId"
          @close="wm.close"
          @minimize="wm.minimize"
          @focus="wm.focus"
          @move="wm.updatePosition"
          @resize="wm.updateSize"
        />
      </div>
    </div>

    <Dock />

    <div
      v-if="dockMenu.visible"
      class="dock-context-overlay"
      :style="{ left: dockMenu.x + 'px', top: dockMenu.y + 'px' }"
      @click="dockMenu.visible = false"
      @contextmenu.prevent="dockMenu.visible = false"
    >
      <div class="dock-context-menu" @click.stop>
        <div
          v-for="(item, i) in dockMenu.items"
          :key="i"
          class="dock-context-item"
          @click="item.action(); dockMenu.visible = false"
        >
          {{ item.label }}
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useAppStore } from '@/stores/app'
import { useWindowManager } from '@/composables/useWindowManager'
import TopMenuBar from './TopMenuBar.vue'
import Dock from './Dock.vue'
import AppWindow from './AppWindow.vue'

const store = useAppStore()
const wm = useWindowManager()
const desktopRef = ref<HTMLElement | null>(null)

const visibleWindows = computed(() =>
  wm.windows.value.filter(w => !w.minimized)
)

const focusedWindowId = computed(() => {
  if (wm.windows.value.length === 0) return null
  const sorted = [...wm.windows.value].sort((a, b) => b.zIndex - a.zIndex)
  return sorted[0].id
})

const dockMenu = ref({
  visible: false,
  x: 0,
  y: 0,
  items: [] as { label: string; action: () => void }[],
})

function handleDockContext(e: CustomEvent) {
  dockMenu.value = {
    visible: true,
    x: e.detail.x,
    y: e.detail.y,
    items: e.detail.items,
  }
}

function handleClickOutside() {
  dockMenu.value.visible = false
}

function handleDesktopClick(e: MouseEvent) {
  if (e.target === desktopRef.value || (e.target as HTMLElement)?.closest('.desktop-area')) {
    wm.windows.value.forEach(w => {
      w.zIndex = 1
    })
  }
}

onMounted(() => {
  window.addEventListener('cybermanju:dock-context', handleDockContext as EventListener)
  document.addEventListener('click', handleClickOutside)
  document.addEventListener('contextmenu', () => { dockMenu.value.visible = false })
})

onUnmounted(() => {
  window.removeEventListener('cybermanju:dock-context', handleDockContext as EventListener)
  document.removeEventListener('click', handleClickOutside)
  document.removeEventListener('contextmenu', () => { dockMenu.value.visible = false })
})
</script>

<style scoped>
.desktop-shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
  width: 100vw;
  overflow: hidden;
  background: #0a0a0a;
  position: relative;
}

.desktop-area {
  flex: 1;
  position: relative;
  overflow: hidden;
}

.desktop-wallpaper {
  position: absolute;
  inset: 0;
  pointer-events: none;
  z-index: 0;
}

.desktop-workspace {
  position: absolute;
  inset: 0;
  z-index: 1;
}

.desktop-icons {
  position: absolute;
  top: 12px;
  left: 12px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  z-index: 2;
}

.desktop-shortcut {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  padding: 6px 8px;
  cursor: pointer;
  border-radius: 6px;
  transition: background 0.1s;
  width: 72px;
  text-align: center;
}

.desktop-shortcut:hover {
  background: rgba(255, 255, 255, 0.05);
}

.shortcut-icon {
  font-family: 'Courier New', monospace;
  font-size: 16px;
  font-weight: 700;
  color: #888;
  transition: color 0.1s;
}

.desktop-shortcut:hover .shortcut-icon {
  color: #00ff41;
}

.shortcut-label {
  font-family: 'Courier New', monospace;
  font-size: 9px;
  color: #666;
  white-space: nowrap;
  text-shadow: 0 1px 4px rgba(0, 0, 0, 0.8);
}

.desktop-shortcut:hover .shortcut-label {
  color: #aaa;
}

.dock-context-overlay {
  position: fixed;
  z-index: 10000;
}

.dock-context-menu {
  background: #181818;
  border: 1px solid #2a2a2a;
  border-radius: 8px;
  padding: 4px;
  min-width: 160px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.6);
}

.dock-context-item {
  padding: 6px 12px;
  font-family: 'Courier New', monospace;
  font-size: 11px;
  color: #ccc;
  cursor: pointer;
  border-radius: 4px;
}

.dock-context-item:hover {
  background: #222;
  color: #fff;
}
</style>
