<template>
  <aside class="sidebar" :class="{ collapsed: store.sidebarCollapsed }">
    <div v-if="!store.sidebarCollapsed" class="sidebar-account">
      <div class="bw-dot" :class="{ 'bw-dot-on': store.activeAccount }" />
      <span class="sidebar-account-name">{{ store.activeAccount?.name || 'NO_ACCOUNT' }}</span>
    </div>

    <div class="sidebar-tabs" role="tablist" aria-label="SIDEBAR SECTIONS">
      <button
        v-for="tab in sectionTabs"
        :key="tab.id"
        class="sidebar-tab"
        :class="{ active: store.sidebarSection === tab.id }"
        :title="tab.label"
        :aria-label="tab.label"
        :aria-selected="store.sidebarSection === tab.id ? 'true' : 'false'"
        role="tab"
        @click="store.sidebarSection = tab.id as SidebarSection; if (tab.id === 'landing') store.currentPanel = 'landing'"
      >
        <span class="tab-icon">{{ tab.icon }}</span>
        <span v-if="!store.sidebarCollapsed" class="tab-label">{{ tab.label }}</span>
      </button>
    </div>

    <div v-if="!store.sidebarCollapsed" class="sidebar-content">
      <div v-if="store.sidebarSection === 'landing'" class="sidebar-section">
        <div class="section-header">QUICK LINKS</div>
        <div class="quick-links">
          <button class="ql-item" @click="store.currentPanel = 'landing'">[IN] LAUNCH APP</button>
          <button class="ql-item" @click="store.currentPanel = 'sync'">[CL] CLOUD SYNC</button>
          <a href="https://github.com/hautlythird211/Cybermanju-Drive" target="_blank" class="ql-item">[GH] SOURCE CODE</a>
          <a href="https://github.com/hautlythird211/Cybermanju-Drive/blob/main/README.md" target="_blank" class="ql-item">[DOC] DOCS</a>
        </div>
      </div>

      <div v-if="store.sidebarSection === 'tree'" class="sidebar-section"
        @contextmenu.prevent="showTreeContextMenu($event)"
      >
        <div class="section-header">FILE TREE</div>
        <div class="tree-container">
          <TreeNode
            v-for="folder in rootFolders"
            :key="folder.id"
            :node="folder"
            :depth="0"
            @select="store.selectFile"
          />
          <div v-if="rootFolders.length === 0" class="empty-section text-muted">NO FOLDERS</div>
        </div>
      </div>

      <div v-if="store.sidebarSection === 'locations'" class="sidebar-section">
        <div class="section-header">LOCATIONS</div>
        <div class="location-list">
          <div
            v-for="account in store.accounts"
            :key="account.id"
            class="sidebar-item"
            :class="{ active: account.id === store.activeAccountId }"
            @click="store.switchAccount(account.id)"
          >
            <div class="bw-dot" :class="{ 'bw-dot-on': account.id === store.activeAccountId }" />
            <span class="item-name truncate">{{ account.name }}</span>
            <span class="item-meta text-muted">{{ account.accountType }}</span>
          </div>
        </div>
      </div>

      <div v-if="store.sidebarSection === 'collections'" class="sidebar-section">
        <div class="section-header">COLLECTIONS</div>
        <div class="collection-list">
          <div
            v-for="col in store.collections"
            :key="col.id"
            class="sidebar-item"
          >
            <div class="bw-dot bw-dot-on" />
            <div class="item-info">
              <span class="item-name truncate">{{ col.name }}</span>
              <span class="item-meta text-muted">{{ col.collectionType }}</span>
            </div>
          </div>
        </div>
      </div>

      <div v-if="store.sidebarSection === 'people'" class="sidebar-section">
        <div class="section-header">PEOPLE</div>
        <div class="people-list">
          <div
            v-for="face in store.faceGroups"
            :key="face.id"
            class="sidebar-item"
          >
            <div class="avatar-circle">
              <span>++</span>
            </div>
            <div class="item-info">
              <span class="item-name truncate">{{ face.name }}</span>
              <span class="item-meta text-muted">{{ face.fileIds.length }} FILES</span>
            </div>
          </div>
        </div>
      </div>

      <div v-if="store.sidebarSection === 'styles'" class="sidebar-section">
        <div class="section-header">TAGS</div>
        <div class="tag-cloud">
          <span v-if="allTags.length === 0" class="text-muted" style="font-size:10px;padding:8px;">NO TAGS</span>
          <span
            v-for="tag in allTags"
            :key="tag"
            class="tag-item"
            @click="store.searchQuery = tag; store.currentPanel = 'search'; store.searchFiles(tag)"
          >
            {{ tag }}
          </span>
        </div>
      </div>

      <div v-if="store.sidebarSection === 'loose'" class="sidebar-section">
        <div class="section-header">LOOSE GROUPS</div>
        <div class="loose-list">
          <div
            v-for="group in store.looseGroups"
            :key="group.id"
            class="sidebar-item"
          >
            <div class="bw-dot bw-dot-on" />
            <div class="item-info">
              <span class="item-name truncate">{{ group.name }}</span>
              <span class="item-meta text-muted">{{ group.fileIds.length }} FILES</span>
            </div>
          </div>
        </div>
      </div>

      <div v-if="store.sidebarSection === 'users'" class="sidebar-section">
        <div class="section-header" @click="store.currentPanel = 'users'">USER ACCESS &gt;</div>
        <div class="section-body">
          <p class="text-muted" style="font-size:10px;padding:8px 0;">PER-FILE USERNAME + PASSWORD AUTH WITH ARGON2</p>
          <button class="bw-btn" style="width:100%;font-size:10px;" @click="store.currentPanel = 'users'">[OPEN] USER MGMT</button>
        </div>
      </div>

      <div v-if="store.sidebarSection === 'sync'" class="sidebar-section">
        <div class="section-header" @click="store.currentPanel = 'sync'">STORAGE SYNC &gt;</div>
        <div class="section-body">
          <p class="text-muted" style="font-size:10px;padding:8px 0;">SYNC TO LOCAL, GITHUB, GDRIVE, GPHOTOS</p>
          <div class="sync-backend-list">
            <div v-for="config in store.syncConfigs" :key="config.id" class="sidebar-item" style="margin-bottom:2px;">
              <div class="bw-dot" :class="{ 'bw-dot-on': config.enabled }" />
              <div class="item-info">
                <span class="item-name truncate">{{ config.backendType }}</span>
              </div>
            </div>
          </div>
          <button class="bw-btn" style="width:100%;font-size:10px;margin-top:6px;" @click="store.currentPanel = 'sync'">[OPEN] SYNC PANEL</button>
        </div>
      </div>

      <div v-if="store.sidebarSection === 'dashboard'" class="sidebar-section">
        <div class="section-header" @click="store.currentPanel = 'dashboard'">REMOTE ACCESS &gt;</div>
        <div class="section-body">
          <p class="text-muted" style="font-size:10px;padding:8px 0;">WEB DASHBOARD ON PORT 3456</p>
          <div class="bw-card" style="padding:6px;margin-bottom:6px;">
            <code style="font-size:10px;color:#000;">{{ dashboardUrl }}</code>
          </div>
          <button class="bw-btn" style="width:100%;font-size:10px;" @click="store.currentPanel = 'dashboard'">[OPEN] DASHBOARD</button>
        </div>
      </div>

      <div v-if="store.sidebarSection === 'tools'" class="sidebar-section">
        <div class="section-header">TOOLS</div>
        <div class="tools-list">
          <button class="ql-item" @click="store.currentPanel = 'favorites'" aria-label="OPEN FAVORITES">[*] FAVORITES ({{ store.starredFiles.length }})</button>
          <button class="ql-item" @click="store.currentPanel = 'recent'" aria-label="OPEN RECENT FILES">[T] RECENT FILES</button>
          <button class="ql-item" @click="store.currentPanel = 'activity'" aria-label="OPEN ACTIVITY LOG">[~] ACTIVITY LOG</button>
          <button class="ql-item" @click="store.currentPanel = 'storage'" aria-label="OPEN STORAGE DASHBOARD">[$] STORAGE</button>
          <button class="ql-item" @click="store.currentPanel = 'settings'" aria-label="OPEN SETTINGS">[@] SETTINGS</button>
          <button class="ql-item" @click="store.currentPanel = 'trash'" aria-label="OPEN TRASH">[%] TRASH</button>
        </div>
      </div>
    </div>

    <div v-if="!store.sidebarCollapsed" class="sidebar-bottom">
      <button class="qa-btn" @click="store.fetchGeoFiles(); store.currentPanel = 'map'" aria-label="OPEN MAP VIEW">[MAP]</button>
      <button class="qa-btn" @click="store.fetchCollections(); store.sidebarSection = 'collections'" aria-label="OPEN COLLECTIONS">[COL]</button>
      <button class="qa-btn" @click="store.detectFaces(store.selectedFileId || '')" aria-label="DETECT FACES">[FACE]</button>
    </div>

    <button class="collapse-btn" @click="store.sidebarCollapsed = !store.sidebarCollapsed" :aria-label="store.sidebarCollapsed ? 'EXPAND SIDEBAR' : 'COLLAPSE SIDEBAR'">
      <span style="display:inline-block;transform:rotate(store.sidebarCollapsed ? 0deg : 180deg)">^</span>
    </button>


  </aside>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useAppStore } from '@/stores/app'
import { useContextMenu } from '@/composables/useContextMenu'
import { isWebMode } from '@/composables/useTauri'
import type { SidebarSection } from '@/types'
import TreeNode from './TreeNode.vue'

const ctx = useContextMenu()

const store = useAppStore()

const dashboardUrl = computed(() => {
  if (typeof window !== 'undefined' && window.location?.port === '3456') {
    return window.location.origin
  }
  return 'HTTP://LOCALHOST:3456'
})

const allTags = computed<string[]>(() => {
  const tagSet = new Set<string>()
  store.files.forEach(f => f.tags?.forEach(t => tagSet.add(t)))
  return [...tagSet].sort()
})

const sectionTabs = computed(() => {
  const tabs: { id: string; label: string; icon: string }[] = []
  if (isWebMode()) {
    tabs.push({ id: 'landing', label: 'HOME', icon: '[~]' })
  }
  tabs.push(
    { id: 'tree', label: 'TREE', icon: '[#]' },
    { id: 'locations', label: 'LOCS', icon: '[@]' },
    { id: 'collections', label: 'COLS', icon: '[*]' },
    { id: 'people', label: 'PEOPLE', icon: '[+]' },
    { id: 'styles', label: 'TAGS', icon: '[&]' },
    { id: 'loose', label: 'LOOSE', icon: '[%]' },
    { id: 'sync', label: 'SYNC', icon: '[~]' },
    { id: 'users', label: 'USERS', icon: '[!]' },
    { id: 'dashboard', label: 'REMOTE', icon: '[@]' },
    { id: 'tools', label: 'TOOLS', icon: '[@]' },
  )
  return tabs
})

const rootFolders = computed(() =>
  store.files.filter(f => f.fileType === 'folder' && !f.parentId)
)

function showTreeContextMenu(e: MouseEvent) {
  ctx.open(e, 'sidebar_bg')
}
</script>

<style scoped>
.sidebar {
  grid-area: sidebar;
  width: 240px;
  min-width: 48px;
  display: flex;
  flex-direction: column;
  background: #000;
  border-right: 2px solid #FFFFFF;
  overflow: hidden;
  transition: width 0.15s;
  position: relative;
  z-index: 5;
}

.sidebar.collapsed {
  width: 48px;
}

.sidebar-account {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 10px 6px;
  border-bottom: 2px solid #FFFFFF;
}

.sidebar-account-name {
  font-family: 'Courier New', monospace;
  font-size: 10px;
  font-weight: 700;
  color: #FFFFFF;
  text-transform: uppercase;
}

.sidebar-tabs {
  display: flex;
  flex-direction: column;
  padding: 4px;
  gap: 1px;
  border-bottom: 2px solid #FFFFFF;
}

.sidebar-tab {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
  cursor: pointer;
  color: rgba(255,255,255,0.5);
  font-size: 10px;
  font-family: 'Courier New', monospace;
  font-weight: 700;
  border: 2px solid transparent;
}

.sidebar-tab:hover {
  color: #FFFFFF;
  border-color: #FFFFFF;
}

.sidebar-tab.active {
  color: #000;
  background: #FFFFFF;
}

.tab-icon {
  font-family: 'Courier New', monospace;
  font-size: 11px;
  width: 20px;
  text-align: center;
}

.tab-label {
  white-space: nowrap;
  font-size: 10px;
  letter-spacing: 0.5px;
}

.quick-links {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.ql-item {
  display: block;
  padding: 6px 10px;
  font-family: 'Courier New', monospace;
  font-size: 10px;
  color: rgba(255,255,255,0.7);
  cursor: pointer;
  text-decoration: none;
  border: none;
  background: transparent;
  text-align: left;
}

.ql-item:hover {
  background: #FFFFFF;
  color: #000;
}

.sidebar-content {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
}

.sidebar-section {
  padding: 6px;
}

.section-header {
  font-family: 'Courier New', monospace;
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 1px;
  color: rgba(255,255,255,0.4);
  padding: 4px 8px;
  cursor: default;
}

.section-header:hover {
  color: #FFFFFF;
}

.sidebar-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 5px 8px;
  margin-bottom: 2px;
  cursor: pointer;
  border: 2px solid transparent;
}

.sidebar-item:hover {
  border-color: #FFFFFF;
}

.sidebar-item.active {
  background: #FFFFFF;
}

.sidebar-item.active .item-name {
  color: #000;
}

.item-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.item-name {
  font-size: 11px;
  color: #FFFFFF;
  font-weight: 500;
}

.item-meta {
  font-size: 9px;
  font-family: 'Courier New', monospace;
}

.avatar-circle {
  width: 24px;
  height: 24px;
  border: 2px solid #FFFFFF;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  font-size: 9px;
  color: #FFFFFF;
}

.tag-cloud {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  padding: 4px;
}

.tag-item {
  display: inline-block;
  padding: 2px 6px;
  font-size: 9px;
  font-family: 'Courier New', monospace;
  font-weight: 700;
  border: 2px solid #FFFFFF;
  color: #FFFFFF;
  cursor: pointer;
}

.tag-item:hover {
  background: #FFFFFF;
  color: #000;
}

.tree-container {
  padding: 2px 0;
}

.tree-node-row {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 3px 8px;
  cursor: pointer;
  font-size: 11px;
  color: #FFFFFF;
}

.tree-node-row:hover {
  background: rgba(255,255,255,0.1);
}

.tree-arrow {
  color: rgba(255,255,255,0.5);
  font-size: 9px;
}

.tree-name {
  font-family: 'Courier New', monospace;
}

.empty-section {
  padding: 8px;
  font-size: 10px;
  text-align: center;
}

.sidebar-bottom {
  display: flex;
  gap: 2px;
  padding: 4px;
  border-top: 2px solid #FFFFFF;
}

.qa-btn {
  flex: 1;
  padding: 4px 2px;
  font-family: 'Courier New', monospace;
  font-size: 8px;
  font-weight: 700;
  color: rgba(255,255,255,0.6);
  cursor: pointer;
  border: 2px solid transparent;
  background: transparent;
  text-align: center;
}

.qa-btn:hover {
  border-color: #FFFFFF;
  color: #FFFFFF;
}

.collapse-btn {
  position: absolute;
  top: 50%;
  right: -12px;
  transform: translateY(-50%);
  width: 14px;
  height: 32px;
  background: #000;
  border: 2px solid #FFFFFF;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  z-index: 20;
  color: #FFFFFF;
  font-size: 9px;
  font-family: 'Courier New', monospace;
  padding: 0;
}

.collapse-btn:hover {
  background: #FFFFFF;
  color: #000;
}

.text-muted { opacity: 0.6; color: #FFFFFF !important; }
.truncate { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

@media (max-width: 768px) {
  .sidebar.collapsed {
    width: 36px;
    min-width: 36px;
  }
  .sidebar.collapsed .sidebar-tab {
    padding: 4px 2px;
  }
}

@media (min-width: 769px) and (max-width: 1024px) {
  .sidebar.collapsed {
    width: 40px;
    min-width: 40px;
  }
}
</style>
