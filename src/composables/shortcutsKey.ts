import type { InjectionKey } from 'vue'
import type { ShortcutGroup } from '@/composables/useShortcuts'

export interface ShortcutsAPI {
  getShortcut(action: string): string
  getAllShortcuts(): Array<{ action: string; keys: string; group: ShortcutGroup; description: string }>
  getComponentActions(componentId: string): string[]
  getContextActions(fileType: string): string[]
  pause(): void
  resume(): void
}

export const ShortcutsKey: InjectionKey<ShortcutsAPI> = Symbol('shortcuts')
