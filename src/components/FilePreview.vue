<template>
  <aside v-if="store.selectedFile" class="file-preview panel-right">
    <!-- File header -->
    <div class="preview-header">
      <File :size="16" class="text-neon" />
      <div class="preview-file-info">
        <span class="preview-filename mono truncate">{{ store.selectedFile.name }}</span>
        <span class="preview-path mono text-muted truncate">{{ store.selectedFile.path }}</span>
      </div>
    </div>

    <div class="preview-scroll">
      <!-- Metadata -->
      <div class="preview-section">
        <div class="section-label">Metadata</div>
        <div class="metadata-grid">
          <div class="meta-row">
            <span class="meta-key text-muted">Size</span>
            <span class="meta-value mono">{{ formatSize(store.selectedFile.sizeBytes) }}</span>
          </div>
          <div class="meta-row">
            <span class="meta-key text-muted">Type</span>
            <span class="meta-value">{{ store.selectedFile.mimeType || store.selectedFile.fileType }}</span>
          </div>
          <div class="meta-row">
            <span class="meta-key text-muted">Created</span>
            <span class="meta-value mono text-secondary">{{ formatDate(store.selectedFile.createdAt) }}</span>
          </div>
          <div class="meta-row">
            <span class="meta-key text-muted">Modified</span>
            <span class="meta-value mono text-secondary">{{ formatDate(store.selectedFile.modifiedAt) }}</span>
          </div>
        </div>
      </div>

      <!-- Encryption status -->
      <div v-if="store.selectedFile.encrypted" class="preview-section">
        <div class="section-label">
          <Lock :size="12" class="text-neon" /> Encryption
        </div>
        <div class="info-card neobrutalism-card" style="border-color: encryptionColor">
          <div class="info-row">
            <span class="info-key text-muted">Algorithm</span>
            <span class="badge" :style="{ color: encryptionColor, borderColor: encryptionColor, background: encryptionColor + '15' }">
              {{ store.selectedFile.encryptionAlgorithm?.toUpperCase() }}
            </span>
          </div>
          <div class="info-row">
            <span class="info-key text-muted">NIST Level</span>
            <span class="badge badge-cyan">L{{ encryptionNistLevel }}</span>
          </div>
          <div v-if="store.selectedFile.contextData?.keyId" class="info-row">
            <span class="info-key text-muted">Key ID</span>
            <span class="meta-value mono text-secondary">{{ String(store.selectedFile.contextData.keyId).substring(0, 16) }}…</span>
          </div>
        </div>
      </div>

      <!-- Compression status -->
      <div v-if="store.selectedFile.compressionLayers && store.selectedFile.compressionLayers[0] && store.selectedFile.compressionLayers[0] !== 'none'" class="preview-section">
        <div class="section-label">
          <Archive :size="12" class="text-gold" /> Compression
        </div>
        <div class="info-card neobrutalism-card" style="border-color: var(--cyber-saffron-gold)">
          <div class="info-row">
            <span class="info-key text-muted">Layer</span>
            <span class="badge badge-gold">{{ store.selectedFile.compressionLayers[0]?.toUpperCase() }}</span>
          </div>
          <div v-if="store.selectedFile.compressionLayers && store.selectedFile.compressionLayers.length > 0" class="info-row">
            <span class="info-key text-muted">Layers</span>
            <span class="meta-value mono">{{ store.selectedFile.compressionLayers.join(' → ') }}</span>
          </div>
          <div v-if="store.selectedFile.hashBlake3" class="info-row">
            <span class="info-key text-muted">BLAKE3</span>
            <span class="meta-value mono text-muted" style="font-size:10px; word-break:break-all">{{ store.selectedFile.hashBlake3.substring(0, 24) }}…</span>
          </div>
        </div>
      </div>

      <!-- Tags -->
      <div v-if="store.selectedFile.tags && store.selectedFile.tags.length > 0" class="preview-section">
        <div class="section-label">
          <Tag :size="12" class="text-cyan" /> Tags
        </div>
        <div class="tag-list">
          <span
            v-for="tag in store.selectedFile.tags"
            :key="tag"
            class="tag-item badge badge-cyan"
          >
            {{ tag }}
          </span>
        </div>
      </div>

      <!-- Face groups -->
      <div v-if="store.selectedFile.faceGroupIds && store.selectedFile.faceGroupIds.length > 0" class="preview-section">
        <div class="section-label">
          <Users :size="12" class="text-purple" /> Face Groups
        </div>
        <div class="face-list">
          <div
            v-for="groupId in store.selectedFile.faceGroupIds"
            :key="groupId"
            class="face-item"
          >
            <div class="avatar-sm" :style="{ borderColor: getFaceGroupColor(groupId) }">
              <Users :size="12" :style="{ color: getFaceGroupColor(groupId) }" />
            </div>
            <span class="face-name">{{ getFaceGroupName(groupId) }}</span>
          </div>
        </div>
      </div>

      <!-- GPS location -->
      <div v-if="store.selectedFile.gpsLat" class="preview-section">
        <div class="section-label">
          <MapPin :size="12" class="text-pink" /> Location
        </div>
        <div class="info-card neobrutalism-card">
          <div class="info-row">
            <span class="info-key text-muted">Coordinates</span>
            <span class="meta-value mono text-secondary" style="font-size:10px">
              {{ store.selectedFile.gpsLat.toFixed(4) }}, {{ store.selectedFile.gpsLon?.toFixed(4) }}
            </span>
          </div>
          <div v-if="store.selectedFile.gpsLon" class="info-row">
            <span class="info-key text-muted">Longitude</span>
            <span class="meta-value text-secondary" style="font-size:11px">{{ store.selectedFile.gpsLon.toFixed(4) }}</span>
          </div>
        </div>
      </div>

      <!-- Tree-sitter symbols -->
      <div v-if="store.parseResult && store.parseResult.symbols.length > 0" class="preview-section">
        <div class="section-label">
          <Code :size="12" class="text-cyan" /> Symbols
          <span class="section-sub text-muted">{{ store.parseResult.language }}</span>
        </div>
        <div class="symbol-tree">
          <SymbolNode
            v-for="sym in store.parseResult.symbols"
            :key="sym.name + sym.startLine"
            :symbol="sym"
            :depth="0"
          />
        </div>
      </div>

      <!-- Context preservation -->
      <div v-if="store.selectedFile.contextData && Object.keys(store.selectedFile.contextData).length > 0" class="preview-section">
        <div class="section-label">
          <Copy :size="12" class="text-gold" /> Context
        </div>
        <div class="info-card neobrutalism-card">
          <div v-for="(value, key) in filteredContextData" :key="String(key)" class="info-row">
            <span class="info-key text-muted">{{ key }}</span>
            <span class="meta-value mono text-secondary" style="font-size:10px">{{ String(value).substring(0, 30) }}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Action buttons -->
    <div class="preview-actions">
      <button class="action-btn" @click="handleEncrypt" title="Encrypt file">
        <Lock :size="14" :class="store.selectedFile.encrypted ? 'text-neon' : 'text-muted'" />
      </button>
      <button class="action-btn" @click="handleCompress" title="Compress file">
        <Archive :size="14" class="text-gold" />
      </button>
      <button class="action-btn" @click="handleStar" title="Star file">
        <Star :size="14" :class="store.selectedFile.isStarred ? 'text-gold' : 'text-muted'" />
      </button>
      <button class="action-btn" @click="handleCopyPath" title="Copy path">
        <Copy :size="14" class="text-secondary" />
      </button>
      <button class="action-btn action-btn-danger" @click="handleDelete" title="Delete file">
        <Trash2 :size="14" class="text-pink" />
      </button>
    </div>
  </aside>

  <!-- Empty state when nothing selected -->
  <aside v-else class="file-preview panel-right preview-empty">
    <div class="empty-content">
      <File :size="32" class="text-muted" />
      <span class="text-muted" style="font-size:12px">Select a file to preview</span>
    </div>
  </aside>
</template>

<script setup lang="ts">
import { computed, defineComponent, h } from 'vue'
import {
  File, Lock, Unlock, Archive, Star, MapPin, Users,
  Tag, Code, Trash2, Copy, ExternalLink, ChevronRight,
} from 'lucide-vue-next'
import { useAppStore } from '@/stores/app'
import { ENCRYPTION_INFO } from '@/types'
import type { EncryptionAlgo, CodeSymbol } from '@/types'

const store = useAppStore()

// Encryption helpers
const encryptionNistLevel = computed(() => {
  if (!store.selectedFile?.encryptionAlgorithm) return 0
  return ENCRYPTION_INFO[store.selectedFile.encryptionAlgorithm as EncryptionAlgo]?.nistLevel ?? 0
})

const encryptionColor = computed(() => {
  if (!store.selectedFile?.encryptionAlgorithm) return 'var(--cyber-matrix-green)'
  return ENCRYPTION_INFO[store.selectedFile.encryptionAlgorithm as EncryptionAlgo]?.color || 'var(--cyber-matrix-green)'
})

// Filter out internal context keys
const filteredContextData = computed(() => {
  if (!store.selectedFile?.contextData) return {}
  const { keyId, ...rest } = store.selectedFile.contextData
  return rest
})

// Face group helpers
function getFaceGroupColor(groupId: string): string {
  return store.faceGroups.find(fg => fg.id === groupId)?.color || '#6B7280'
}

function getFaceGroupName(groupId: string): string {
  return store.faceGroups.find(fg => fg.id === groupId)?.name || groupId
}

// Formatting
function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const k = 1024
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + units[i]
}

function formatDate(dateStr: string): string {
  if (!dateStr) return '—'
  const d = new Date(dateStr)
  return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric', hour: '2-digit', minute: '2-digit' })
}

// Actions
function handleEncrypt() {
  if (!store.selectedFile) return
  if (store.selectedFile.encrypted) return // Could add decrypt
  store.encryptFile(store.selectedFile.id, 'hybrid')
}

function handleCompress() {
  if (!store.selectedFile) return
  store.compressFile(store.selectedFile.id, 'zstd')
}

function handleStar() {
  if (!store.selectedFile) return
  store.toggleStar(store.selectedFile.id)
}

async function handleCopyPath() {
  if (!store.selectedFile?.path) return
  try {
    await navigator.clipboard.writeText(store.selectedFile.path)
  } catch {
    // fallback
  }
}

function handleDelete() {
  if (!store.selectedFile) return
  if (confirm(`Delete "${store.selectedFile.name}"?`)) {
    store.deleteFile(store.selectedFile.id)
  }
}

// Recursive symbol tree node (inline component)
const SymbolNode = defineComponent({
  name: 'SymbolNode',
  props: {
    symbol: { type: Object as () => CodeSymbol, required: true },
    depth: { type: Number, required: true },
  },
  setup(props) {
    const isExpanded = computed(() => props.depth < 2)
    return (): ReturnType<typeof h> => h('div', { class: 'symbol-node' }, [
      h('div', {
        class: 'symbol-row',
        style: { paddingLeft: `${props.depth * 14 + 4}px` },
      }, [
        h(ChevronRight, {
          size: 10,
          style: {
            transform: isExpanded.value ? 'rotate(90deg)' : 'rotate(0deg)',
            transition: 'transform 0.1s',
            flexShrink: 0,
          },
          class: 'text-muted',
        }),
        h('span', { class: 'symbol-kind text-muted' }, props.symbol.kind),
        h('span', { class: 'symbol-name mono' }, props.symbol.name),
        props.symbol.detail
          ? h('span', { class: 'symbol-detail text-muted' }, props.symbol.detail)
          : null,
      ]),
      isExpanded.value && props.symbol.children.length > 0
        ? props.symbol.children.map(child =>
            h(SymbolNode, { symbol: child, depth: props.depth + 1, key: child.name + child.startLine })
          )
        : null,
    ])
  },
})
</script>

<style scoped>
.file-preview {
  grid-area: preview;
  width: 350px;
  min-width: 300px;
  display: flex;
  flex-direction: column;
  background: var(--cyber-bg-panel);
  border-left: 3px solid #000000;
  overflow: hidden;
  z-index: 5;
}

.preview-empty {
  align-items: center;
  justify-content: center;
}

.empty-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  opacity: 0.5;
}

/* Header */
.preview-header {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 12px;
  border-bottom: 2px solid var(--cyber-bg-hover);
  flex-shrink: 0;
}

.preview-file-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.preview-filename {
  font-size: 13px;
  font-weight: 700;
  color: var(--cyber-text-primary);
}

.preview-path {
  font-size: 10px;
  color: var(--cyber-text-muted);
}

/* Scrollable content */
.preview-scroll {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 0 0 8px;
}

/* Sections */
.preview-section {
  padding: 10px 12px 0;
}

.section-label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-family: monospace;
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.8px;
  color: var(--cyber-text-muted);
  margin-bottom: 6px;
}

.section-sub {
  font-weight: 400;
  letter-spacing: 0;
  text-transform: none;
}

/* Metadata grid */
.metadata-grid {
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.meta-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
}

.meta-key {
  font-size: 11px;
  flex-shrink: 0;
}

.meta-value {
  font-size: 11px;
  color: var(--cyber-text-primary);
  text-align: right;
}

/* Info card */
.info-card {
  padding: 8px 10px;
  background: var(--cyber-bg-card);
  border: 2px solid #000000;
  border-radius: 2px;
}

.info-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
  padding: 2px 0;
}

.info-key {
  font-size: 11px;
  flex-shrink: 0;
}

/* Tags */
.tag-list {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.tag-item {
  cursor: pointer;
  font-size: 10px;
  transition: transform 0.1s;
}

.tag-item:hover {
  transform: translateY(-1px);
}

/* Face list */
.face-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.face-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 8px;
  border-radius: 2px;
  cursor: pointer;
  transition: background 0.1s;
}

.face-item:hover {
  background: var(--cyber-bg-hover);
}

.avatar-sm {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  border: 2px solid;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  background: var(--cyber-bg-deep);
}

.face-name {
  font-size: 12px;
  color: var(--cyber-text-primary);
}

/* Symbol tree */
.symbol-tree {
  border: 2px solid var(--cyber-bg-hover);
  border-radius: 2px;
  overflow: hidden;
}

.symbol-node {
  border-bottom: 1px solid rgba(37, 37, 64, 0.4);
}

.symbol-node:last-child {
  border-bottom: none;
}

.symbol-row {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 3px 6px;
  font-size: 11px;
  transition: background 0.1s;
  cursor: default;
}

.symbol-row:hover {
  background: var(--cyber-bg-hover);
}

.symbol-kind {
  font-family: monospace;
  font-size: 9px;
  color: var(--cyber-cyber-blue);
  text-transform: uppercase;
  flex-shrink: 0;
}

.symbol-name {
  color: var(--cyber-text-primary);
  font-size: 11px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.symbol-detail {
  font-size: 10px;
  color: var(--cyber-text-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* Action buttons */
.preview-actions {
  display: flex;
  gap: 2px;
  padding: 8px;
  border-top: 2px solid var(--cyber-bg-hover);
  flex-shrink: 0;
}

.action-btn {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  height: 32px;
  background: var(--cyber-bg-card);
  border: 2px solid #000000;
  border-radius: 2px;
  cursor: pointer;
  transition: all 0.1s;
}

.action-btn:hover {
  background: var(--cyber-bg-hover);
  border-color: var(--cyber-bg-hover);
}

.action-btn-danger:hover {
  border-color: var(--cyber-lotus-pink);
  background: rgba(255, 45, 111, 0.08);
}

/* Badge */
.badge {
  display: inline-flex;
  align-items: center;
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

.badge-cyan {
  color: var(--cyber-cyber-blue);
  border-color: var(--cyber-cyber-blue);
  background: rgba(0, 212, 255, 0.08);
}

/* Utilities */
.mono { font-family: 'Courier New', monospace; }
.text-neon { color: var(--cyber-matrix-green); }
.text-gold { color: var(--cyber-saffron-gold); }
.text-cyan { color: var(--cyber-cyber-blue); }
.text-pink { color: var(--cyber-lotus-pink); }
.text-purple { color: var(--cyber-cyber-purple); }
.text-secondary { color: var(--cyber-text-secondary); }
.text-muted { color: var(--cyber-text-muted); }
.truncate { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

.neobrutalism-card {
  background: var(--cyber-bg-card);
  border: 3px solid #000000;
  border-radius: 2px;
  box-shadow: 3px 3px 0 #000000;
}
</style>