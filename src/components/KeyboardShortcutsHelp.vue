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
          <button class="close-btn" @click="store.showShortcutsHelp = false" aria-label="CLOSE"><AppIcon name="solar:close-bold" :size="13" /></button>
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
import AppIcon from '@/components/AppIcon.vue'
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
  background: color-mix(in srgb, var(--ui-bg-deep) 62%, transparent);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10001;
  backdrop-filter: blur(6px);
  -webkit-backdrop-filter: blur(6px);
}

.ks-help-modal {width: 520px;
  max-width: 90vw;
  max-height: 70vh;
  background: var(--ui-glass-2);
  border: 1px solid var(--ui-border);
  box-shadow: var(--ui-shadow-2);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border-radius: var(--ui-radius-lg);
  backdrop-filter: blur(var(--ui-blur)) saturate(var(--ui-saturate));
  -webkit-backdrop-filter: blur(var(--ui-blur)) saturate(var(--ui-saturate));
}

.ks-help-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid var(--ui-border);
  background: var(--ui-surface);
  color: var(--ui-text);
}

.ks-help-header h2 {
  font-family: var(--ui-font);
  font-size: 13px;
  font-weight: 800;
  letter-spacing: 1px;
  margin: 0;
}

.close-btn {
  background: none;
  border: 1px solid var(--ui-border);
  color: var(--ui-text);
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  font-family: var(--ui-font);
  font-size: 10px;
  font-weight: 700;
}

.close-btn:hover {
  background: var(--ui-glass-2);
  color: var(--ui-text);
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
  font-family: var(--ui-font);
  font-size: 9px;
  font-weight: 700;
  color: color-mix(in srgb, var(--ui-text) 40%, transparent);
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
  font-family: var(--ui-font);
  font-size: 10px;
  font-weight: 700;
  color: var(--ui-text);
  background: color-mix(in srgb, var(--ui-bg-deep) 4%, transparent);
  padding: 1px 6px;
  border: 1px solid var(--ui-border-strong);
  min-width: 100px;
  text-align: center;
}

.ks-desc {
  font-family: var(--ui-font);
  font-size: 10px;
  color: color-mix(in srgb, var(--ui-text) 70%, transparent);
}
</style>
