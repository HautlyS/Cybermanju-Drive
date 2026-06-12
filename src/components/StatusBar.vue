<template>
  <footer class="statusbar">
    <div class="sb-left">
      <span class="sb-path">{{ store.currentPath }}</span>
    </div>

    <div class="sb-center">
      <span class="sb-item">{{ store.files.length }} FILES</span>
      <template v-if="store.selectedFile">
        <span class="sb-div">|</span>
        <span class="sb-item">SEL: {{ store.selectedFile.name }}</span>
      </template>

      <template v-if="store.selectedFile?.encrypted">
        <span class="sb-div">|</span>
        <span class="sb-badge">ENC {{ store.selectedFile.encryptionAlgorithm?.toUpperCase() || '' }}</span>
      </template>

      <template v-if="store.selectedFile?.compressionLayers && store.selectedFile.compressionLayers[0] && store.selectedFile.compressionLayers[0] !== 'none'">
        <span class="sb-div">|</span>
        <span class="sb-badge">{{ store.selectedFile.compressionLayers[0].toUpperCase() }}</span>
      </template>

      <template v-if="store.selectedFile?.hashBlake3">
        <span class="sb-div">|</span>
        <span class="sb-hash">B3:{{ store.selectedFile.hashBlake3.substring(0, 10) }}..</span>
      </template>
    </div>

    <div class="sb-right">
      <span class="sb-tech">TAURI V2 | REDB | RUSTPQ | TANTIVY</span>
    </div>
  </footer>
</template>

<script setup lang="ts">
import { useAppStore } from '@/stores/app'

const store = useAppStore()
</script>

<style scoped>
.statusbar {
  display: flex;
  align-items: center;
  height: 24px;
  padding: 0 8px;
  gap: 6px;
  background: #000;
  border-top: 2px solid #FFFFFF;
  font-size: 10px;
  overflow: hidden;
  z-index: 10;
  font-family: 'Courier New', monospace;
}

.sb-left {
  display: flex;
  align-items: center;
  flex-shrink: 0;
}

.sb-path {
  font-size: 10px;
  font-weight: 600;
  max-width: 240px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: rgba(255,255,255,0.7);
}

.sb-center {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

.sb-item {
  white-space: nowrap;
  color: rgba(255,255,255,0.6);
}

.sb-div {
  color: rgba(255,255,255,0.3);
}

.sb-badge {
  font-weight: 700;
  font-size: 9px;
  color: #FFFFFF;
  border: 1px solid #FFFFFF;
  padding: 0 4px;
}

.sb-hash {
  font-size: 9px;
  color: rgba(255,255,255,0.5);
}

.sb-right {
  margin-left: auto;
  display: flex;
  align-items: center;
}

.sb-tech {
  font-size: 9px;
  color: rgba(255,255,255,0.3);
  letter-spacing: 0.5px;
}
</style>
