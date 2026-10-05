import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { existsSync } from "fs";
import { resolve } from "path";
import wasmStub from "./vite-plugin-wasm-stub";

const host = process.env.TAURI_DEV_HOST;

// Real wasm-pack output, when present (built at crates/drive-wasm/pkg).
// Otherwise the stub file below stands in — a real file (not a virtual
// module) so worker bundles resolve it deterministically. The entry file
// (not the directory) is aliased: worker pipelines don't apply
// package.json directory resolution the same way the main bundle does.
const pkgEntry = resolve(__dirname, "crates/drive-wasm/pkg/cybermanju_drive_wasm.js");
const wasmTarget = existsSync(pkgEntry)
  ? pkgEntry
  : resolve(__dirname, "src/wasm-stub.js");

export default defineConfig(async () => ({
  plugins: [vue(), wasmStub()],
  // Worker bundles don't inherit config plugins — the db worker imports
  // 'cybermanju-drive-wasm', so the stub must apply there too.
  worker: {
    format: 'es',
    plugins: () => [wasmStub()],
  },
  resolve: {
    alias: {
      "@": resolve(__dirname, "src"),
      "cybermanju-drive-wasm": wasmTarget,
    },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
}));
