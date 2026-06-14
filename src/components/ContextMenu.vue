<template>
  <Teleport to="body">
    <Transition name="ctx-fade">
      <div
        v-if="ctx.state.visible"
        ref="menuRef"
        class="ctx-menu"
        :style="menuStyle"
        role="menu"
        :aria-label="'CONTEXT MENU'"
        @click.stop
        @contextmenu.prevent
        @keydown="handleKeydown"
        tabindex="-1"
      >
        <template v-for="(item, i) in visibleItems" :key="item.id">
          <div v-if="item.divider" class="ctx-divider" role="separator" />
          <div
            v-else
            class="ctx-item"
            :class="{
              'ctx-disabled': item.disabled,
              'ctx-focused': focusIndex === i,
              'ctx-has-submenu': !!item.submenu?.length,
            }"
            role="menuitem"
            :aria-disabled="item.disabled"
            :aria-haspopup="!!item.submenu?.length"
            :aria-expanded="!!item.submenu?.length && openSubmenuId === item.id ? 'true' : 'false'"
            @mouseenter="onItemHover(i, item)"
            @click="onItemClick(item)"
          >
            <span class="ctx-icon">{{ item.icon || '' }}</span>
            <span class="ctx-label">{{ item.label }}</span>
            <span class="ctx-shortcut text-muted">{{ item.shortcut || '' }}</span>
            <span v-if="item.submenu?.length" class="ctx-arrow">^</span>
          </div>
          <!-- submenu -->
          <Teleport to="body" v-if="item.submenu?.length && openSubmenuId === item.id">
            <div
              class="ctx-menu ctx-submenu"
              :style="submenuStyle(item.submenu || [])"
              role="menu"
              :aria-label="item.label + ' SUBMENU'"
              @click.stop
              @contextmenu.prevent
            >
              <template v-for="(sub, si) in item.submenu" :key="sub.id">
                <div v-if="sub.divider" class="ctx-divider" role="separator" />
                <div
                  v-else
                  class="ctx-item"
                  :class="{ 'ctx-disabled': sub.disabled }"
                  role="menuitem"
                  :aria-disabled="sub.disabled"
                  @click="onItemClick(sub)"
                >
                  <span class="ctx-icon">{{ sub.icon || '' }}</span>
                  <span class="ctx-label">{{ sub.label }}</span>
                  <span class="ctx-shortcut text-muted">{{ sub.shortcut || '' }}</span>
                </div>
              </template>
            </div>
          </Teleport>
        </template>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, ref, watch, nextTick, onMounted, onUnmounted } from 'vue'
import { useContextMenu, type ContextMenuEntry } from '@/composables/useContextMenu'

const ctx = useContextMenu()
const menuRef = ref<HTMLElement | null>(null)
const focusIndex = ref(-1)
const openSubmenuId = ref<string | null>(null)

const visibleItems = computed(() => ctx.state.items)

const menuStyle = computed(() => {
  const vw = window.innerWidth
  const vh = window.innerHeight
  const estW = 200
  const estH = visibleItems.value.length * 28 + 8
  let x = ctx.state.x
  let y = ctx.state.y
  if (x + estW > vw) x = vw - estW - 4
  if (y + estH > vh) y = vh - estH - 4
  if (x < 4) x = 4
  if (y < 4) y = 4
  return { left: x + 'px', top: y + 'px' }
})

function submenuStyle(entries: ContextMenuEntry[]) {
  const vw = window.innerWidth
  const vh = window.innerHeight
  let x = ctx.state.x + 190
  let y = ctx.state.y
  const estH = entries.length * 28 + 8
  if (x + 180 > vw) x = ctx.state.x - 190
  if (y + estH > vh) y = vh - estH - 4
  return { left: x + 'px', top: y + 'px' }
}

function onItemHover(i: number, item: ContextMenuEntry) {
  focusIndex.value = i
  if (item.submenu?.length) {
    openSubmenuId.value = item.id
  } else if (openSubmenuId.value) {
    openSubmenuId.value = null
  }
}

function onItemClick(item: ContextMenuEntry) {
  if (item.disabled) return
  if (item.submenu?.length) return
  ctx.triggerAction(item)
  openSubmenuId.value = null
}

function handleKeydown(e: KeyboardEvent) {
  const items = visibleItems.value.filter(it => !it.divider)
  switch (e.key) {
    case 'ArrowDown':
      e.preventDefault()
      focusIndex.value = focusIndex.value < items.length - 1 ? focusIndex.value + 1 : 0
      break
    case 'ArrowUp':
      e.preventDefault()
      focusIndex.value = focusIndex.value > 0 ? focusIndex.value - 1 : items.length - 1
      break
    case 'ArrowRight': {
      const item = items[focusIndex.value]
      if (item?.submenu?.length) openSubmenuId.value = item.id
      break
    }
    case 'ArrowLeft':
      openSubmenuId.value = null
      break
    case 'Enter':
    case ' ':
      e.preventDefault()
      if (items[focusIndex.value]) onItemClick(items[focusIndex.value])
      break
    case 'Escape':
      if (openSubmenuId.value) {
        openSubmenuId.value = null
      } else {
        ctx.close()
      }
      break
  }
}

function closeOnScroll() {
  if (ctx.state.visible) ctx.close()
}

function closeOnResize() {
  if (ctx.state.visible) ctx.close()
}

watch(() => ctx.state.visible, (v) => {
  if (v) {
    nextTick(() => menuRef.value?.focus())
  } else {
    openSubmenuId.value = null
    focusIndex.value = -1
  }
})

onMounted(() => {
  document.addEventListener('scroll', closeOnScroll, true)
  window.addEventListener('resize', closeOnResize)
  document.addEventListener('click', (e) => {
    if (ctx.state.visible) ctx.close()
  })
})

onUnmounted(() => {
  document.removeEventListener('scroll', closeOnScroll, true)
  window.removeEventListener('resize', closeOnResize)
})
</script>

<style scoped>
.ctx-menu {
  position: fixed;
  z-index: 9999;
  background: #000;
  border: 2px solid #FFFFFF;
  min-width: 180px;
  max-width: 300px;
  padding: 4px 0;
  font-family: 'Courier New', monospace;
  font-size: 10px;
  color: #FFFFFF;
  box-shadow: 4px 4px 0 rgba(255,255,255,0.15);
  outline: none;
}

.ctx-submenu {
  position: fixed;
  z-index: 10000;
}

.ctx-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 5px 12px;
  cursor: pointer;
  user-select: none;
  white-space: nowrap;
}

.ctx-item:hover,
.ctx-focused {
  background: #FFFFFF;
  color: #000;
}

.ctx-disabled {
  opacity: 0.35;
  cursor: default;
  pointer-events: none;
}

.ctx-icon {
  width: 16px;
  flex-shrink: 0;
  font-size: 9px;
}

.ctx-label {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
}

.ctx-shortcut {
  flex-shrink: 0;
  font-size: 9px;
  margin-left: 12px;
}

.ctx-arrow {
  flex-shrink: 0;
  font-size: 9px;
  transform: rotate(90deg);
}

.ctx-divider {
  height: 1px;
  background: rgba(255,255,255,0.2);
  margin: 2px 8px;
}

.ctx-fade-enter-active,
.ctx-fade-leave-active {
  transition: opacity 0.1s ease;
}

.ctx-fade-enter-from,
.ctx-fade-leave-to {
  opacity: 0;
}
</style>
