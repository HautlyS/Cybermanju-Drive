<template>
  <Teleport to="body">
    <div
      v-if="store.showShortcutsHelp"
      class="ks-help-overlay"
      @click.self="store.showShortcutsHelp = false"
    >
      <div class="ks-help-modal" role="dialog" aria-label="Keyboard shortcuts">
        <div class="ks-help-header">
          <h2>KEYBOARD SHORTCUTS</h2>
          <button class="close-btn" @click="store.showShortcutsHelp = false">X</button>
        </div>
        <div class="ks-help-body">
          <div v-for="group in groupedShortcuts" :key="group.label" class="ks-group">
            <div class="ks-group-label">{{ group.label }}</div>
            <div v-for="s in group.shortcuts" :key="s.action" class="ks-row">
              <span class="ks-key">{{ s.keys }}</span>
              <span class="ks-desc">{{ s.description }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, inject } from 'vue'
import { useAppStore } from '@/stores/app'
import { ShortcutsKey } from '@/composables/shortcutsKey'
import type { ShortcutEntry } from '@/composables/useShortcuts'

const store = useAppStore()
const shortcuts = inject(ShortcutsKey)

const allShortcuts = computed<ShortcutEntry[]>(() => {
  return shortcuts?.getAllShortcuts() || []
})

const groupLabels: Record<string, string> = {
  'Global Shortcuts': 'GLOBAL',
  'Navigation': 'NAVIGATION',
  'File Operations': 'FILE OPERATIONS',
  'View': 'VIEW',
  'Panels': 'PANELS',
}

const groupedShortcuts = computed(() => {
  const map = new Map<string, ShortcutEntry[]>()
  for (const s of allShortcuts.value) {
    const group = s.group || 'Other'
    if (!map.has(group)) map.set(group, [])
    map.get(group)!.push(s)
  }
  return Array.from(map.entries()).map(([group, shortcuts]) => ({
    label: groupLabels[group] || group,
    shortcuts,
  }))
})
</script>

<style scoped>
.ks-help-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.85);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10001;
}

.ks-help-modal {
  width: 520px;
  max-width: 90vw;
  max-height: 70vh;
  background: #FFFFFF;
  border: 2px solid #000000;
  box-shadow: 4px 4px 0 #000000;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.ks-help-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 2px solid #000000;
  background: #000000;
  color: #FFFFFF;
}

.ks-help-header h2 {
  font-family: 'Courier New', monospace;
  font-size: 13px;
  font-weight: 800;
  letter-spacing: 1px;
  margin: 0;
}

.close-btn {
  background: none;
  border: 2px solid #FFFFFF;
  color: #FFFFFF;
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  font-family: 'Courier New', monospace;
  font-size: 10px;
  font-weight: 700;
}

.close-btn:hover {
  background: #FFFFFF;
  color: #000000;
}

.ks-help-body {
  flex: 1;
  overflow-y: auto;
  padding: 8px 0;
}

.ks-group {
  padding: 6px 0;
}

.ks-group-label {
  padding: 2px 16px;
  font-family: 'Courier New', monospace;
  font-size: 9px;
  font-weight: 700;
  color: rgba(0, 0, 0, 0.4);
  letter-spacing: 1px;
  margin-bottom: 2px;
}

.ks-row {
  display: flex;
  align-items: center;
  padding: 3px 16px;
  gap: 12px;
}

.ks-key {
  font-family: 'Courier New', monospace;
  font-size: 10px;
  font-weight: 700;
  color: #000000;
  background: rgba(0, 0, 0, 0.06);
  padding: 1px 6px;
  border: 1px solid #000000;
  min-width: 100px;
  text-align: center;
}

.ks-desc {
  font-family: 'Courier New', monospace;
  font-size: 10px;
  color: rgba(0, 0, 0, 0.7);
}
</style>
