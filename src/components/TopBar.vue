<template>
  <header class="topbar">
    <div class="topbar-left">
      <div class="logo">
        <span class="logo-text">CYBERMANJU</span>
        <span class="logo-sub">DRIVE</span>
      </div>
    </div>

    <div class="topbar-center">
      <div class="search-wrap" :class="{ searching: store.isSearching }">
        <span class="search-icon">&gt;</span>
        <input
          v-model="store.searchQuery"
          class="search-input"
          type="text"
          placeholder="SEARCH_WITH_TANTIVY_"
          @keyup.enter="handleSearch"
        />
        <span v-if="store.isSearching" class="search-cursor">_</span>
      </div>
    </div>

    <div class="topbar-right">
      <button
        class="status-badge"
        :class="store.encryptionStatus.isEncrypted ? 'on' : 'off'"
        @click="store.showEncryptionPanel = !store.showEncryptionPanel"
        title="TOGGLE ENCRYPTION PANEL"
      >
        <span class="badge-label">{{ store.encryptionStatus.isEncrypted ? 'PQC:ON' : 'PQC:OFF' }}</span>
      </button>

      <div class="status-badge neutral">
        <span class="badge-label">CMP:{{ store.compressedFiles.length }}</span>
      </div>

      <div v-if="store.activeAccount" class="status-badge neutral">
        <span class="badge-label">{{ store.activeAccount.name }}</span>
      </div>

      <button
        class="status-badge neutral matrix-btn"
        :class="{ on: store.matrixRainEnabled }"
        @click="store.matrixRainEnabled = !store.matrixRainEnabled"
        title="TOGGLE BACKGROUND ANIMATION"
      >
        <span class="badge-label">{{ store.matrixRainEnabled ? 'GFX:ON' : 'GFX:OFF' }}</span>
      </button>
    </div>
  </header>
</template>

<script setup lang="ts">
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
  display: flex;
  align-items: center;
  height: 48px;
  padding: 0 10px;
  gap: 12px;
  background: #000;
  border-bottom: 2px solid #FFFFFF;
  z-index: 10;
  position: relative;
}

.topbar-left {
  display: flex;
  align-items: center;
  min-width: 180px;
  flex-shrink: 0;
}

.logo {
  display: flex;
  align-items: baseline;
  gap: 4px;
}

.logo-text {
  font-family: 'Courier New', monospace;
  font-size: 15px;
  font-weight: 800;
  letter-spacing: 2px;
  color: #FFFFFF;
}

.logo-sub {
  font-family: 'Courier New', monospace;
  font-size: 13px;
  font-weight: 600;
  color: rgba(255,255,255,0.5);
  letter-spacing: 1px;
}

.topbar-center {
  flex: 1;
  display: flex;
  justify-content: center;
  max-width: 420px;
  margin: 0 auto;
}

.search-wrap {
  position: relative;
  display: flex;
  align-items: center;
  width: 100%;
  border: 2px solid #FFFFFF;
  background: #000;
  padding: 0 8px;
  height: 30px;
}

.search-icon {
  color: rgba(255,255,255,0.6);
  font-family: 'Courier New', monospace;
  font-size: 12px;
  margin-right: 6px;
}

.search-input {
  flex: 1;
  background: transparent;
  border: none;
  color: #FFFFFF;
  font-family: 'Courier New', monospace;
  font-size: 11px;
  height: 100%;
}

.search-input::placeholder {
  color: rgba(255,255,255,0.3);
}

.searching .search-input {
  color: #FFFFFF;
}

.search-cursor {
  color: #FFFFFF;
  animation: blink 0.8s step-end infinite;
  font-size: 12px;
}

@keyframes blink {
  50% { opacity: 0; }
}

.topbar-right {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
}

.status-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 3px 8px;
  border: 2px solid #FFFFFF;
  background: #000;
  color: #FFFFFF;
  cursor: pointer;
  font-family: 'Courier New', monospace;
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  transition: none;
}

.status-badge:hover {
  background: #FFFFFF;
  color: #000;
}

.status-badge.on {
  background: #FFFFFF;
  color: #000;
}

.status-badge.off {
  opacity: 0.6;
}

.status-badge.neutral {
  cursor: default;
}

.status-badge.neutral:hover {
  background: #000;
  color: #FFFFFF;
}

.badge-label {
  font-family: 'Courier New', monospace;
  font-size: 10px;
}
</style>
