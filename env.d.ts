/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_BASE?: string
  readonly VITE_TRANSPORT?: 'tauri' | 'rest' | 'wasm'
  readonly VITE_API_URL?: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}

declare module '*.vue' {
  import type { DefineComponent } from 'vue'
  const component: DefineComponent<{}, {}, any>
  export default component
}

declare module '*?raw' {
  const content: string
  export default content
}
