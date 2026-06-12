<template>
  <footer class="statusbar panel-bottom">
    <!-- Left: Current path -->
    <div class="statusbar-left">
      <span class="status-path mono neon-text">{{ store.currentPath }}</span>
    </div>

    <!-- Center: File info -->
    <div class="statusbar-center">
      <span class="status-item text-secondary">
        {{ store.files.length }} files
      </span>

      <template v-if="store.selectedFile">
        <span class="status-divider">│</span>
        <span class="status-item text-secondary">
          Selected: {{ store.selectedFile.name }}
        </span>
      </template>
    </div>

    <!-- Encryption status -->
    <div v-if="store.selectedFile?.encrypted" class="status-section">
      <span class="status-divider">│</span>
      <span class="badge badge-green">
        {{ store.selectedFile.encryptionAlgorithm?.toUpperCase() || 'ENC' }}
      </span>
      <span v-if="encryptionNistLevel > 0" class="badge badge-cyan">
        NIST-L{{ encryptionNistLevel }}
      </span>
    </div>

    <!-- Compression status -->
    <div v-if="store.selectedFile?.compressionLayers && store.selectedFile.compressionLayers[0] && store.selectedFile.compressionLayers[0] !== 'none'" class="status-section">
      <span class="status-divider">│</span>
      <span class="badge badge-gold">
        {{ store.selectedFile.compressionLayers[0].toUpperCase() }}
      </span>
      <span v-if="store.selectedFile?.compressionLayers && store.selectedFile.compressionLayers[0]" class="status-item text-secondary">
        {{ store.selectedFile.compressionLayers.length }} layers
      </span>
    </div>

    <!-- BLAKE3 hash -->
    <div v-if="store.selectedFile?.hashBlake3" class="status-section">
      <span class="status-divider">│</span>
      <span class="status-hash mono text-muted">
        BLAKE3: {{ store.selectedFile.hashBlake3.substring(0, 16) }}…
      </span>
    </div>

    <!-- Right: Tech stack -->
    <div class="statusbar-right">
      <span class="status-tech mono text-muted">
        Tauri v2 • redb • rustpq • Tantivy
      </span>
    </div>
  </footer>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useAppStore } from '@/stores/app'
import { ENCRYPTION_INFO } from '@/types'
import type { EncryptionAlgo } from '@/types'

const store = useAppStore()

const encryptionNistLevel = computed(() => {
  if (!store.selectedFile?.encryptionAlgorithm) return 0
  return ENCRYPTION_INFO[store.selectedFile.encryptionAlgorithm as EncryptionAlgo]?.nistLevel ?? 0
})
</script>

<style scoped>
.statusbar {
  grid-area: statusbar;
  display: flex;
  align-items: center;
  height: 28px;
  padding: 0 10px;
  gap: 8px;
  background: var(--cyber-bg-deep);
  border-top: 2px solid #252540;
  font-size: 11px;
  overflow: hidden;
  z-index: 10;
}

.statusbar-left {
  display: flex;
  align-items: center;
  flex-shrink: 0;
}

.status-path {
  font-size: 11px;
  font-weight: 600;
  max-width: 300px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.statusbar-center {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

.status-section {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
}

.status-item {
  white-space: nowrap;
}

.status-divider {
  color: var(--cyber-bg-hover);
  font-size: 11px;
  flex-shrink: 0;
}

.status-hash {
  font-size: 10px;
  white-space: nowrap;
}

.statusbar-right {
  margin-left: auto;
  display: flex;
  align-items: center;
  flex-shrink: 0;
}

.status-tech {
  font-size: 10px;
  white-space: nowrap;
  letter-spacing: 0.3px;
}

.badge {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  padding: 0 5px;
  font-size: 9px;
  font-family: monospace;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.4px;
  border-radius: 2px;
  border: 1px solid;
  line-height: 18px;
}

.badge-green {
  color: var(--cyber-matrix-green);
  border-color: var(--cyber-matrix-green);
  background: rgba(0, 255, 65, 0.08);
}

.badge-cyan {
  color: var(--cyber-cyber-blue);
  border-color: var(--cyber-cyber-blue);
  background: rgba(0, 212, 255, 0.08);
}

.badge-gold {
  color: var(--cyber-saffron-gold);
  border-color: var(--cyber-saffron-gold);
  background: rgba(255, 184, 0, 0.08);
}

.mono {
  font-family: 'Courier New', monospace;
}

.neon-text {
  color: var(--cyber-matrix-green);
  text-shadow: 0 0 4px rgba(0, 255, 65, 0.4);
}

.text-secondary {
  color: var(--cyber-text-secondary);
}

.text-muted {
  color: var(--cyber-text-muted);
}
</style>