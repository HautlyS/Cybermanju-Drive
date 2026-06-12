<!-- Cybermanju Drive — Main Application Shell -->
<!-- Neobrutalism × Buddhist-Nepalese × Matrix × Cyberpunk -->
<script setup lang="ts">
import { ref, onMounted, watch, computed } from 'vue'
import { useAppStore } from '@/stores/app'
import { isWebMode } from '@/composables/useTauri'
import MatrixRain from '@/components/MatrixRain.vue'
import PrayerFlags from '@/components/PrayerFlags.vue'
import TopBar from '@/components/TopBar.vue'
import Sidebar from '@/components/Sidebar.vue'
import FileGrid from '@/components/FileGrid.vue'
import FilePreview from '@/components/FilePreview.vue'
import StatusBar from '@/components/StatusBar.vue'
import EncryptionPanel from '@/components/EncryptionPanel.vue'
import CompressionPanel from '@/components/CompressionPanel.vue'
import CollectionsPanel from '@/components/CollectionsPanel.vue'
import FaceGroupingPanel from '@/components/FaceGroupingPanel.vue'
import MapView from '@/components/MapView.vue'
import CodeIntelligencePanel from '@/components/CodeIntelligencePanel.vue'
import UserManagementPanel from '@/components/UserManagementPanel.vue'
import WebDashboardPanel from '@/components/WebDashboardPanel.vue'
import DashboardOverlay from '@/components/DashboardOverlay.vue'
import SyncPanel from '@/components/SyncPanel.vue'
import LandingPage from '@/components/LandingPage.vue'
import type { PanelType } from '@/types'

const store = useAppStore()

const showRightPanel = computed(() => {
  return store.selectedFileId !== null
    || store.showEncryptionPanel
    || store.showCompressionPanel
})

const rightPanelTitle = computed(() => {
  if (store.showEncryptionPanel) return 'Quantum Shield'
  if (store.showCompressionPanel) return 'Compression Engine'
  return 'File Preview'
})

onMounted(() => {
  store.initialize()
  // Show landing page as default in web/WASM mode
  if (isWebMode() && store.currentPanel === 'files') {
    store.currentPanel = 'landing'
  }
})
</script>

<template>
  <div class="cybermanju-shell">
    <!-- Matrix Rain Background -->
    <MatrixRain :enabled="store.matrixRainEnabled" :opacity="0.06" />

    <!-- Prayer Flags -->
    <PrayerFlags />

    <!-- Top Bar -->
    <TopBar />

    <!-- Main Content Area -->
    <div class="main-area">
      <!-- Left Sidebar -->
      <Sidebar />

      <!-- Center Content -->
      <main class="center-content">
        <!-- View mode toolbar -->
        <div class="view-toolbar">
          <div class="toolbar-left">
            <span class="breadcrumb" style="color: var(--text-muted)">
              <span style="color: var(--matrix-green)">/~</span>
              <template v-for="part in store.currentPath.split('/').filter(Boolean)" :key="part">
                <span class="bc-sep"> / </span>
                <span style="color: var(--text-primary)">{{ part }}</span>
              </template>
            </span>
          </div>
          <div class="toolbar-right">
            <button
              class="view-btn"
              :class="{ active: store.viewMode === 'grid' }"
              @click="store.viewMode = 'grid'"
              title="Grid view"
            >
              <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><rect x="1" y="1" width="6" height="6" rx="1"/><rect x="9" y="1" width="6" height="6" rx="1"/><rect x="1" y="9" width="6" height="6" rx="1"/><rect x="9" y="9" width="6" height="6" rx="1"/></svg>
            </button>
            <button
              class="view-btn"
              :class="{ active: store.viewMode === 'list' }"
              @click="store.viewMode = 'list'"
              title="List view"
            >
              <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><rect x="1" y="2" width="14" height="2.5" rx="0.5"/><rect x="1" y="6.75" width="14" height="2.5" rx="0.5"/><rect x="1" y="11.5" width="14" height="2.5" rx="0.5"/></svg>
            </button>
            <div class="toolbar-divider" />
            <span class="file-count" style="color: var(--text-muted); font-family: monospace; font-size: 12px;">
              {{ store.currentFolderFiles.length }} items
              <span v-if="store.encryptedFiles.length" style="color: var(--matrix-green); margin-left: 8px;">
                {{ store.encryptedFiles.length }} encrypted
              </span>
              <span v-if="store.compressedFiles.length" style="color: var(--cyber-blue); margin-left: 8px;">
                {{ store.compressedFiles.length }} compressed
              </span>
            </span>
          </div>
        </div>

        <!-- Panel router -->
        <div class="content-panels">
          <!-- Landing page (web/WASM mode default) -->
          <LandingPage v-if="store.currentPanel === 'landing'" @open-app="store.currentPanel = 'files'" />

          <!-- Default file grid -->
          <FileGrid v-else-if="store.currentPanel === 'files'" />

          <!-- Web Dashboard overlay -->
          <DashboardOverlay v-else-if="store.currentPanel === 'webdash'" @close="store.currentPanel = 'files'" />

          <!-- Search results -->
          <div v-else-if="store.currentPanel === 'search'" class="panel-search">
            <div class="neobrutalism-card" style="padding: 16px; margin-bottom: 16px;">
              <h3 style="color: var(--matrix-green); font-family: monospace; margin-bottom: 8px;">
                Tantivy Search Results
              </h3>
              <p style="color: var(--text-secondary); font-size: 13px;">
                "{{ store.searchQuery }}" — {{ store.searchResults.length }} results
              </p>
            </div>
            <div v-if="store.searchResults.length" class="search-results-list">
              <div
                v-for="result in store.searchResults"
                :key="result.fileId"
                class="neobrutalism-card search-result-item"
                @click="store.selectFile(result.fileId)"
              >
                <div class="search-match-type" style="background: var(--saffron-gold)">
                  search
                </div>
                <div class="search-result-body">
                  <div class="search-result-name">{{ result.fileName }}</div>
                  <div class="search-result-snippet">{{ result.snippet }}</div>
                </div>
                <div class="search-result-score">{{ result.score.toFixed(3) }}</div>
              </div>
            </div>
            <div v-else class="empty-state">
              <p style="color: var(--text-muted);">Type in the search bar to search with Tantivy (BM25 + fuzzy + faceted)</p>
            </div>
          </div>

          <!-- Collections panel -->
          <CollectionsPanel v-else-if="store.currentPanel === 'collections'" />

          <!-- Face grouping panel -->
          <FaceGroupingPanel v-else-if="store.currentPanel === 'faces'" />

          <!-- Map view -->
          <MapView v-else-if="store.currentPanel === 'map'" />

          <!-- Code intelligence -->
          <CodeIntelligencePanel v-else-if="store.currentPanel === 'code'" />

          <!-- User Management panel -->
          <UserManagementPanel v-else-if="store.currentPanel === 'users'" />

          <!-- Web Dashboard panel -->
          <WebDashboardPanel v-else-if="store.currentPanel === 'dashboard'" />

          <!-- Sync panel -->
          <SyncPanel v-else-if="store.currentPanel === 'sync'" />

          <!-- Accounts panel -->
          <div v-else-if="store.currentPanel === 'accounts'" class="panel-accounts">
            <div class="neobrutalism-card" style="padding: 20px;">
              <h3 style="color: var(--saffron-gold); font-family: monospace; margin-bottom: 16px;">Multi-Account Manager</h3>
              <div class="accounts-list">
                <div
                  v-for="account in store.accounts"
                  :key="account.id"
                  class="account-item neobrutalism-card"
                  :class="{ active: account.isActive }"
                  @click="store.switchAccount(account.id)"
                >
                  <div class="account-dot" :style="{ background: account.color }" />
                  <div class="account-info">
                    <div class="account-name">{{ account.name }}</div>
                    <div class="account-meta">
                      <span class="account-type">{{ account.accountType }}</span>
                      <span v-if="account.path" class="account-path">{{ account.path }}</span>
                    </div>
                  </div>
                  <div v-if="account.isActive" class="active-badge">ACTIVE</div>
                </div>
              </div>
            </div>
          </div>

          <!-- Loose groups panel -->
          <div v-else-if="store.currentPanel === 'loose-groups'" class="panel-loose">
            <div class="neobrutalism-card" style="padding: 20px;">
              <h3 style="color: var(--cyber-blue); font-family: monospace; margin-bottom: 16px;">Loose File Grouping</h3>
              <div class="loose-groups-list">
                <div
                  v-for="group in store.looseGroups"
                  :key="group.id"
                  class="loose-group-item neobrutalism-card"
                >
                  <div class="lg-color-bar" :style="{ background: group.color }" />
                  <div class="lg-icon">{{ group.icon === 'crab' ? '🦀' : '🏔' }}</div>
                  <div class="lg-info">
                    <div class="lg-name">{{ group.name }}</div>
                    <div class="lg-count">{{ group.fileIds.length }} files</div>
                  </div>
                </div>
              </div>
              <div class="loose-empty" style="margin-top: 16px;">
                <p style="color: var(--text-muted); font-size: 13px;">
                  Group any files together regardless of folder structure. Drag & drop files into groups.
                </p>
              </div>
            </div>
          </div>

          <!-- Style-based organization -->
          <div v-else-if="store.currentPanel === 'style'" class="panel-style">
            <div class="neobrutalism-card" style="padding: 20px;">
              <h3 style="color: var(--lotus-pink); font-family: monospace; margin-bottom: 16px;">Style-Based Organization</h3>
              <p style="color: var(--text-secondary); font-size: 13px; margin-bottom: 16px;">
                Files organized by visual style classification (CLIP model via candle crate)
              </p>
              <div class="style-tags">
                <span
                  v-for="tag in [...new Set(store.files.flatMap(f => f.tags || []))]"
                  :key="tag"
                  class="style-tag neobrutalism-btn"
                  @click="store.searchQuery = tag; store.currentPanel = 'search'; store.searchFiles(tag)"
                >
                  {{ tag }}
                </span>
              </div>
            </div>
          </div>
        </div>
      </main>

      <!-- Right Panel -->
      <aside v-if="showRightPanel" class="right-panel">
        <EncryptionPanel v-if="store.showEncryptionPanel" @close="store.showEncryptionPanel = false" />
        <CompressionPanel v-else-if="store.showCompressionPanel" @close="store.showCompressionPanel = false" />
        <FilePreview v-else />
      </aside>
    </div>

    <!-- Status Bar -->
    <StatusBar />
  </div>
</template>

<style scoped>
.cybermanju-shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
  width: 100vw;
  background: var(--bg-deep);
  color: var(--text-primary);
  overflow: hidden;
  position: relative;
}

.main-area {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.center-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-width: 0;
}

.view-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 16px;
  border-bottom: 2px solid var(--bg-hover);
  background: var(--bg-panel);
  min-height: 40px;
}

.toolbar-left {
  display: flex;
  align-items: center;
  gap: 8px;
}

.toolbar-right {
  display: flex;
  align-items: center;
  gap: 6px;
}

.toolbar-divider {
  width: 1px;
  height: 16px;
  background: var(--text-muted);
  opacity: 0.3;
}

.view-btn {
  background: transparent;
  border: 2px solid transparent;
  color: var(--text-muted);
  padding: 4px 8px;
  cursor: pointer;
  display: flex;
  align-items: center;
  transition: all 0.15s;
}

.view-btn:hover {
  color: var(--text-primary);
  border-color: var(--bg-hover);
}

.view-btn.active {
  color: var(--matrix-green);
  border-color: var(--matrix-green);
  box-shadow: 0 0 8px rgba(0, 255, 65, 0.3);
}

.bc-sep {
  color: var(--text-muted);
  opacity: 0.5;
}

.content-panels {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
}

.right-panel {
  width: 380px;
  min-width: 380px;
  border-left: 3px solid #000;
  background: var(--bg-panel);
  overflow-y: auto;
  overflow-x: hidden;
}

/* Search results */
.search-results-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 0 16px 16px;
}

.search-result-item {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 12px !important;
  cursor: pointer;
  transition: transform 0.1s, box-shadow 0.1s;
}

.search-result-item:hover {
  transform: translate(2px, 2px);
  box-shadow: 2px 2px 0 #000 !important;
}

.search-match-type {
  font-size: 10px;
  font-weight: 700;
  padding: 2px 6px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  white-space: nowrap;
  border: 2px solid #000;
}

.search-result-body {
  flex: 1;
  min-width: 0;
}

.search-result-name {
  font-family: monospace;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 4px;
}

.search-result-snippet {
  font-size: 12px;
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.search-result-score {
  font-family: monospace;
  font-size: 11px;
  color: var(--saffron-gold);
  white-space: nowrap;
}

/* Accounts panel */
.accounts-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.account-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px !important;
  cursor: pointer;
  transition: transform 0.1s, box-shadow 0.1s;
}

.account-item:hover {
  transform: translate(2px, 2px);
  box-shadow: 2px 2px 0 #000 !important;
}

.account-item.active {
  border-color: var(--matrix-green) !important;
}

.account-dot {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  border: 2px solid #000;
  flex-shrink: 0;
}

.account-info {
  flex: 1;
  min-width: 0;
}

.account-name {
  font-weight: 600;
  font-size: 14px;
}

.account-meta {
  display: flex;
  gap: 8px;
  font-size: 12px;
  color: var(--text-secondary);
  margin-top: 2px;
}

.account-type {
  text-transform: uppercase;
  font-size: 10px;
  padding: 1px 4px;
  border: 1px solid var(--text-muted);
}

.account-path {
  font-family: monospace;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.active-badge {
  font-size: 10px;
  font-weight: 700;
  padding: 2px 8px;
  background: var(--matrix-green);
  color: #000;
  border: 2px solid #000;
  letter-spacing: 0.5px;
}

/* Loose groups */
.loose-groups-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.loose-group-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px !important;
  overflow: hidden;
}

.lg-color-bar {
  width: 4px;
  height: 36px;
  border: 1px solid #000;
  flex-shrink: 0;
}

.lg-icon {
  font-size: 24px;
  flex-shrink: 0;
}

.lg-info {
  flex: 1;
  min-width: 0;
}

.lg-name {
  font-weight: 600;
}

.lg-count {
  font-size: 12px;
  color: var(--text-secondary);
  font-family: monospace;
}

/* Style tags */
.style-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.style-tag {
  font-family: monospace;
  font-size: 13px;
  padding: 6px 12px !important;
  cursor: pointer;
  transition: transform 0.1s, box-shadow 0.1s;
}

.style-tag:hover {
  transform: translate(2px, 2px);
}

.empty-state {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 200px;
}

/* Panel transitions */
.panel-search,
.panel-accounts,
.panel-loose,
.panel-style {
  padding: 16px;
}

/* Scrollbar */
.content-panels::-webkit-scrollbar,
.right-panel::-webkit-scrollbar {
  width: 6px;
}

.content-panels::-webkit-scrollbar-track,
.right-panel::-webkit-scrollbar-track {
  background: var(--bg-deep);
}

.content-panels::-webkit-scrollbar-thumb,
.right-panel::-webkit-scrollbar-thumb {
  background: var(--matrix-dark-green);
  border: 1px solid #000;
}
</style>