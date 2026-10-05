import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { resolve } from "path";

// Determine the base path:
// - GitHub Pages: /cybermanju-drive/ (lowercase repo slug)
// - Docker / standalone: / (served from root)
// Override with VITE_BASE env var if needed
const base =
  process.env.VITE_BASE ||
  (process.env.NODE_ENV === "production" &&
  process.env.DOCKER_BUILD !== "true"
    ? "/cybermanju-drive/"
    : "/");

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      "@": resolve(__dirname, "src"),
      // wasm-pack output (`wasm-pack build crates/drive-wasm --target web
      // --out-dir crates/drive-wasm/pkg`) — the integrated backend for the
      // static/GH-Pages bundle.
      "cybermanju-drive-wasm": resolve(__dirname, "crates/drive-wasm/pkg"),
    },
  },
  base,
  build: {
    outDir: "dist-wasm",
    emptyOutDir: true,
    // Chunk splitting for better caching in production
    rollupOptions: {
      output: {
        manualChunks: {
          "vendor-vue": ["vue", "pinia"],
          "vendor-map": ["maplibre-gl"],
          "vendor-icons": ["lucide-vue-next"],
        },
      },
    },
    // Source maps for debugging (disabled in production for smaller bundles)
    sourcemap: process.env.NODE_ENV !== "production",
    // Minification settings
    minify: "esbuild",
    // Chunk size warning threshold (500KB)
    chunkSizeWarningLimit: 500,
  },
  // Ensure Tauri APIs are stubbed out for web/WASM builds
  define: {
    __TAURI__: "false",
    "window.__TAURI__": "false",
    "import.meta.env.TAURI": "false",
    "import.meta.env.VITE_TAURI": "false",
  },
  css: {
    devSourcemap: true,
  },
  // Development server for local WASM testing
  server: {
    port: 4174,
    strictPort: false,
    open: false,
  },
  // Preview server configuration
  preview: {
    port: 4175,
  },
});