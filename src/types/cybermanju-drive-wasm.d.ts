// Type declaration for the wasm-pack bundle of `crates/drive-wasm`
// (resolved at build time via the `cybermanju-drive-wasm` alias in
// vite.config.wasm.ts).
declare module 'cybermanju-drive-wasm' {
  /** One entry point for every transport: dispatches an os/* command. */
  export function os_dispatch(cmd: string, args_json: string): string
  /** wasm-pack `--target web` init hook. */
  export default function init(): Promise<void>
}
