<template>
  <div class="window-content-panel">
    <!-- Search Panel -->
    <div v-if="panelType === 'search'" class="panel-search">
      <div class="bw-card" style="padding: 12px; margin-bottom: 12px;">
        <div class="bw-title">TANTIVY SEARCH</div>
        <div class="search-controls">
          <p class="text-muted" style="flex:1;">"{{ store.searchQuery }}" - {{ filteredSearchResults.length }} RESULTS</p>
          <select v-model="searchTypeFilter" class="bw-select-sm" title="FILTER BY TYPE">
            <option value="all">ALL</option>
            <option value="image">IMAGES</option>
            <option value="text">TEXT</option>
            <option value="folder">FOLDERS</option>
            <option value="file">FILES</option>
          </select>
          <label class="search-current-dir" title="SEARCH ONLY IN CURRENT DIRECTORY">
            <input type="checkbox" v-model="searchCurrentDir" class="bw-checkbox" />
            <span class="text-muted" style="font-size:9px;">DIR</span>
          </label>
        </div>
      </div>

      <div v-if="recentSearches.length > 0 && !store.searchQuery" class="recent-searches">
        <div class="bw-title" style="padding: 0 12px; margin-bottom: 6px;">RECENT SEARCHES</div>
        <div v-for="sq in recentSearches" :key="sq" class="recent-search-item" @click="store.searchQuery = sq; store.searchFiles(sq)">
          <span class="text-muted">[R]</span>
          <span>{{ sq }}</span>
        </div>
      </div>

      <div v-if="filteredSearchResults.length" class="search-results-list">
        <div
          v-for="result in filteredSearchResults"
          :key="result.fileId"
          class="search-result-item bw-card"
          @click="store.selectFile(result.fileId); wm.open('files')"
        >
          <div class="search-match-type">{{ result.matchType?.toUpperCase() || 'SEARCH' }}</div>
          <div class="search-result-body">
            <div class="search-result-name" v-html="highlightTerms(result.fileName, store.searchQuery)"></div>
            <div class="search-result-snippet text-muted" v-if="result.snippet" v-html="highlightTerms(result.snippet, store.searchQuery)"></div>
          </div>
          <div class="search-result-score">{{ result.score.toFixed(3) }}</div>
        </div>
        <button
          v-if="filteredSearchResults.length < store.searchTotalResults"
          class="load-more-btn"
          @click="store.loadMoreSearchResults()"
          :disabled="store.isSearching"
        >[LOAD MORE] ({{ store.searchTotalResults - filteredSearchResults.length }} MORE)</button>
      </div>
      <div v-else-if="store.searchQuery && !store.isSearching" class="empty-state">
        <p class="text-muted">NO RESULTS FOR "{{ store.searchQuery }}"</p>
      </div>
      <div v-else-if="!store.searchQuery" class="empty-state">
        <p class="text-muted">TYPE IN SEARCH BAR FOR TANTIVY BM25 SEARCH</p>
      </div>
    </div>

    <!-- Trash Panel -->
    <div v-if="panelType === 'trash'" class="panel-page">
      <div class="panel-card">
        <div class="trash-header">
          <div class="panel-title">TRASH</div>
          <div class="trash-actions">
            <button class="panel-btn" @click="store.fetchTrashItems()" title="REFRESH TRASH">[R]</button>
            <button class="panel-btn panel-btn-danger" @click="store.emptyTrash()" title="EMPTY TRASH">[EMPTY]</button>
          </div>
        </div>
        <p class="panel-hint">DELETED FILES CAN BE RESTORED FROM HERE.</p>
        <div v-if="store.trashItems.length === 0" class="empty-state" style="height:80px;">
          <p class="text-muted">NO FILES IN TRASH</p>
        </div>
        <div v-else class="trash-list">
          <div v-for="item in store.trashItems" :key="item.id" class="trash-item">
            <span class="trash-icon">{{ item.originalFile.fileType === 'folder' ? '[+]' : '[=]' }}</span>
            <div class="trash-info">
              <span class="trash-name truncate">{{ item.originalFile.name }}</span>
              <span class="trash-date text-muted">{{ new Date(item.deletedAt).toLocaleDateString() }}</span>
            </div>
            <div class="trash-actions">
              <button class="trash-action-btn" @click="store.restoreTrashItem(item.originalFile.id)" title="RESTORE">[RST]</button>
              <button class="trash-action-btn danger" @click="store.deleteFromTrash(item.originalFile.id)" title="DELETE PERMANENTLY">[DEL]</button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Activity Panel -->
    <div v-if="panelType === 'activity'" class="panel-page">
      <div class="panel-card">
        <div class="trash-header">
          <div class="panel-title">ACTIVITY LOG</div>
          <button class="panel-btn" @click="store.fetchAuditLog()" title="REFRESH">[R]</button>
        </div>
        <p class="panel-hint">FILE OPERATIONS TIMELINE.</p>
        <div v-if="store.auditLog.length === 0" class="empty-state" style="height:80px;">
          <p class="text-muted">NO RECENT ACTIVITY</p>
        </div>
        <div v-else class="activity-list">
          <div v-for="entry in store.auditLog" :key="entry.id" class="activity-item">
            <span class="activity-action">{{ entry.action.toUpperCase() }}</span>
            <span class="activity-entity text-muted">{{ entry.entityType }}</span>
            <span class="activity-date text-muted">{{ new Date(entry.timestamp).toLocaleString() }}</span>
            <span v-if="entry.details && Object.keys(entry.details).length" class="activity-detail text-muted">{{ JSON.stringify(entry.details).substring(0, 40) }}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Favorites Panel -->
    <div v-if="panelType === 'favorites'" class="panel-page">
      <div class="panel-card">
        <div class="panel-title">FAVORITES</div>
        <div v-if="store.starredFiles.length === 0" class="empty-state" style="height:100px;">
          <p class="text-muted">NO STARRED FILES</p>
        </div>
        <div v-else class="fav-list">
          <div v-for="f in store.starredFiles" :key="f.id" class="fav-item" @click="store.selectFile(f.id)">
            <span class="fav-icon">{{ f.fileType === 'folder' ? '[+]' : '[=]' }}</span>
            <span class="fav-name truncate">{{ f.name }}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Recent Panel -->
    <div v-if="panelType === 'recent'" class="panel-page">
      <div class="panel-card">
        <div class="panel-title">RECENT FILES</div>
        <div v-if="recentFiles.length === 0" class="empty-state" style="height:100px;">
          <p class="text-muted">NO FILES YET</p>
        </div>
        <div v-else class="recent-list">
          <div
            v-for="f in recentFiles"
            :key="f.id"
            class="recent-item"
            @click="store.selectFile(f.id)"
          >
            <span class="recent-icon">{{ f.fileType === 'folder' ? '[+]' : '[=]' }}</span>
            <div class="recent-info">
              <span class="recent-name truncate">{{ f.name }}</span>
              <span class="recent-date text-muted">{{ new Date(f.modifiedAt).toLocaleDateString() }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Accounts Panel -->
    <div v-if="panelType === 'accounts'" class="panel-page">
      <div class="panel-card">
        <div class="panel-title">MULTI-ACCOUNT MANAGER</div>
        <div class="accounts-list">
          <div
            v-for="account in store.accounts"
            :key="account.id"
            class="account-item panel-card-row"
            :class="{ active: account.isActive }"
            @click="store.switchAccount(account.id)"
          >
            <div class="bw-dot" :class="{ 'bw-dot-on': account.isActive }" />
            <div class="account-info">
              <div class="account-name">{{ account.name }}</div>
              <div class="account-meta">{{ account.accountType }} {{ account.path }}</div>
            </div>
            <div v-if="account.isActive" class="active-badge">ACTIVE</div>
          </div>
        </div>
      </div>
    </div>

    <!-- Loose Groups Panel -->
    <div v-if="panelType === 'loose-groups'" class="panel-page">
      <div class="panel-card">
        <div class="panel-title">LOOSE FILE GROUPING</div>
        <div class="loose-groups-list">
          <div v-for="group in store.looseGroups" :key="group.id" class="loose-group-item panel-card-row">
            <div class="lg-info">
              <div class="lg-name">{{ group.name }}</div>
              <div class="lg-count">{{ group.fileIds.length }} FILES</div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Style Tags Panel -->
    <div v-if="panelType === 'style'" class="panel-page">
      <div class="panel-card">
        <div class="panel-title">STYLE-BASED ORGANIZATION</div>
        <p class="panel-hint">FILES ORGANIZED BY VISUAL STYLE (CLIP MODEL)</p>
        <div class="style-tags">
          <span v-for="tag in [...new Set(store.files.flatMap(f => f.tags || []))]" :key="tag" class="style-tag" @click="store.searchQuery = tag; wm.open('search'); store.searchFiles(tag)">
            {{ tag }}
          </span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useAppStore } from '@/stores/app'
import { useWindowManager } from '@/composables/useWindowManager'
import type { PanelType } from '@/types'

const props = defineProps<{
  panelType: PanelType
}>()

const store = useAppStore()
const wm = useWindowManager()

const searchTypeFilter = ref('all')
const searchCurrentDir = ref(false)

const recentSearches = ref<string[]>((() => {
  try { return JSON.parse(localStorage.getItem('cybermanju_recent_searches') || '[]') as string[] } catch { return [] }
})())

const recentFiles = computed(() =>
  [...store.files]
    .sort((a, b) => new Date(b.modifiedAt).getTime() - new Date(a.modifiedAt).getTime())
    .slice(0, 20)
)

const filteredSearchResults = computed(() => {
  const results = store.searchResults
  if (searchTypeFilter.value === 'all') return results
  return results.filter(r => {
    if (searchTypeFilter.value === 'folder') return r.matchType === 'folder'
    if (searchTypeFilter.value === 'image') return r.matchType === 'image'
    if (searchTypeFilter.value === 'text') return r.matchType === 'text'
    return true
  })
})

function escapeHtml(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
}

function highlightTerms(text: string, query: string): string {
  if (!query.trim()) return escapeHtml(text)
  const escaped = escapeHtml(query).replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  return escapeHtml(text).replace(new RegExp(`(${escaped})`, 'gi'), '<mark>$1</mark>')
}
</script>

<style scoped>
.window-content-panel {
  height: 100%;
  overflow-y: auto;
  padding: 0;
  background: #111;
}

.panel-page {
  padding: 12px;
}

.panel-card {
  background: #1a1a1a;
  border: 1px solid #2a2a2a;
  border-radius: 8px;
  padding: 16px;
}

.panel-card-row {
  background: #1a1a1a;
  border: 1px solid #2a2a2a;
  border-radius: 6px;
  padding: 10px 12px;
  cursor: pointer;
  transition: border-color 0.1s;
}

.panel-card-row:hover {
  border-color: #3a3a3a;
}

.panel-card-row.active {
  border-color: rgba(0, 255, 65, 0.3);
}

.panel-title {
  font-family: 'Courier New', monospace;
  font-size: 11px;
  font-weight: 700;
  color: #e0e0e0;
  letter-spacing: 1px;
  margin-bottom: 8px;
}

.panel-hint {
  font-family: 'Courier New', monospace;
  font-size: 9px;
  color: #555;
  margin-bottom: 12px;
}

.panel-btn {
  background: transparent;
  border: 1px solid #333;
  color: #999;
  padding: 2px 8px;
  font-family: 'Courier New', monospace;
  font-size: 9px;
  font-weight: 700;
  cursor: pointer;
  border-radius: 4px;
  transition: all 0.1s;
}

.panel-btn:hover {
  border-color: #555;
  color: #e0e0e0;
  background: #222;
}

.panel-btn-danger:hover {
  border-color: #ff5f57;
  color: #ff5f57;
}

.bw-title {
  font-family: 'Courier New', monospace;
  font-size: 11px;
  font-weight: 700;
  color: #e0e0e0;
  margin-bottom: 12px;
  letter-spacing: 1px;
}

.bw-card {
  background: #1a1a1a;
  border: 1px solid #2a2a2a;
  border-radius: 6px;
  padding: 12px;
  color: #ccc;
}

.text-muted {
  color: #555 !important;
}

.empty-state {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  color: #555;
  font-family: 'Courier New', monospace;
  font-size: 10px;
}

/* Search panel styles */
.panel-search {
  padding: 12px;
}

.search-controls {
  display: flex;
  align-items: center;
  gap: 8px;
}

.bw-select-sm {
  background: #111;
  border: 1px solid #333;
  color: #ccc;
  font-family: 'Courier New', monospace;
  font-size: 9px;
  padding: 2px 4px;
  cursor: pointer;
  appearance: none;
  border-radius: 4px;
}

.search-current-dir {
  display: flex;
  align-items: center;
  gap: 4px;
  cursor: pointer;
}

.bw-checkbox {
  appearance: none;
  width: 12px;
  height: 12px;
  border: 1px solid #555;
  background: #111;
  cursor: pointer;
  border-radius: 2px;
}

.bw-checkbox:checked {
  background: #00ff41;
  border-color: #00ff41;
}

.search-results-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.search-result-item {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 10px !important;
  cursor: pointer;
  transition: border-color 0.1s;
}

.search-result-item:hover {
  border-color: #444;
}

.search-match-type {
  font-size: 8px;
  font-weight: 700;
  padding: 2px 6px;
  background: #222;
  color: #999;
  white-space: nowrap;
  border-radius: 3px;
}

.search-result-body {
  flex: 1;
  min-width: 0;
}

.search-result-name {
  font-family: 'Courier New', monospace;
  font-weight: 600;
  color: #e0e0e0;
  margin-bottom: 2px;
  font-size: 11px;
}

.search-result-snippet {
  font-size: 9px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: #666;
}

.search-result-score {
  font-family: 'Courier New', monospace;
  font-size: 9px;
  color: #555;
  white-space: nowrap;
}

.load-more-btn {
  width: 100%;
  padding: 8px;
  background: transparent;
  border: 1px solid #333;
  border-radius: 6px;
  color: #999;
  font-family: 'Courier New', monospace;
  font-size: 10px;
  font-weight: 700;
  cursor: pointer;
  text-align: center;
  transition: all 0.1s;
}

.load-more-btn:hover {
  border-color: #555;
  color: #e0e0e0;
  background: #1a1a1a;
}

.load-more-btn:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

.recent-searches {
  display: flex;
  flex-direction: column;
  gap: 2px;
  margin-bottom: 12px;
}

.recent-search-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  font-size: 10px;
  cursor: pointer;
  border: 1px solid transparent;
  font-family: 'Courier New', monospace;
  color: #ccc;
  border-radius: 4px;
}

.recent-search-item:hover {
  border-color: #333;
  background: #1a1a1a;
}

/* Trash styles */
.trash-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 4px;
}

.trash-actions {
  display: flex;
  gap: 4px;
}

.trash-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.trash-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border: 1px solid transparent;
  border-radius: 4px;
}

.trash-item:hover {
  border-color: #333;
  background: #1a1a1a;
}

.trash-icon {
  font-size: 10px;
  flex-shrink: 0;
  color: #666;
}

.trash-info {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.trash-name {
  font-size: 10px;
  font-weight: 600;
  color: #ccc;
}

.trash-date {
  font-size: 8px;
  color: #555 !important;
}

.trash-action-btn {
  background: transparent;
  border: 1px solid #333;
  color: #888;
  padding: 1px 6px;
  font-family: 'Courier New', monospace;
  font-size: 8px;
  font-weight: 700;
  cursor: pointer;
  border-radius: 3px;
  transition: all 0.1s;
}

.trash-action-btn:hover {
  border-color: #555;
  color: #e0e0e0;
}

.trash-action-btn.danger:hover {
  border-color: #ff5f57;
  color: #ff5f57;
}

/* Activity styles */
.activity-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.activity-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 6px;
  font-size: 9px;
  border-bottom: 1px solid #222;
}

.activity-action {
  font-weight: 700;
  color: #e0e0e0;
  flex-shrink: 0;
  min-width: 50px;
}

.activity-entity {
  flex-shrink: 0;
  min-width: 30px;
}

.activity-date {
  flex-shrink: 0;
}

.activity-detail {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  text-align: right;
}

/* Favorites/Recent styles */
.fav-list, .recent-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.fav-item, .recent-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 5px 8px;
  cursor: pointer;
  border: 1px solid transparent;
  border-radius: 4px;
}

.fav-item:hover, .recent-item:hover {
  border-color: #333;
  background: #1a1a1a;
}

.fav-icon, .recent-icon {
  font-size: 10px;
  flex-shrink: 0;
  color: #666;
}

.fav-name, .recent-name {
  font-size: 10px;
  font-weight: 600;
  color: #ccc;
  flex: 1;
}

.recent-info {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.recent-date {
  font-size: 8px;
  color: #555 !important;
}

/* Accounts styles */
.accounts-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.account-item {
  display: flex;
  align-items: center;
  gap: 10px;
}

.account-info {
  flex: 1;
  min-width: 0;
}

.account-name {
  font-weight: 600;
  font-size: 11px;
  color: #e0e0e0;
}

.account-meta {
  font-size: 9px;
  margin-top: 1px;
  color: #666;
}

.active-badge {
  font-size: 8px;
  font-weight: 700;
  padding: 2px 8px;
  background: rgba(0, 255, 65, 0.1);
  color: #00ff41;
  border: 1px solid rgba(0, 255, 65, 0.2);
  border-radius: 3px;
  letter-spacing: 0.5px;
}

/* Loose groups styles */
.loose-groups-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.loose-group-item {
  display: flex;
  align-items: center;
  gap: 10px;
}

.lg-info {
  flex: 1;
  min-width: 0;
}

.lg-name {
  font-weight: 600;
  color: #ccc;
  font-size: 11px;
}

.lg-count {
  font-size: 9px;
  color: #666;
}

/* Style tags */
.style-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.style-tag {
  display: inline-block;
  padding: 2px 8px;
  font-size: 9px;
  font-family: 'Courier New', monospace;
  font-weight: 600;
  border: 1px solid #333;
  border-radius: 4px;
  color: #aaa;
  cursor: pointer;
  transition: all 0.1s;
}

.style-tag:hover {
  border-color: #555;
  color: #e0e0e0;
  background: #1a1a1a;
}

.bw-dot {
  width: 8px;
  height: 8px;
  display: inline-block;
  border: 2px solid #555;
  background: transparent;
  border-radius: 50%;
  flex-shrink: 0;
}

.bw-dot-on {
  background: #00ff41;
  border-color: #00ff41;
}

.truncate {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.search-result-name :deep(mark) {
  background: rgba(0, 255, 65, 0.2);
  color: #00ff41;
  padding: 0 2px;
  border-radius: 2px;
}

.search-result-snippet :deep(mark) {
  background: rgba(255, 255, 255, 0.1);
  color: #ccc;
  padding: 0 2px;
  border-radius: 2px;
}
</style>
