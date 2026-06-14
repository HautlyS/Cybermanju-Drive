import type { InjectionKey } from 'vue'

export interface ShortcutsAPI {
  getShortcut(action: string): string
  getAllShortcuts(): Array<{ action: string; keys: string; group: string; description: string }>
  getComponentActions(componentId: string): string[]
  getContextActions(fileType: string): string[]
  pause(): void
  resume(): void
}

export const ShortcutsKey: InjectionKey<ShortcutsAPI> = Symbol('shortcuts')
