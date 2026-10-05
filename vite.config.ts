import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { resolve } from "path";
import wasmStub from "./vite-plugin-wasm-stub";

const host = process.env.TAURI_DEV_HOST;

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
