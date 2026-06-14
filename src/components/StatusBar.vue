<template>
  <footer class="statusbar">
    <div class="sb-left">
      <span class="sb-path">{{ store.currentPath }}</span>
      <div v-if="store.isLoading" class="sb-progress-bar">
        <div class="sb-progress-fill" />
      </div>
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
      <span
        class="sb-clickable sync-icon"
        :class="{ 'sb-active': isSyncActive }"
        title="SYNC STATUS"
        aria-label="SYNC STATUS"
      >{{ isSyncActive ? 'SYNC:' + store.syncProgress?.status.toUpperCase() : 'SYNC:IDLE' }}</span>
      <span class="sb-div">|</span>
      <span
        class="sb-clickable"
        :class="{ 'sb-active': store.matrixRainEnabled }"
        @click="store.matrixRainEnabled = !store.matrixRainEnabled"
        title="TOGGLE MATRIX RAIN"
        aria-label="TOGGLE MATRIX RAIN BACKGROUND"
      >{{ store.matrixRainEnabled ? 'GFX:ON' : 'GFX:OFF' }}</span>
      <span class="sb-div">|</span>
      <span class="sb-clickable" @click="store.commandPaletteOpen = true" title="COMMAND PALETTE (CTRL+K)" aria-label="OPEN COMMAND PALETTE">CMD+K</span>
      <span class="sb-div">|</span>
      <span class="sb-tech">{{ isWebMode() ? 'WEB MODE' : 'TAURI MODE' }}</span>
    </div>
  </footer>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useAppStore } from '@/stores/app'
import { isWebMode } from '@/composables/useTauri'

const store = useAppStore()
const isSyncActive = computed(() =>
  store.syncProgress !== null && store.syncProgress.status !== 'idle' && store.syncProgress.status !== 'done'
)
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

.sb-progress-bar {
  width: 60px;
  height: 6px;
  border: 1px solid #FFFFFF;
  margin-left: 8px;
  overflow: hidden;
}

.sb-progress-fill {
  height: 100%;
  width: 30%;
  background: #FFFFFF;
  animation: sb-progress 1.2s ease-in-out infinite;
}

@keyframes sb-progress {
  0% { transform: translateX(-100%); }
  100% { transform: translateX(calc(60px * 3.33)); }
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

.sb-clickable {
  cursor: pointer;
  color: rgba(255,255,255,0.5);
  font-size: 9px;
}

.sb-clickable:hover {
  color: #FFFFFF;
  text-decoration: underline;
}

.sb-active {
  color: #FFFFFF;
  font-weight: 700;
}

@media (max-width: 768px) {
  .sb-center {
    display: none;
  }
  .sb-path {
    max-width: 120px;
  }
}
</style>
