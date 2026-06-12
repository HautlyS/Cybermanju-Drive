<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useAppStore } from '@/stores/app'
import { CYBER } from '@/types'
import { Search, Folder, Map, Settings, Shield, Menu } from 'lucide-vue-next'

const store = useAppStore()
const emit = defineEmits<{ close: [] }>()

const searchInput = ref('')
const mobileMenuOpen = ref(false)
const activeTab = ref<'files' | 'search' | 'map' | 'settings'>('files')

const filesCount = computed(() => store.files.length)
const accountName = computed(() => {
  const active = store.accounts.find(a => a.isActive)
  return active?.name ?? 'No Account'
})

let searchTimeout: ReturnType<typeof setTimeout> | null = null

function onSearchInput() {
  if (searchTimeout) clearTimeout(searchTimeout)
  searchTimeout = setTimeout(() => {
    if (searchInput.value.trim()) {
      store.searchFiles(searchInput.value)
    }
  }, 300)
}

watch(() => store.searchQuery, (q) => {
  searchInput.value = q
})
</script>

<template>
  <div class="overlay-root">
    <!-- ═══════════ TOP BAR ═══════════ -->
    <header class="topbar">
      <button class="menu-btn" @click="mobileMenuOpen = !mobileMenuOpen">
        <Menu :size="22" />
      </button>

      <div class="topbar-title">
        <Shield :size="18" class="title-icon" />
        <h1 class="title-text">Cybermanju Drive</h1>
        <span class="title-badge">Web</span>
      </div>

      <div class="topbar-account">
        <span class="account-dot" />
        <span class="account-name">{{ accountName }}</span>
      </div>
    </header>

    <!-- ═══════════ SEARCH BAR (always visible on search tab) ═══════════ -->
    <div v-if="activeTab === 'search'" class="search-bar">
      <Search :size="18" class="search-icon" />
      <input
        v-model="searchInput"
        class="search-input"
        placeholder="Search files, content, tags..."
        autofocus
        spellcheck="false"
        @input="onSearchInput"
      />
      <span v-if="searchInput" class="result-count">{{ store.searchResults.length }} results</span>
    </div>

    <!-- ═══════════ CONTENT AREA ═══════════ -->
    <main class="content">
      <!-- FILES TAB -->
      <template v-if="activeTab === 'files'">
        <div v-if="filesCount === 0" class="empty-state">
          <Folder :size="48" class="empty-icon" />
          <p>No files indexed yet</p>
        </div>

        <div v-else class="file-grid">
          <div
            v-for="file in store.files"
            :key="file.id"
            class="file-card"
            @click="store.selectFile(file.id)"
          >
            <div class="file-icon-wrap" :style="{ borderColor: file.encrypted ? CYBER.matrixGreen : '#333' }">
              <Folder v-if="file.fileType === 'folder'" :size="28" />
              <span v-else class="file-ext">{{ file.name.split('.').pop()?.slice(0, 4) ?? '?' }}</span>
            </div>
            <div class="file-meta">
              <span class="file-name">{{ file.name }}</span>
              <span class="file-size">{{ (file.sizeBytes / 1024).toFixed(1) }} KB</span>
            </div>
          </div>
        </div>
      </template>

      <!-- SEARCH TAB -->
      <template v-if="activeTab === 'search'">
        <div v-if="!searchInput.trim()" class="empty-state">
          <Search :size="48" class="empty-icon" />
          <p>Type to search your files</p>
        </div>

        <div v-else-if="store.searchResults.length === 0" class="empty-state">
          <Search :size="48" class="empty-icon" />
          <p>No results for "{{ searchInput }}"</p>
        </div>

        <div v-else class="search-results">
          <div v-for="result in store.searchResults" :key="result.fileId" class="result-card" @click="store.selectFile(result.fileId)">
            <div class="result-icon">
              <span class="result-ext">{{ result.matchType }}</span>
            </div>
            <div class="result-info">
              <span class="result-name">{{ result.fileName }}</span>
              <span class="result-path mono">{{ result.snippet }}</span>
            </div>
            <span class="result-score">{{ result.score.toFixed(3) }}</span>
          </div>
        </div>
      </template>

      <!-- MAP TAB -->
      <template v-if="activeTab === 'map'">
        <div class="empty-state">
          <Map :size="48" class="empty-icon" />
          <p>Geo map view</p>
          <span class="subtle">Requires location data in files</span>
        </div>
      </template>

      <!-- SETTINGS TAB -->
      <template v-if="activeTab === 'settings'">
        <div class="settings-list">
          <div class="setting-item">
            <span class="setting-label">Theme</span>
            <span class="setting-val">Dark (Cybermanju)</span>
          </div>
          <div class="setting-item">
            <span class="setting-label">Version</span>
            <span class="setting-val mono">0.1.0</span>
          </div>
          <div class="setting-item">
            <span class="setting-label">Files Indexed</span>
            <span class="setting-val mono">{{ filesCount }}</span>
          </div>
          <div class="setting-item">
            <span class="setting-label">Protocol</span>
            <span class="setting-val">HTTP (local)</span>
          </div>
        </div>
      </template>
    </main>

    <!-- ═══════════ BOTTOM NAV ═══════════ -->
    <nav class="bottom-nav">
      <button
        v-for="tab in ([
          { key: 'files', icon: Folder, label: 'Files' },
          { key: 'search', icon: Search, label: 'Search' },
          { key: 'map', icon: Map, label: 'Map' },
          { key: 'settings', icon: Settings, label: 'Settings' },
        ] as const)"
        :key="tab.key"
        :class="['nav-btn', { active: activeTab === tab.key }]"
        @click="activeTab = tab.key"
      >
        <component :is="tab.icon" :size="20" />
        <span>{{ tab.label }}</span>
      </button>
    </nav>
  </div>
</template>

<style scoped>
.overlay-root {
  display: flex;
  flex-direction: column;
  height: 100vh;
  height: 100dvh;
  background: #0a0a0f;
  color: #F5F5F4;
  font-family: system-ui, -apple-system, sans-serif;
  overflow: hidden;
  -webkit-font-smoothing: antialiased;
}

/* ── Helpers ── */
.mono { font-family: 'Courier New', monospace; }
.subtle { color: #6B7280; font-size: 12px; }

/* ═══════════ TOP BAR ═══════════ */
.topbar {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  background: #12121a;
  border-bottom: 3px solid #000;
  box-shadow: 0 4px 0 #000;
  flex-shrink: 0;
  z-index: 10;
}
.menu-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: 2px solid #333;
  color: #F5F5F4;
  padding: 6px;
  cursor: pointer;
  border-radius: 2px;
}
.topbar-title {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
}
.title-icon {
  color: #00FF41;
}
.title-text {
  font-size: 16px;
  font-weight: 800;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: #00FF41;
}
.title-badge {
  font-family: 'Courier New', monospace;
  font-size: 10px;
  font-weight: 700;
  color: #00D4FF;
  padding: 1px 6px;
  border: 2px solid #00D4FF;
  text-transform: uppercase;
  letter-spacing: 1px;
}
.topbar-account {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: #9CA3AF;
}
.account-dot {
  width: 8px;
  height: 8px;
  background: #00FF41;
  border: 1px solid #000;
  border-radius: 50%;
}
.account-name {
  font-family: 'Courier New', monospace;
  max-width: 100px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* ═══════════ SEARCH BAR ═══════════ */
.search-bar {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 16px;
  background: #12121a;
  border-bottom: 3px solid #000;
  flex-shrink: 0;
}
.search-icon {
  color: #9CA3AF;
  flex-shrink: 0;
}
.search-input {
  flex: 1;
  background: #1a1a2e;
  border: 2px solid #000;
  color: #F5F5F4;
  padding: 8px 12px;
  font-family: 'Courier New', monospace;
  font-size: 14px;
  outline: none;
  box-shadow: 2px 2px 0 #000;
}
.search-input:focus {
  border-color: #00FF41;
  box-shadow: 2px 2px 0 #00FF41;
}
.search-input::placeholder {
  color: #4B5563;
}
.result-count {
  font-family: 'Courier New', monospace;
  font-size: 11px;
  color: #00FF41;
  white-space: nowrap;
  flex-shrink: 0;
}

/* ═══════════ CONTENT ═══════════ */
.content {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
  -webkit-overflow-scrolling: touch;
}

/* ── Empty state ── */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 60px 20px;
  text-align: center;
  color: #6B7280;
}
.empty-icon {
  opacity: 0.3;
}
.empty-state p {
  font-size: 14px;
  margin: 0;
}

/* ── File grid ── */
.file-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
  gap: 12px;
}
.file-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 16px 10px 12px;
  background: #1a1a2e;
  border: 3px solid #000;
  box-shadow: 3px 3px 0 #000;
  cursor: pointer;
  transition: transform 0.1s, box-shadow 0.1s;
  text-align: center;
}
.file-card:hover {
  transform: translate(1px, 1px);
  box-shadow: 2px 2px 0 #000;
}
.file-card:active {
  transform: translate(2px, 2px);
  box-shadow: 1px 1px 0 #000;
}
.file-icon-wrap {
  width: 56px;
  height: 56px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 3px solid #333;
  background: #12121a;
  color: #9CA3AF;
}
.file-ext {
  font-family: 'Courier New', monospace;
  font-size: 11px;
  font-weight: 800;
  text-transform: uppercase;
  color: #00D4FF;
}
.file-meta {
  display: flex;
  flex-direction: column;
  gap: 2px;
  width: 100%;
}
.file-name {
  font-size: 12px;
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: #F5F5F4;
}
.file-size {
  font-family: 'Courier New', monospace;
  font-size: 11px;
  color: #6B7280;
}

/* ── Search results ── */
.search-results {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.result-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 14px;
  background: #1a1a2e;
  border: 3px solid #000;
  box-shadow: 3px 3px 0 #000;
  cursor: pointer;
  transition: transform 0.1s, box-shadow 0.1s;
}
.result-card:hover {
  transform: translate(1px, 1px);
  box-shadow: 2px 2px 0 #000;
}
.result-icon {
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 2px solid #333;
  background: #12121a;
  color: #9CA3AF;
  flex-shrink: 0;
}
.result-ext {
  font-family: 'Courier New', monospace;
  font-size: 10px;
  font-weight: 800;
  text-transform: uppercase;
  color: #00D4FF;
}
.result-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}
.result-name {
  font-size: 13px;
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.result-path {
  font-size: 11px;
  color: #6B7280;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.result-score {
  font-family: 'Courier New', monospace;
  font-size: 11px;
  color: #FFB800;
  white-space: nowrap;
  flex-shrink: 0;
}

/* ── Settings ── */
.settings-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.setting-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 16px;
  background: #1a1a2e;
  border: 3px solid #000;
  box-shadow: 3px 3px 0 #000;
}
.setting-label {
  font-size: 14px;
  font-weight: 600;
  color: #F5F5F4;
}
.setting-val {
  font-family: 'Courier New', monospace;
  font-size: 13px;
  color: #00D4FF;
}

/* ═══════════ BOTTOM NAV ═══════════ */
.bottom-nav {
  display: flex;
  background: #12121a;
  border-top: 3px solid #000;
  box-shadow: 0 -4px 0 #000;
  flex-shrink: 0;
  padding-bottom: env(safe-area-inset-bottom, 0px);
}
.nav-btn {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 10px 4px;
  background: transparent;
  border: none;
  border-right: 1px solid #1a1a2e;
  color: #6B7280;
  font-family: system-ui, sans-serif;
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  cursor: pointer;
  transition: color 0.15s, background 0.15s;
}
.nav-btn:last-child {
  border-right: none;
}
.nav-btn.active {
  color: #00FF41;
  background: #1a1a2e;
}
.nav-btn.active svg {
  filter: drop-shadow(0 0 4px rgba(0, 255, 65, 0.5));
}

/* ── Scrollbar ── */
.content::-webkit-scrollbar { width: 6px; }
.content::-webkit-scrollbar-track { background: #0a0a0f; }
.content::-webkit-scrollbar-thumb { background: #1a1a2e; border: 1px solid #000; }

/* ── Responsive ── */
@media (min-width: 768px) {
  .file-grid {
    grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
  }
  .overlay-root {
    max-width: 900px;
    margin: 0 auto;
    border-left: 3px solid #000;
    border-right: 3px solid #000;
  }
}
</style>