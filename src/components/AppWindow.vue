<template>
  <div
    class="app-window"
    :class="{ minimized: win.minimized, focused: isFocused }"
    :style="windowStyle"
    @mousedown.prevent="onFocus"
  >
    <div
      class="window-titlebar"
      @mousedown.prevent="startDrag"
      @dblclick="toggleMaximize"
    >
      <div class="titlebar-dots">
        <span class="dot dot-close" @click.stop="onClose" title="Close"></span>
        <span class="dot dot-minimize" @click.stop="onMinimize" title="Minimize"></span>
        <span class="dot dot-maximize" @click.stop="toggleMaximize" title="Maximize"></span>
      </div>
      <div class="titlebar-icon">{{ win.icon }}</div>
      <div class="titlebar-label">{{ win.title }}</div>
      <div class="titlebar-spacer" />
    </div>
    <div class="window-content" ref="contentRef">
      <component :is="win.component" v-bind="win.props" @close="onClose" />
    </div>
    <div class="resize-handle n" @mousedown.prevent.stop="startResize('n')"></div>
    <div class="resize-handle s" @mousedown.prevent.stop="startResize('s')"></div>
    <div class="resize-handle e" @mousedown.prevent.stop="startResize('e')"></div>
    <div class="resize-handle w" @mousedown.prevent.stop="startResize('w')"></div>
    <div class="resize-handle ne" @mousedown.prevent.stop="startResize('ne')"></div>
    <div class="resize-handle nw" @mousedown.prevent.stop="startResize('nw')"></div>
    <div class="resize-handle se" @mousedown.prevent.stop="startResize('se')"></div>
    <div class="resize-handle sw" @mousedown.prevent.stop="startResize('sw')"></div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import type { WindowState } from '@/composables/useWindowManager'

const props = defineProps<{
  win: WindowState
  focused: boolean
}>()

const emit = defineEmits<{
  close: [id: string]
  minimize: [id: string]
  focus: [id: string]
  move: [id: string, x: number, y: number]
  resize: [id: string, w: number, h: number]
}>()

const contentRef = ref<HTMLElement | null>(null)
const isFocused = computed(() => props.focused)
const isMaximized = ref(false)
const savedRect = ref({ x: 0, y: 0, width: 0, height: 0 })

const windowStyle = computed(() => {
  if (props.win.minimized) {
    return { display: 'none' }
  }
  const style: Record<string, string | number> = {
    left: `${props.win.x}px`,
    top: `${props.win.y}px`,
    width: `${props.win.width}px`,
    height: `${props.win.height}px`,
    zIndex: props.win.zIndex,
  }
  return style
})

let dragging = false
let dragStartX = 0
let dragStartY = 0
let dragOrigX = 0
let dragOrigY = 0

function startDrag(e: MouseEvent) {
  if (isMaximized.value) return
  dragging = true
  dragStartX = e.clientX
  dragStartY = e.clientY
  dragOrigX = props.win.x
  dragOrigY = props.win.y
  document.addEventListener('mousemove', onDrag)
  document.addEventListener('mouseup', stopDrag)
}

function onDrag(e: MouseEvent) {
  if (!dragging) return
  const dx = e.clientX - dragStartX
  const dy = e.clientY - dragStartY
  const newX = Math.max(0, dragOrigX + dx)
  const newY = Math.max(0, dragOrigY + dy)
  emit('move', props.win.id, newX, newY)
}

function stopDrag() {
  dragging = false
  document.removeEventListener('mousemove', onDrag)
  document.removeEventListener('mouseup', stopDrag)
}

let resizing = false
let resizeDir = ''
let resizeStartX = 0
let resizeStartY = 0
let resizeOrigX = 0
let resizeOrigY = 0
let resizeOrigW = 0
let resizeOrigH = 0

function startResize(dir: string) {
  if (isMaximized.value) return
  resizing = true
  resizeDir = dir
  resizeStartX = window.event ? (window.event as MouseEvent).clientX : 0
  resizeStartY = window.event ? (window.event as MouseEvent).clientY : 0
  resizeOrigX = props.win.x
  resizeOrigY = props.win.y
  resizeOrigW = props.win.width
  resizeOrigH = props.win.height
  document.addEventListener('mousemove', onResize)
  document.addEventListener('mouseup', stopResize)
}

function onResize(e: MouseEvent) {
  if (!resizing) return
  const dx = e.clientX - resizeStartX
  const dy = e.clientY - resizeStartY
  let newX = resizeOrigX
  let newY = resizeOrigY
  let newW = resizeOrigW
  let newH = resizeOrigH

  if (resizeDir.includes('e')) newW = Math.max(320, resizeOrigW + dx)
  if (resizeDir.includes('w')) {
    newW = Math.max(320, resizeOrigW - dx)
    newX = resizeOrigX + (resizeOrigW - newW)
  }
  if (resizeDir.includes('s')) newH = Math.max(240, resizeOrigH + dy)
  if (resizeDir.includes('n')) {
    newH = Math.max(240, resizeOrigH - dy)
    newY = resizeOrigY + (resizeOrigH - newH)
  }

  if (newX !== resizeOrigX || newY !== resizeOrigY) {
    emit('move', props.win.id, newX, newY)
  }
  emit('resize', props.win.id, newW, newH)
}

function stopResize() {
  resizing = false
  resizeDir = ''
  document.removeEventListener('mousemove', onResize)
  document.removeEventListener('mouseup', stopResize)
}

function onClose() {
  emit('close', props.win.id)
}

function onMinimize() {
  emit('minimize', props.win.id)
}

function onFocus() {
  emit('focus', props.win.id)
}

function toggleMaximize() {
  if (isMaximized.value) {
    isMaximized.value = false
    emit('move', props.win.id, savedRect.value.x, savedRect.value.y)
    emit('resize', props.win.id, savedRect.value.width, savedRect.value.height)
  } else {
    savedRect.value = {
      x: props.win.x,
      y: props.win.y,
      width: props.win.width,
      height: props.win.height,
    }
    isMaximized.value = true
    emit('move', props.win.id, 0, 32)
    const h = window.innerHeight - 32 - 60
    const w = window.innerWidth
    emit('resize', props.win.id, w, h)
  }
}

function handleGlobalKeydown(e: KeyboardEvent) {
  if (!isFocused.value) return
  if (e.key === 'Escape' && isMaximized.value) {
    toggleMaximize()
  }
}

onMounted(() => {
  document.addEventListener('keydown', handleGlobalKeydown)
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleGlobalKeydown)
  stopDrag()
  stopResize()
})
</script>

<style scoped>
.app-window {
  position: absolute;
  display: flex;
  flex-direction: column;
  background: #141414;
  border: 1px solid #2a2a2a;
  border-radius: 8px;
  overflow: hidden;
  box-shadow:
    0 8px 32px rgba(0, 0, 0, 0.6),
    0 2px 8px rgba(0, 0, 0, 0.3),
    inset 0 1px 0 rgba(255, 255, 255, 0.05);
  transition: box-shadow 0.15s, border-color 0.15s;
  min-width: 320px;
  min-height: 240px;
  will-change: left, top, width, height;
}

.app-window.focused {
  border-color: #3a3a3a;
  box-shadow:
    0 12px 48px rgba(0, 255, 65, 0.08),
    0 4px 16px rgba(0, 0, 0, 0.5),
    inset 0 1px 0 rgba(255, 255, 255, 0.08);
}

.app-window.minimized {
  pointer-events: none;
}

.window-titlebar {
  display: flex;
  align-items: center;
  height: 34px;
  padding: 0 10px;
  background: #1a1a1a;
  border-bottom: 1px solid #2a2a2a;
  cursor: default;
  user-select: none;
  flex-shrink: 0;
  gap: 8px;
}

.titlebar-dots {
  display: flex;
  gap: 6px;
  flex-shrink: 0;
}

.dot {
  width: 11px;
  height: 11px;
  border-radius: 50%;
  cursor: pointer;
  transition: filter 0.1s;
}

.dot:hover {
  filter: brightness(1.3);
}

.dot-close {
  background: #ff5f57;
}

.dot-minimize {
  background: #febc2e;
}

.dot-maximize {
  background: #28c840;
}

.titlebar-icon {
  font-size: 11px;
  margin-left: 4px;
  flex-shrink: 0;
}

.titlebar-label {
  font-family: 'Courier New', 'Fira Code', 'JetBrains Mono', monospace;
  font-size: 11px;
  font-weight: 600;
  color: #ccc;
  letter-spacing: 0.3px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.titlebar-spacer {
  flex: 1;
}

.window-content {
  flex: 1;
  overflow: auto;
  position: relative;
  background: #111;
}

.window-content > :deep(*) {
  height: 100%;
}

.resize-handle {
  position: absolute;
  z-index: 10;
}

.resize-handle.n { top: -3px; left: 4px; right: 4px; height: 6px; cursor: n-resize; }
.resize-handle.s { bottom: -3px; left: 4px; right: 4px; height: 6px; cursor: s-resize; }
.resize-handle.e { right: -3px; top: 4px; bottom: 4px; width: 6px; cursor: e-resize; }
.resize-handle.w { left: -3px; top: 4px; bottom: 4px; width: 6px; cursor: w-resize; }
.resize-handle.ne { top: -4px; right: -4px; width: 10px; height: 10px; cursor: ne-resize; }
.resize-handle.nw { top: -4px; left: -4px; width: 10px; height: 10px; cursor: nw-resize; }
.resize-handle.se { bottom: -4px; right: -4px; width: 10px; height: 10px; cursor: se-resize; }
.resize-handle.sw { bottom: -4px; left: -4px; width: 10px; height: 10px; cursor: sw-resize; }
</style>
