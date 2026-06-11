<template>
  <header class="topbar panel-top">
    <!-- Left: Logo -->
    <div class="topbar-left">
      <div class="logo-area">
        <span class="logo-text neon-text">Cybermanju</span>
        <span class="logo-text-secondary text-gold">Drive</span>
        <span class="mandala-dot" />
      </div>
    </div>

    <!-- Center: Search -->
    <div class="topbar-center">
      <div class="search-wrapper" :class="{ 'search-active': store.isSearching }">
        <Search class="search-icon" :size="14" />
        <input
          v-model="store.searchQuery"
          class="search-input cyber-input"
          type="text"
          placeholder="Search files with Tantivy..."
          @keyup.enter="handleSearch"
        />
        <div v-if="store.isSearching" class="search-spinner" />
      </div>
    </div>

    <!-- Right: Status indicators -->
    <div class="topbar-right">
      <!-- Encryption badge -->
      <button
        class="status-badge"
        :class="store.encryptionStatus.isEncrypted ? 'badge-active' : 'badge-inactive'"
        @click="store.showEncryptionPanel = !store.showEncryptionPanel"
        title="Toggle encryption panel"
      >
        <Shield v-if="store.encryptionStatus.isEncrypted" :size="13" class="icon-green" />
        <Unlock v-else :size="13" class="icon-red" />
        <span class="badge-label">{{ store.encryptionStatus.isEncrypted ? 'PQC' : 'OFF' }}</span>
      </button>

      <!-- Compression badge -->
      <div class="status-badge badge-neutral">
        <Layers :size="13" class="icon-cyan" />
        <span class="badge-label">{{ store.compressedFiles.length }}</span>
      </div>

      <!-- Account indicator -->
      <div v-if="store.activeAccount" class="status-badge badge-neutral">
        <User :size="13" class="text-secondary" />
        <span class="dot" :style="{ background: store.activeAccount.color, boxShadow: `0 0 6px ${store.activeAccount.color}` }" />
        <span class="badge-label">{{ store.activeAccount.name }}</span>
      </div>

      <!-- Matrix rain toggle -->
      <button
        class="status-badge matrix-toggle"
        :class="{ 'matrix-active': store.matrixRainEnabled }"
        @click="store.matrixRainEnabled = !store.matrixRainEnabled"
        title="Toggle matrix rain"
      >
        <Eye v-if="store.matrixRainEnabled" :size="14" class="icon-green" />
        <EyeOff v-else :size="14" class="text-muted" />
      </button>
    </div>
  </header>
</template>

<script setup lang="ts">
import { Search, Shield, Layers, User, Eye, EyeOff, Lock, Unlock, UserCheck, Globe } from 'lucide-vue-next'
import { useAppStore } from '@/stores/app'

const store = useAppStore()

function handleSearch() {
  if (store.searchQuery.trim()) {
    store.searchFiles(store.searchQuery)
  }
}
</script>

<style scoped>
.topbar {
  grid-area: topbar;
  display: flex;
  align-items: center;
  height: 56px;
  padding: 0 12px;
  gap: 16px;
  background: var(--cyber-bg-panel);
  border-bottom: 3px solid #000000;
  z-index: 10;
  position: relative;
}

.topbar-left {
  display: flex;
  align-items: center;
  min-width: 200px;
  flex-shrink: 0;
}

.logo-area {
  display: flex;
  align-items: baseline;
  gap: 2px;
  position: relative;
}

.logo-text {
  font-family: 'Courier New', monospace;
  font-size: 16px;
  font-weight: 800;
  letter-spacing: 1px;
  text-transform: uppercase;
}

.logo-text-secondary {
  font-family: 'Courier New', monospace;
  font-size: 14px;
  font-weight: 600;
  letter-spacing: 1px;
}

.mandala-dot {
  position: absolute;
  right: -14px;
  top: 50%;
  transform: translateY(-60%);
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--cyber-saffron-gold);
  box-shadow: 0 0 8px var(--cyber-saffron-gold), 0 0 16px rgba(255, 184, 0, 0.3);
  animation: pulse-glow 2.5s ease-in-out infinite;
}

.topbar-center {
  flex: 1;
  display: flex;
  justify-content: center;
  max-width: 480px;
  margin: 0 auto;
}

.search-wrapper {
  position: relative;
  display: flex;
  align-items: center;
  width: 100%;
}

.search-icon {
  position: absolute;
  left: 10px;
  color: var(--cyber-text-muted);
  pointer-events: none;
  z-index: 1;
}

.search-input {
  width: 100%;
  padding-left: 32px;
  padding-right: 32px;
  font-size: 12px;
  height: 32px;
}

.search-wrapper.search-active .search-input {
  border-color: var(--cyber-cyber-blue);
  box-shadow: 0 0 10px rgba(0, 212, 255, 0.2);
}

.search-spinner {
  position: absolute;
  right: 10px;
  width: 12px;
  height: 12px;
  border: 2px solid var(--cyber-bg-hover);
  border-top-color: var(--cyber-cyber-blue);
  border-radius: 50%;
  animation: spin-slow 0.6s linear infinite;
}

.topbar-right {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

.status-badge {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 4px 8px;
  background: var(--cyber-bg-card);
  border: 2px solid #000000;
  border-radius: 2px;
  cursor: pointer;
  transition: all 0.1s ease;
  font-size: 11px;
  font-family: monospace;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.3px;
}

.status-badge:hover {
  border-color: var(--cyber-bg-hover);
  background: var(--cyber-bg-hover);
}

.badge-active {
  border-color: rgba(0, 255, 65, 0.4);
}

.badge-inactive {
  border-color: rgba(220, 38, 38, 0.3);
}

.badge-neutral {
  cursor: default;
}

.badge-label {
  color: var(--cyber-text-secondary);
}

.icon-green {
  color: var(--cyber-matrix-green);
}

.icon-red {
  color: var(--cyber-prayer-red);
}

.icon-cyan {
  color: var(--cyber-cyber-blue);
}

.matrix-toggle {
  padding: 4px 6px;
}

.matrix-active {
  border-color: rgba(0, 255, 65, 0.3);
  box-shadow: 0 0 6px rgba(0, 255, 65, 0.15);
}

@keyframes pulse-glow {
  0%, 100% {
    box-shadow: 0 0 4px var(--cyber-saffron-gold), 0 0 8px rgba(255, 184, 0, 0.3);
  }
  50% {
    box-shadow: 0 0 8px var(--cyber-saffron-gold), 0 0 20px rgba(255, 184, 0, 0.4), 0 0 40px rgba(255, 184, 0, 0.15);
  }
}

@keyframes spin-slow {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>