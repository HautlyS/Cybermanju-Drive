<template>
  <header class="topbar">
    <div class="topbar-left">
      <div class="logo">
        <span class="logo-text">CYBERMANJU</span>
        <span class="logo-sub">DRIVE</span>
      </div>
    </div>

    <div class="topbar-center">
      <div class="search-wrap" :class="{ searching: store.isSearching }" role="search">
        <span class="search-icon">&gt;</span>
        <input
          v-model="store.searchQuery"
          class="search-input"
          type="text"
          placeholder="SEARCH_WITH_TANTIVY_"
          aria-label="SEARCH FILES WITH TANTIVY BM25"
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
        title="TOGGLE ENCRYPTION PANEL (CTRL+E)"
        aria-label="TOGGLE ENCRYPTION PANEL"
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
        aria-label="TOGGLE MATRIX RAIN ANIMATION"
      >
        <span class="badge-label">{{ store.matrixRainEnabled ? 'GFX:ON' : 'GFX:OFF' }}</span>
      </button>

      <button
        class="status-badge neutral"
        @click="store.commandPaletteOpen = true"
        title="COMMAND PALETTE (CTRL+K)"
        aria-label="OPEN COMMAND PALETTE"
      >
        <span class="badge-label">[K]</span>
      </button>

      <button
        class="status-badge neutral"
        @click="store.showLoginPopup = true"
        title="LOGIN / REGISTER"
        aria-label="OPEN LOGIN DIALOG"
      >
        <span class="badge-label">{{ store.currentUser ? store.currentUser.username : 'LOGIN' }}</span>
      </button>
    </div>
  </header>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useAppStore } from '@/stores/app'

const store = useAppStore()

let searchTimer: ReturnType<typeof setTimeout> | null = null

function handleSearch() {
  if (store.searchQuery.trim()) {
    store.searchFiles(store.searchQuery)
    store.currentPanel = 'search'
  }
}

watch(() => store.searchQuery, () => {
  if (searchTimer) clearTimeout(searchTimer)
  if (!store.searchQuery.trim()) return
  searchTimer = setTimeout(() => {
    if (store.searchQuery.trim()) {
      store.searchFiles(store.searchQuery)
    }
  }, 400)
})
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

@media (max-width: 768px) {
  .topbar {
    padding: 0 6px;
    gap: 6px;
  }
  .topbar-left {
    min-width: auto;
  }
  .logo-text {
    font-size: 12px;
  }
  .logo-sub {
    display: none;
  }
  .topbar-center {
    max-width: none;
    flex: 1;
  }
  .topbar-right .status-badge {
    padding: 2px 4px;
    font-size: 8px;
  }
  .badge-label {
    font-size: 8px;
  }
}
</style>
