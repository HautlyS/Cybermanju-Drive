<template>
  <aside
    class="sidebar panel-left"
    :class="{ 'sidebar-collapsed': store.sidebarCollapsed }"
  >
    <!-- Account indicator -->
    <div v-if="!store.sidebarCollapsed" class="sidebar-account">
      <span
        class="dot"
        :style="{
          background: store.activeAccount?.color || '#6B7280',
          boxShadow: `0 0 6px ${store.activeAccount?.color || '#6B7280'}`
        }"
      />
      <span class="sidebar-account-name">{{ store.activeAccount?.name || 'No Account' }}</span>
    </div>

    <!-- Section tabs -->
    <div class="sidebar-tabs">
      <button
        v-for="tab in sectionTabs"
        :key="tab.id"
        class="sidebar-tab"
        :class="{ 'tab-active': store.sidebarSection === tab.id }"
        :title="tab.label"
        @click="store.sidebarSection = tab.id as any; if (tab.id === 'landing') store.currentPanel = 'landing'"
      >
        <component :is="tab.icon" :size="16" />
        <span v-if="!store.sidebarCollapsed" class="tab-label">{{ tab.label }}</span>
      </button>
    </div>

    <!-- Content area -->
    <div v-if="!store.sidebarCollapsed" class="sidebar-content">
      <!-- Landing page quick links -->
      <div v-if="store.sidebarSection === 'landing'" class="sidebar-section">
        <div class="section-header">
          <Home :size="14" class="text-neon" />
          <span class="section-title">Quick Links</span>
        </div>
        <div class="quick-links">
          <button class="quick-link" @click="store.currentPanel = 'landing'">
            <span class="ql-icon">📥</span>
            <span>Download App</span>
          </button>
          <button class="quick-link" @click="store.currentPanel = 'sync'">
            <span class="ql-icon">☁️</span>
            <span>Cloud Sync Setup</span>
          </button>
          <a href="https://github.com/hautlythird211/Cybermanju-Drive" target="_blank" class="quick-link">
            <span class="ql-icon">💻</span>
            <span>Source Code</span>
          </a>
          <a href="https://github.com/hautlythird211/Cybermanju-Drive/blob/main/README.md" target="_blank" class="quick-link">
            <span class="ql-icon">📖</span>
            <span>Documentation</span>
          </a>
        </div>
      </div>

      <!-- Tree view -->
      <div v-if="store.sidebarSection === 'tree'" class="sidebar-section">
        <div class="section-header">
          <FolderTree :size="14" class="text-neon" />
          <span class="section-title">File Tree</span>
        </div>
        <div class="tree-container">
          <TreeNode
            v-for="folder in rootFolders"
            :key="folder.id"
            :node="folder"
            :depth="0"
          />
          <div v-if="rootFolders.length === 0" class="empty-state">
            <Folder :size="20" class="text-muted" />
            <span class="text-muted">No folders</span>
          </div>
        </div>
      </div>

      <!-- Locations -->
      <div v-if="store.sidebarSection === 'locations'" class="sidebar-section">
        <div class="section-header">
          <HardDrive :size="14" class="text-cyan" />
          <span class="section-title">Locations</span>
        </div>
        <div class="location-list">
          <div
            v-for="account in store.accounts"
            :key="account.id"
            class="sidebar-item neobrutalism-card"
            :class="{ 'item-active': account.id === store.activeAccountId }"
            @click="store.switchAccount(account.id)"
          >
            <span class="dot" :style="{ background: account.color, boxShadow: `0 0 4px ${account.color}` }" />
            <span class="item-name truncate">{{ account.name }}</span>
            <span class="item-meta text-muted">{{ account.accountType }}</span>
          </div>
        </div>
      </div>

      <!-- Collections -->
      <div v-if="store.sidebarSection === 'collections'" class="sidebar-section">
        <div class="section-header">
          <BookMarked :size="14" class="text-gold" />
          <span class="section-title">Collections</span>
        </div>
        <div class="collection-list">
          <div
            v-for="col in store.collections"
            :key="col.id"
            class="sidebar-item neobrutalism-card"
            :style="{ borderLeftColor: col.color, borderLeftWidth: '4px' }"
          >
            <span class="dot" :style="{ background: col.color, boxShadow: `0 0 4px ${col.color}` }" />
            <div class="item-info">
              <span class="item-name truncate">{{ col.name }}</span>
              <span class="item-meta text-muted">{{ col.collectionType }}</span>
            </div>
            <Star v-if="col.collectionType === 'highlights'" :size="12" class="text-gold" />
          </div>
        </div>
      </div>

      <!-- People (face groups) -->
      <div v-if="store.sidebarSection === 'people'" class="sidebar-section">
        <div class="section-header">
          <Users :size="14" class="text-purple" />
          <span class="section-title">People</span>
        </div>
        <div class="people-list">
          <div
            v-for="face in store.faceGroups"
            :key="face.id"
            class="sidebar-item neobrutalism-card"
          >
            <div class="avatar-circle" :style="{ borderColor: face.color, boxShadow: `0 0 6px ${face.color}40` }">
              <Users :size="14" :style="{ color: face.color }" />
            </div>
            <div class="item-info">
              <span class="item-name truncate">{{ face.name }}</span>
              <span class="item-meta text-muted">{{ face.fileIds.length }} files</span>
            </div>
            <ChevronRight :size="12" class="text-muted" />
          </div>
        </div>
      </div>

      <!-- Styles (tags) -->
      <div v-if="store.sidebarSection === 'styles'" class="sidebar-section">
        <div class="section-header">
          <Palette :size="14" class="text-pink" />
          <span class="section-title">Tags</span>
        </div>
        <div class="tag-cloud">
          <span v-if="allTags.length === 0" class="tag-item badge badge-cyan" style="opacity:0.5">No tags yet</span>
          <span
            v-for="tag in allTags"
            :key="tag"
            class="tag-item badge badge-green"
            @click="store.searchQuery = tag; store.currentPanel = 'search'; store.searchFiles(tag)"
          >
            {{ tag }}
          </span>
        </div>
      </div>

      <!-- Loose groups -->
      <div v-if="store.sidebarSection === 'loose'" class="sidebar-section">
        <div class="section-header">
          <Group :size="14" class="text-orange" />
          <span class="section-title">Loose Groups</span>
        </div>
        <div class="loose-list">
          <div
            v-for="group in store.looseGroups"
            :key="group.id"
            class="sidebar-item neobrutalism-card"
            :style="{ borderLeftColor: group.color, borderLeftWidth: '4px' }"
          >
            <span class="dot" :style="{ background: group.color, boxShadow: `0 0 4px ${group.color}` }" />
            <div class="item-info">
              <span class="item-name truncate">{{ group.name }}</span>
              <span class="item-meta text-muted">{{ group.fileIds.length }} files</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Users / Access Control -->
      <div v-if="store.sidebarSection === 'users'" class="sidebar-section">
        <div class="section-header" @click="store.currentPanel = 'users'">
          <UserCheck :size="14" style="color: #FFB800" />
          <span class="section-title">User Access</span>
          <ChevronRight :size="14" class="text-muted ml-auto" />
        </div>
        <div class="section-body">
          <p class="text-muted" style="font-size: 12px; padding: 8px 0;">
            Per-file username + password authentication with argon2 hashing.
            Role-based access: admin, user, viewer.
          </p>
          <button
            class="neobrutalism-btn"
            style="width: 100%; font-size: 12px; padding: 6px 10px;"
            @click="store.currentPanel = 'users'"
          >
            <Shield :size="12" style="margin-right: 6px;" />
            Open User Management
          </button>
        </div>
      </div>

      <!-- Sync / Storage Backends -->
      <div v-if="store.sidebarSection === 'sync'" class="sidebar-section">
        <div class="section-header" @click="store.currentPanel = 'sync'">
          <CloudUpload :size="14" style="color: #00D4FF" />
          <span class="section-title">Storage Sync</span>
          <ChevronRight :size="14" class="text-muted ml-auto" />
        </div>
        <div class="section-body">
          <p class="text-muted" style="font-size: 12px; padding: 8px 0;">
            Sync files to Local, GitHub, Google Drive, or Google Photos backends.
          </p>
          <div class="sync-backend-list">
            <div
              v-for="config in store.syncConfigs"
              :key="config.id"
              class="sidebar-item neobrutalism-card"
            >
              <span class="dot" :style="{ background: getBackendColor(config.backendType), boxShadow: `0 0 4px ${getBackendColor(config.backendType)}` }" />
              <div class="item-info">
                <span class="item-name truncate">{{ getBackendLabel(config.backendType) }}</span>
                <span class="item-meta text-muted">{{ config.backendType }}</span>
              </div>
              <span
                class="enabled-dot"
                :class="{ on: config.enabled }"
              />
            </div>
          </div>
          <button
            class="neobrutalism-btn"
            style="width: 100%; font-size: 12px; padding: 6px 10px; margin-top: 8px;"
            @click="store.currentPanel = 'sync'"
          >
            <CloudUpload :size="12" style="margin-right: 6px;" />
            Open Sync Panel
          </button>
        </div>
      </div>

      <!-- Web Dashboard / Remote Access -->
      <div v-if="store.sidebarSection === 'dashboard'" class="sidebar-section">
        <div class="section-header" @click="store.currentPanel = 'dashboard'">
          <Globe :size="14" style="color: #00D4FF" />
          <span class="section-title">Remote Access</span>
          <ChevronRight :size="14" class="text-muted ml-auto" />
        </div>
        <div class="section-body">
          <p class="text-muted" style="font-size: 12px; padding: 8px 0;">
            Web dashboard on port 3456. Access from any device via browser.
          </p>
          <div class="neobrutalism-card" style="padding: 8px; margin-bottom: 8px;">
            <code style="font-size: 11px; color: var(--matrix-green);">{{ dashboardUrl }}</code>
          </div>
          <button
            class="neobrutalism-btn"
            style="width: 100%; font-size: 12px; padding: 6px 10px;"
            @click="store.currentPanel = 'dashboard'"
          >
            <Globe :size="12" style="margin-right: 6px;" />
            Open Dashboard Panel
          </button>
        </div>
      </div>
    </div>

    <!-- Bottom quick actions -->
    <div v-if="!store.sidebarCollapsed" class="sidebar-bottom">
      <button class="quick-action" title="Add location" @click="store.fetchGeoFiles(); store.currentPanel = 'map'">
        <Plus :size="14" class="text-neon" />
        <span>Location</span>
      </button>
      <button class="quick-action" title="New collection" @click="store.fetchCollections(); store.sidebarSection = 'collections'">
        <Star :size="14" class="text-gold" />
        <span>Collection</span>
      </button>
      <button class="quick-action" title="Scan faces" @click="store.detectFaces(store.selectedFileId || ''); store.searchFiles(store.searchQuery || '*')">
        <Users :size="14" class="text-purple" />
        <span>Scan</span>
      </button>
    </div>

    <!-- Collapse toggle -->
    <button class="collapse-toggle" @click="store.sidebarCollapsed = !store.sidebarCollapsed">
      <ChevronRight
        :size="14"
        :style="{ transform: store.sidebarCollapsed ? 'rotate(0deg)' : 'rotate(180deg)', transition: 'transform 0.2s' }"
      />
    </button>
  </aside>
</template>

<script setup lang="ts">
import { computed, h, defineComponent } from 'vue'
import {
  FolderTree, HardDrive, BookMarked, Users, Palette, Group,
  Plus, ChevronRight, Folder, File, Star, Lock, Link,
  UserCheck, Globe, Shield, CloudUpload, Home,
} from 'lucide-vue-next'
import { useAppStore } from '@/stores/app'
import { isWebMode } from '@/composables/useTauri'
import type { FileNode, SyncBackendType } from '@/types'
import { SYNC_BACKEND_INFO } from '@/types'

function getBackendLabel(type: SyncBackendType): string {
  return SYNC_BACKEND_INFO[type]?.name || type
}

function getBackendColor(type: SyncBackendType): string {
  return SYNC_BACKEND_INFO[type]?.color || '#6B7280'
}

const store = useAppStore()

const dashboardUrl = computed(() => {
  if (typeof window !== 'undefined' && window.location?.port === '3456') {
    return window.location.origin
  }
  return 'http://localhost:3456'
})

const allTags = computed<string[]>(() => {
  const tagSet = new Set<string>()
  store.files.forEach(f => f.tags?.forEach(t => tagSet.add(t)))
  return [...tagSet].sort()
})

const sectionTabs = computed(() => {
  const tabs = []
  // Show Home/landing tab only in web mode
  if (isWebMode()) {
    tabs.push({ id: 'landing', label: 'Home', icon: Home })
  }
  tabs.push(
    { id: 'tree', label: 'Tree', icon: FolderTree },
    { id: 'locations', label: 'Locations', icon: HardDrive },
    { id: 'collections', label: 'Collections', icon: BookMarked },
    { id: 'people', label: 'People', icon: Users },
    { id: 'styles', label: 'Styles', icon: Palette },
    { id: 'loose', label: 'Loose', icon: Group },
    { id: 'sync', label: 'Sync', icon: CloudUpload },
    { id: 'users', label: 'Users', icon: UserCheck },
    { id: 'dashboard', label: 'Remote', icon: Globe },
  )
  return tabs
})

const rootFolders = computed(() =>
  store.files.filter(f => f.fileType === 'folder' && !f.parentId)
)

// Recursive tree node component (inline)
const TreeNode = defineComponent({
  name: 'TreeNode',
  props: {
    node: { type: Object as () => FileNode, required: true },
    depth: { type: Number, required: true },
  },
  setup(props) {
    const store = useAppStore()
    const isExpanded = computed(() => store.selectedFileId === props.node.id)

    function toggleFolder() {
      if (isExpanded.value) {
        store.selectedFileId = null
      } else {
        store.selectedFileId = props.node.id
      }
    }

    const children = computed(() =>
      store.files.filter(f => f.parentId === props.node.id)
    )

    return (): ReturnType<typeof h> => h('div', { class: 'tree-node' }, [
      h('div', {
        class: ['tree-node-row', { 'tree-node-active': isExpanded.value }],
        style: { paddingLeft: `${props.depth * 16 + 8}px` },
        onClick: toggleFolder,
      }, [
        h(ChevronRight, {
          size: 12,
          style: {
            transform: isExpanded.value ? 'rotate(90deg)' : 'rotate(0deg)',
            transition: 'transform 0.15s',
            flexShrink: 0,
          },
          class: 'text-muted',
        }),
        props.node.encrypted
          ? h(Lock, { size: 13, class: 'text-neon', style: { flexShrink: 0 } })
          : h(Folder, { size: 13, class: 'text-gold', style: { flexShrink: 0 } }),
        h('span', { class: 'tree-node-name truncate' }, props.node.name),
      ]),
      isExpanded.value
        ? children.value.map(child =>
            h(TreeNode, { node: child, depth: props.depth + 1, key: child.id })
          )
        : null,
    ])
  },
})
</script>

<style scoped>
.sidebar {
  grid-area: sidebar;
  width: 280px;
  min-width: 56px;
  display: flex;
  flex-direction: column;
  background: var(--cyber-bg-panel);
  border-right: 3px solid #000000;
  overflow: hidden;
  transition: width 0.2s ease;
  position: relative;
  z-index: 5;
}

.sidebar-collapsed {
  width: 56px;
}

/* Account */
.sidebar-account {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px 8px;
  border-bottom: 2px solid var(--cyber-bg-hover);
}

.sidebar-account-name {
  font-family: monospace;
  font-size: 11px;
  font-weight: 700;
  color: var(--cyber-text-primary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

/* Tabs */
.sidebar-tabs {
  display: flex;
  flex-direction: column;
  padding: 6px;
  gap: 2px;
  border-bottom: 2px solid var(--cyber-bg-hover);
}

.sidebar-collapsed .sidebar-tabs {
  padding: 4px;
}

.sidebar-tab {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px;
  border-radius: 2px;
  cursor: pointer;
  color: var(--cyber-text-muted);
  transition: all 0.1s;
  font-size: 12px;
}

.sidebar-tab:hover {
  background: var(--cyber-bg-hover);
  color: var(--cyber-text-primary);
}

.sidebar-tab.tab-active {
  background: var(--cyber-bg-card);
  color: var(--cyber-matrix-green);
  border: 2px solid rgba(0, 255, 65, 0.3);
}

.tab-label {
  white-space: nowrap;
}

/* Quick links */
.quick-links {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 0 4px;
}

.quick-link {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: var(--cyber-text-secondary, #aaa);
  font-size: 12px;
  cursor: pointer;
  text-decoration: none;
  transition: all 0.1s;
  text-align: left;
  width: 100%;
}

.quick-link:hover {
  background: var(--cyber-bg-hover, rgba(255,255,255,0.05));
  color: var(--cyber-text-primary, #fff);
}

.ql-icon {
  font-size: 14px;
}

/* Content */
.sidebar-content {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
}

.sidebar-section {
  padding: 8px;
}

.section-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px 8px;
}

.section-title {
  font-family: monospace;
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: var(--cyber-text-muted);
}

/* Sidebar items */
.sidebar-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  margin-bottom: 4px;
  cursor: pointer;
  border-left: 4px solid transparent;
}

.sidebar-item:hover {
  border-color: var(--cyber-bg-hover);
  background: var(--cyber-bg-hover);
}

.sidebar-item.item-active {
  border-color: var(--cyber-matrix-green);
  box-shadow: inset 0 0 8px rgba(0, 255, 65, 0.05);
}

.item-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.item-name {
  font-size: 12px;
  color: var(--cyber-text-primary);
  font-weight: 500;
}

.item-meta {
  font-size: 10px;
  font-family: monospace;
}

/* Avatar circle */
.avatar-circle {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  border: 2px solid;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  background: var(--cyber-bg-deep);
}

/* Tree */
.tree-container {
  padding: 2px 0;
}

.tree-node-row {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 3px 8px;
  cursor: pointer;
  border-radius: 2px;
  transition: background 0.1s;
}

.tree-node-row:hover {
  background: var(--cyber-bg-hover);
}

.tree-node-row.tree-node-active {
  background: rgba(0, 255, 65, 0.06);
}

.tree-node-name {
  font-size: 12px;
  color: var(--cyber-text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* Tag cloud */
.tag-cloud {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  padding: 4px 8px;
}

.tag-item {
  cursor: pointer;
  transition: transform 0.1s, box-shadow 0.1s;
}

.tag-item:hover {
  transform: translateY(-1px);
  box-shadow: 2px 2px 0 #000;
}

/* Bottom */
.sidebar-bottom {
  display: flex;
  gap: 2px;
  padding: 6px;
  border-top: 2px solid var(--cyber-bg-hover);
}

.quick-action {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  padding: 6px 4px;
  border-radius: 2px;
  cursor: pointer;
  font-size: 9px;
  font-family: monospace;
  font-weight: 600;
  text-transform: uppercase;
  color: var(--cyber-text-muted);
  transition: all 0.1s;
}

.quick-action:hover {
  background: var(--cyber-bg-card);
  color: var(--cyber-text-primary);
}

/* Collapse toggle */
.collapse-toggle {
  position: absolute;
  top: 50%;
  right: -10px;
  transform: translateY(-50%);
  width: 20px;
  height: 40px;
  background: var(--cyber-bg-card);
  border: 2px solid #000000;
  border-radius: 0 4px 4px 0;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  z-index: 20;
  color: var(--cyber-text-muted);
  transition: color 0.1s;
}

.collapse-toggle:hover {
  color: var(--cyber-text-primary);
}

/* Empty state */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 20px;
  font-size: 11px;
}

/* Sync backend enabled dot */
.enabled-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--cyber-text-muted);
  opacity: 0.4;
  flex-shrink: 0;
  transition: all 0.15s;
}

.enabled-dot.on {
  background: var(--matrix-green);
  opacity: 1;
  box-shadow: 0 0 6px rgba(0, 255, 65, 0.5);
}
</style>