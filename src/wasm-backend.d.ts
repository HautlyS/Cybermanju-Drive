// Types for the `cybermanju-drive-wasm` build artifact.
//
// In the static/WASM bundle this resolves to `crates/drive-wasm/pkg`
// (vite.config.wasm.ts alias); in desktop builds the virtual-module stub
// (vite-plugin-wasm-stub) stands in. Declared here so `vue-tsc` is
// deterministic whether or not `pkg/` has been built.
declare module 'cybermanju-drive-wasm' {
  const init: () => Promise<void>;
  export default init;
  export function os_dispatch(cmd: string, argsJson: string): string;
  export function db_open(): Promise<string>;
  export function db_dispatch(op: string, argsJson: string): string;
}
