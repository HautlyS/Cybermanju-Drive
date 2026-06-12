<script setup lang="ts">
import { ref, onMounted, watch, computed } from 'vue'
import { useAppStore } from '@/stores/app'
import { isWebMode } from '@/composables/useTauri'
import CanvasEngine from '@/components/CanvasEngine.vue'
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
  if (store.showEncryptionPanel) return 'QUANTUM SHIELD'
  if (store.showCompressionPanel) return 'COMPRESSION ENGINE'
  return 'FILE PREVIEW'
})

onMounted(() => {
  store.initialize()
  if (isWebMode() && store.currentPanel === 'files') {
    store.currentPanel = 'landing'
  }
})
</script>

<template>
  <div class="cybermanju-shell">
    <CanvasEngine :enabled="store.matrixRainEnabled" />

    <TopBar />

    <div class="main-area">
      <Sidebar />

      <main class="center-content">
        <div class="view-toolbar">
          <div class="toolbar-left">
            <span class="breadcrumb">
              <span class="bc-root">/~</span>
              <template v-for="part in store.currentPath.split('/').filter(Boolean)" :key="part">
                <span class="bc-sep">/</span>
                <span class="bc-part">{{ part }}</span>
              </template>
            </span>
          </div>
          <div class="toolbar-right">
            <button
              class="view-btn"
              :class="{ active: store.viewMode === 'grid' }"
              @click="store.viewMode = 'grid'"
              title="GRID"
            >
              <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><rect x="1" y="1" width="6" height="6"/><rect x="9" y="1" width="6" height="6"/><rect x="1" y="9" width="6" height="6"/><rect x="9" y="9" width="6" height="6"/></svg>
            </button>
            <button
              class="view-btn"
              :class="{ active: store.viewMode === 'list' }"
              @click="store.viewMode = 'list'"
              title="LIST"
            >
              <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><rect x="1" y="2" width="14" height="2.5"/><rect x="1" y="6.75" width="14" height="2.5"/><rect x="1" y="11.5" width="14" height="2.5"/></svg>
            </button>
            <div class="tb-divider" />
            <span class="file-count">
              {{ store.currentFolderFiles.length }} ITEMS
              <span v-if="store.encryptedFiles.length" class="count-badge">{{ store.encryptedFiles.length }} ENC</span>
              <span v-if="store.compressedFiles.length" class="count-badge">{{ store.compressedFiles.length }} CMP</span>
            </span>
          </div>
        </div>

        <div class="content-panels">
          <LandingPage v-if="store.currentPanel === 'landing'" @open-app="store.currentPanel = 'files'" />
          <FileGrid v-else-if="store.currentPanel === 'files'" />
          <DashboardOverlay v-else-if="store.currentPanel === 'webdash'" @close="store.currentPanel = 'files'" />
          <div v-else-if="store.currentPanel === 'search'" class="panel-search">
            <div class="bw-card" style="padding: 12px; margin-bottom: 12px;">
              <div class="bw-title">TANTIVY SEARCH</div>
              <p class="text-muted">"{{ store.searchQuery }}" - {{ store.searchResults.length }} RESULTS</p>
            </div>
            <div v-if="store.searchResults.length" class="search-results-list">
              <div
                v-for="result in store.searchResults"
                :key="result.fileId"
                class="search-result-item bw-card"
                @click="store.selectFile(result.fileId)"
              >
                <div class="search-match-type">SEARCH</div>
                <div class="search-result-body">
                  <div class="search-result-name">{{ result.fileName }}</div>
                  <div class="search-result-snippet text-muted">{{ result.snippet }}</div>
                </div>
                <div class="search-result-score">{{ result.score.toFixed(3) }}</div>
              </div>
            </div>
            <div v-else class="empty-state">
              <p class="text-muted">TYPE IN SEARCH BAR FOR TANTIVY BM25 SEARCH</p>
            </div>
          </div>
          <CollectionsPanel v-else-if="store.currentPanel === 'collections'" />
          <FaceGroupingPanel v-else-if="store.currentPanel === 'faces'" />
          <MapView v-else-if="store.currentPanel === 'map'" />
          <CodeIntelligencePanel v-else-if="store.currentPanel === 'code'" />
          <UserManagementPanel v-else-if="store.currentPanel === 'users'" />
          <WebDashboardPanel v-else-if="store.currentPanel === 'dashboard'" />
          <SyncPanel v-else-if="store.currentPanel === 'sync'" />
          <div v-else-if="store.currentPanel === 'accounts'" class="panel-page">
            <div class="bw-card" style="padding: 16px;">
              <div class="bw-title">MULTI-ACCOUNT MANAGER</div>
              <div class="accounts-list">
                <div
                  v-for="account in store.accounts"
                  :key="account.id"
                  class="account-item bw-card"
                  :class="{ active: account.isActive }"
                  @click="store.switchAccount(account.id)"
                >
                  <div class="bw-dot" :class="{ 'bw-dot-on': account.isActive }" />
                  <div class="account-info">
                    <div class="account-name">{{ account.name }}</div>
                    <div class="account-meta text-muted">{{ account.accountType }} {{ account.path }}</div>
                  </div>
                  <div v-if="account.isActive" class="active-badge">ACTIVE</div>
                </div>
              </div>
            </div>
          </div>
          <div v-else-if="store.currentPanel === 'loose-groups'" class="panel-page">
            <div class="bw-card" style="padding: 16px;">
              <div class="bw-title">LOOSE FILE GROUPING</div>
              <div class="loose-groups-list">
                <div v-for="group in store.looseGroups" :key="group.id" class="loose-group-item bw-card">
                  <div class="lg-info">
                    <div class="lg-name">{{ group.name }}</div>
                    <div class="lg-count text-muted">{{ group.fileIds.length }} FILES</div>
                  </div>
                </div>
              </div>
            </div>
          </div>
          <div v-else-if="store.currentPanel === 'style'" class="panel-page">
            <div class="bw-card" style="padding: 16px;">
              <div class="bw-title">STYLE-BASED ORGANIZATION</div>
              <p class="text-muted" style="margin-bottom: 12px;">FILES ORGANIZED BY VISUAL STYLE (CLIP MODEL)</p>
              <div class="style-tags">
                <span v-for="tag in [...new Set(store.files.flatMap(f => f.tags || []))]" :key="tag" class="bw-btn" style="padding: 4px 8px;" @click="store.searchQuery = tag; store.currentPanel = 'search'; store.searchFiles(tag)">
                  {{ tag }}
                </span>
              </div>
            </div>
          </div>
        </div>
      </main>

      <aside v-if="showRightPanel" class="right-panel">
        <EncryptionPanel v-if="store.showEncryptionPanel" @close="store.showEncryptionPanel = false" />
        <CompressionPanel v-else-if="store.showCompressionPanel" @close="store.showCompressionPanel = false" />
        <FilePreview v-else />
      </aside>
    </div>

    <StatusBar />
  </div>
</template>

<style scoped>
.cybermanju-shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
  width: 100vw;
  background: #000;
  color: #FFFFFF;
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
  padding: 6px 12px;
  border-bottom: 2px solid #FFFFFF;
  background: #000;
  min-height: 34px;
}

.toolbar-left, .toolbar-right {
  display: flex;
  align-items: center;
  gap: 6px;
}

.breadcrumb {
  font-family: 'Courier New', monospace;
  font-size: 11px;
  color: rgba(255,255,255,0.6);
}

.bc-root { color: #FFFFFF; }
.bc-sep { opacity: 0.4; margin: 0 2px; }
.bc-part { color: #FFFFFF; }

.tb-divider {
  width: 1px;
  height: 14px;
  background: rgba(255,255,255,0.3);
}

.file-count {
  font-family: 'Courier New', monospace;
  font-size: 10px;
  color: rgba(255,255,255,0.6);
}

.count-badge {
  margin-left: 8px;
  color: #FFFFFF;
  border: 1px solid #FFFFFF;
  padding: 0 4px;
  font-size: 9px;
}

.view-btn {
  background: transparent;
  border: 2px solid transparent;
  color: rgba(255,255,255,0.4);
  padding: 3px 6px;
  cursor: pointer;
  display: flex;
  align-items: center;
}

.view-btn:hover {
  color: #FFFFFF;
  border-color: #FFFFFF;
}

.view-btn.active {
  color: #000;
  background: #FFFFFF;
  border-color: #FFFFFF;
}

.content-panels {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
}

.right-panel {
  width: 360px;
  min-width: 360px;
  border-left: 2px solid #FFFFFF;
  background: #000;
  overflow-y: auto;
  overflow-x: hidden;
}

.search-results-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 0 12px 12px;
}

.search-result-item {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 10px !important;
  cursor: pointer;
}

.search-result-item:hover {
  transform: translate(2px, 2px);
  box-shadow: 1px 1px 0 #000 !important;
}

.search-match-type {
  font-size: 9px;
  font-weight: 700;
  padding: 2px 6px;
  border: 2px solid #000;
  background: #FFFFFF;
  color: #000;
  white-space: nowrap;
}

.search-result-body {
  flex: 1;
  min-width: 0;
}

.search-result-name {
  font-family: 'Courier New', monospace;
  font-weight: 700;
  color: #000;
  margin-bottom: 2px;
  font-size: 12px;
}

.search-result-snippet {
  font-size: 10px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.search-result-score {
  font-family: 'Courier New', monospace;
  font-size: 10px;
  color: #000;
  white-space: nowrap;
}

.accounts-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.account-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px !important;
  cursor: pointer;
}

.account-item:hover {
  transform: translate(2px, 2px);
  box-shadow: 1px 1px 0 #000 !important;
}

.account-item.active {
  border-color: #000 !important;
  border-width: 3px !important;
}

.account-info {
  flex: 1;
  min-width: 0;
}

.account-name {
  font-weight: 700;
  font-size: 13px;
  color: #000;
}

.account-meta {
  font-size: 10px;
  margin-top: 2px;
}

.active-badge {
  font-size: 9px;
  font-weight: 700;
  padding: 2px 8px;
  background: #000;
  color: #FFFFFF;
  border: 2px solid #000;
  letter-spacing: 0.5px;
}

.loose-groups-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.loose-group-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px !important;
}

.lg-info {
  flex: 1;
  min-width: 0;
}

.lg-name {
  font-weight: 700;
  color: #000;
}

.lg-count {
  font-size: 10px;
  font-family: 'Courier New', monospace;
}

.style-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.panel-page {
  padding: 12px;
}

.empty-state {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 160px;
}

.bw-title {
  font-family: 'Courier New', monospace;
  font-size: 11px;
  font-weight: 700;
  color: #000;
  margin-bottom: 12px;
  letter-spacing: 1px;
}
</style>
