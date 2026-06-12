<template>
  <div class="face-panel">
    <!-- Header -->
    <div class="panel-header">
      <div class="header-left">
        <Users :size="22" class="icon-users" />
        <h2 class="panel-title">People</h2>
        <span class="face-badge" v-if="faceGroups.length">{{ faceGroups.length }}</span>
      </div>
      <button class="close-btn" @click="$emit('close')">
        <span class="close-x">✕</span>
      </button>
    </div>

    <!-- Action Buttons -->
    <div class="action-row">
      <button class="action-btn primary" @click="handleBatchDetect" :disabled="isProcessing">
        <Scan :size="14" />
        {{ isProcessing ? 'Scanning...' : 'Scan All Photos' }}
      </button>
      <button class="action-btn secondary" @click="handleRecluster" :disabled="isProcessing || faceGroups.length === 0">
        <RefreshCw :size="14" />
        Re-cluster
      </button>
    </div>

    <!-- Clustering Strategy Selector -->
    <div class="strategy-selector" v-if="showStrategySelector">
      <label class="strategy-label">Algorithm:</label>
      <div class="strategy-options">
        <button
          v-for="s in strategies"
          :key="s.value"
          class="strategy-btn"
          :class="{ active: selectedStrategy === s.value }"
          @click="selectedStrategy = s.value"
        >
          {{ s.label }}
        </button>
      </div>
      <button class="action-btn primary small" @click="executeRecluster">
        Run
      </button>
    </div>

    <!-- Stats Bar -->
    <div class="stats-bar" v-if="lastResult">
      <div class="stat">
        <span class="stat-value">{{ lastResult.clustersCreated }}</span>
        <span class="stat-label">Groups</span>
      </div>
      <div class="stat">
        <span class="stat-value">{{ lastResult.totalFaces }}</span>
        <span class="stat-label">Faces</span>
      </div>
      <div class="stat">
        <span class="stat-value">{{ lastResult.noiseFaces }}</span>
        <span class="stat-label">Noise</span>
      </div>
      <div class="stat">
        <span class="stat-value">{{ (lastResult.avgCohesion * 100).toFixed(0) }}%</span>
        <span class="stat-label">Cohesion</span>
      </div>
    </div>

    <!-- Face Groups Grid -->
    <div class="section" v-if="faceGroups.length > 0">
      <h3 class="section-title">
        <UserCircle :size="16" />
        Detected People ({{ faceGroups.length }})
      </h3>
      <div class="face-grid">
        <div
          v-for="group in faceGroups"
          :key="group.id"
          class="face-card"
          :class="{ selected: selectedGroupId === group.id }"
          @click="handleSelectGroup(group)"
          @dblclick="startRename(group)"
        >
          <div
            class="face-avatar"
            :style="{
              backgroundColor: getGroupColor(group.id),
              boxShadow: `0 0 12px ${getGroupColor(group.id)}44`,
            }"
          >
            <UserCircle :size="36" class="avatar-icon" />
          </div>
          <span class="face-name">{{ group.name }}</span>
          <span class="face-count">
            <Camera :size="10" />
            {{ group.fileIds.length }} faces
          </span>
          <span class="face-cohesion" v-if="group.cohesion != null">
            {{ (group.cohesion * 100).toFixed(0) }}% match
          </span>
        </div>
      </div>

      <!-- Detailed List -->
      <div class="face-list">
        <div
          v-for="group in faceGroups"
          :key="'list-' + group.id"
          class="face-list-item"
          :class="{ selected: selectedGroupId === group.id }"
          :style="{ borderLeftColor: getGroupColor(group.id) }"
          @click="handleSelectGroup(group)"
        >
          <div
            class="face-list-avatar"
            :style="{ backgroundColor: getGroupColor(group.id) }"
          >
            <UserCircle :size="20" class="avatar-icon-sm" />
          </div>
          <div class="face-list-info">
            <span class="face-list-name">{{ group.name }}</span>
            <span class="face-list-meta">
              {{ group.fileIds.length }} files
              <span v-if="group.algorithm" class="face-algo-badge">{{ group.algorithm }}</span>
            </span>
          </div>
          <div class="face-list-badge">
            {{ group.fileIds.length }}
          </div>
          <div class="face-list-actions">
            <button class="icon-btn" @click.stop="startRename(group)" title="Rename">
              <Pencil :size="12" />
            </button>
            <button class="icon-btn" @click.stop="handleFindSimilar(group)" title="Find Similar">
              <Search :size="12" />
            </button>
            <button class="icon-btn danger" @click.stop="handleDeleteGroup(group)" title="Delete">
              <Trash2 :size="12" />
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Rename Dialog -->
    <div class="rename-dialog" v-if="renamingGroup">
      <div class="rename-header">
        <Pencil :size="14" />
        Rename {{ renamingGroup.name }}
      </div>
      <input
        ref="renameInput"
        v-model="renameValue"
        class="rename-input"
        @keyup.enter="confirmRename"
        @keyup.escape="cancelRename"
        placeholder="Enter name..."
      />
      <div class="rename-actions">
        <button class="action-btn secondary small" @click="cancelRename">Cancel</button>
        <button class="action-btn primary small" @click="confirmRename">Save</button>
      </div>
    </div>

    <!-- Merge Dialog -->
    <div class="merge-dialog" v-if="selectedGroupId && mergeMode">
      <div class="merge-header">
        <GitMerge :size="14" />
        Merge "{{ getSelectedGroupName }}" into:
      </div>
      <div class="merge-targets">
        <button
          v-for="group in faceGroups.filter(g => g.id !== selectedGroupId)"
          :key="group.id"
          class="merge-target-btn"
          @click="confirmMerge(group.id)"
        >
          <UserCircle :size="14" />
          {{ group.name }} ({{ group.fileIds.length }})
        </button>
      </div>
      <button class="action-btn secondary small" @click="mergeMode = false">Cancel</button>
    </div>

    <!-- Similar Faces -->
    <div class="similar-panel" v-if="similarFaces.length > 0">
      <div class="similar-header">
        <Search :size="14" />
        Similar Faces Found: {{ similarFaces.length }}
      </div>
      <div class="similar-list">
        <div v-for="sf in similarFaces" :key="sf.id" class="similar-item">
          <UserCircle :size="16" />
          <span>{{ sf.name }}</span>
          <span class="similar-count">{{ sf.fileIds.length }} files</span>
          <button class="action-btn secondary tiny" @click="confirmMergeInto(sf.id)">
            Merge
          </button>
        </div>
      </div>
      <button class="action-btn secondary small" @click="similarFaces = []">Close</button>
    </div>

    <!-- Empty State -->
    <div class="empty-state" v-if="faceGroups.length === 0 && !isProcessing">
      <Brain :size="40" class="empty-icon" />
      <p>No face groups detected yet.</p>
      <p class="empty-hint">Click "Scan All Photos" to detect and cluster faces.</p>
    </div>

    <!-- Processing Indicator -->
    <div class="processing-indicator" v-if="isProcessing">
      <Loader2 :size="20" class="spin" />
      <span>Detecting faces and clustering...</span>
    </div>

    <!-- Status Footer -->
    <div class="status-footer">
      <span>SCRFD detection + ArcFace 512-d embeddings + HDBSCAN clustering</span>
      <span class="footer-sep">|</span>
      <span>SimHash binary codes + adaptive threshold + medoid centroids</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, nextTick } from 'vue'
import { useAppStore } from '@/stores/app'
import type { FaceGroup } from '@/types'
import {
  Users,
  Scan,
  UserCircle,
  Camera,
  Brain,
  RefreshCw,
  Pencil,
  Search,
  Trash2,
  GitMerge,
  Loader2,
} from 'lucide-vue-next'

const store = useAppStore()
const emit = defineEmits<{ close: [] }>()

const faceGroups = computed(() => store.faceGroups)
const isProcessing = ref(false)
const lastResult = ref<{ clustersCreated: number; totalFaces: number; noiseFaces: number; avgCohesion: number; strategyUsed: string } | null>(null)

// Group selection
const selectedGroupId = ref<string | null>(null)
const mergeMode = ref(false)

// Rename
const renamingGroup = ref<FaceGroup | null>(null)
const renameValue = ref('')
const renameInput = ref<HTMLInputElement | null>(null)

// Similar faces
const similarFaces = ref<FaceGroup[]>([])

// Strategy
const showStrategySelector = ref(false)
const selectedStrategy = ref('hdbscan')
const strategies = [
  { value: 'hdbscan', label: 'HDBSCAN' },
  { value: 'chinese_whispers', label: 'Chinese Whispers' },
  { value: 'bruteforce', label: 'Brute Force' },
  { value: 'simhash', label: 'SimHash' },
]

// Color palette for groups
const GROUP_COLORS = [
  '#A855F7', '#00D4FF', '#FF2D6F', '#FFB800', '#16A34A',
  '#FF6B2B', '#DC2626', '#00FF41', '#F59E0B', '#8B5CF6',
  '#EC4899', '#06B6D4', '#84CC16', '#F97316', '#6366F1',
]

function getGroupColor(id: string): string {
  const idx = id.split('').reduce((acc, c) => acc + c.charCodeAt(0), 0) % GROUP_COLORS.length
  return GROUP_COLORS[idx]
}

const selectedGroupName = computed(() => {
  if (!selectedGroupId.value) return ''
  return faceGroups.value.find(g => g.id === selectedGroupId.value)?.name || ''
})

const getSelectedGroupName = computed(() => selectedGroupName.value)

// Actions
async function handleBatchDetect() {
  isProcessing.value = true
  try {
    const result = await store.detectFacesBatch()
    if (result) lastResult.value = result
  } finally {
    isProcessing.value = false
  }
}

function handleRecluster() {
  showStrategySelector.value = !showStrategySelector.value
}

async function executeRecluster() {
  isProcessing.value = true
  showStrategySelector.value = false
  try {
    const result = await store.reclusterFaces(selectedStrategy.value)
    if (result) lastResult.value = result
  } finally {
    isProcessing.value = false
  }
}

function handleSelectGroup(group: FaceGroup) {
  if (selectedGroupId.value === group.id) {
    // Toggle merge mode on second click
    mergeMode.value = !mergeMode.value
  } else {
    selectedGroupId.value = group.id
    mergeMode.value = false
    similarFaces.value = []
  }
}

function startRename(group: FaceGroup) {
  renamingGroup.value = group
  renameValue.value = group.name
  nextTick(() => {
    renameInput.value?.focus()
    renameInput.value?.select()
  })
}

function cancelRename() {
  renamingGroup.value = null
  renameValue.value = ''
}

async function confirmRename() {
  if (!renamingGroup.value || !renameValue.value.trim()) return
  await store.renameFaceGroup(renamingGroup.value.id, renameValue.value.trim())
  cancelRename()
}

async function handleDeleteGroup(group: FaceGroup) {
  if (confirm(`Delete "${group.name}"? This won't delete the actual files.`)) {
    await store.deleteFaceGroup(group.id)
    if (selectedGroupId.value === group.id) {
      selectedGroupId.value = null
    }
  }
}

async function handleFindSimilar(group: FaceGroup) {
  const similar = await store.findSimilarFaces(group.id, 0.6)
  similarFaces.value = similar
}

async function confirmMerge(targetGroupId: string) {
  if (!selectedGroupId.value) return
  if (confirm(`Merge "${selectedGroupName.value}" into this group?`)) {
    await store.mergeFaceGroups(selectedGroupId.value, targetGroupId)
    selectedGroupId.value = null
    mergeMode.value = false
  }
}

async function confirmMergeInto(targetGroupId: string) {
  if (!selectedGroupId.value) return
  await store.mergeFaceGroups(selectedGroupId.value, targetGroupId)
  selectedGroupId.value = null
  similarFaces.value = []
}
</script>

<style scoped>
.face-panel {
  width: 420px;
  height: 100%;
  background: var(--cyber-bg-panel, #12121a);
  border-left: 3px solid #000;
  box-shadow: -4px 0 0 0 #000;
  overflow-y: auto;
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  font-family: 'Inter', system-ui, sans-serif;
  color: #F5F5F4;
}

.face-panel::-webkit-scrollbar { width: 6px; }
.face-panel::-webkit-scrollbar-track { background: #0a0a0f; }
.face-panel::-webkit-scrollbar-thumb { background: #333; border-radius: 3px; }

/* Header */
.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-bottom: 12px;
  border-bottom: 3px solid #000;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 10px;
}

.icon-users {
  color: #A855F7;
  filter: drop-shadow(0 0 6px #A855F7);
}

.panel-title {
  font-size: 18px;
  font-weight: 800;
  text-transform: uppercase;
  letter-spacing: 2px;
  color: #A855F7;
  text-shadow: 0 0 10px #A855F7, 0 0 20px rgba(168, 85, 247, 0.3);
  margin: 0;
}

.face-badge {
  background: #A855F7;
  color: #0a0a0f;
  font-size: 11px;
  font-weight: 800;
  padding: 2px 8px;
  border-radius: 2px;
  border: 2px solid #000;
}

.close-btn {
  background: none;
  border: 2px solid #333;
  color: #9CA3AF;
  cursor: pointer;
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
  transition: all 0.15s;
}
.close-btn:hover { border-color: #FF2D6F; color: #FF2D6F; }

/* Action Row */
.action-row {
  display: flex;
  gap: 8px;
}

.action-btn {
  flex: 1;
  padding: 8px 12px;
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  border: 3px solid #000;
  box-shadow: 3px 3px 0 0 #000;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  transition: all 0.15s;
}
.action-btn:hover:not(:disabled) {
  transform: translate(-1px, -1px);
  box-shadow: 4px 4px 0 0 #000;
}
.action-btn:active:not(:disabled) {
  transform: translate(1px, 1px);
  box-shadow: 2px 2px 0 0 #000;
}
.action-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.action-btn.primary { background: #A855F7; color: #0a0a0f; }
.action-btn.secondary { background: #1a1a2e; color: #F5F5F4; }
.action-btn.small { padding: 4px 10px; font-size: 10px; flex: none; }
.action-btn.tiny { padding: 2px 6px; font-size: 9px; flex: none; }

/* Strategy Selector */
.strategy-selector {
  background: #1a1a2e;
  border: 2px solid #333;
  padding: 10px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.strategy-label {
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: #9CA3AF;
}

.strategy-options {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
}

.strategy-btn {
  background: #0a0a0f;
  border: 2px solid #333;
  color: #6B7280;
  font-size: 10px;
  font-weight: 600;
  padding: 4px 8px;
  cursor: pointer;
  transition: all 0.15s;
}
.strategy-btn.active {
  background: #A855F7;
  color: #0a0a0f;
  border-color: #A855F7;
}

/* Stats Bar */
.stats-bar {
  display: flex;
  gap: 8px;
}

.stat {
  flex: 1;
  background: #1a1a2e;
  border: 2px solid #333;
  padding: 8px;
  text-align: center;
}

.stat-value {
  display: block;
  font-size: 18px;
  font-weight: 800;
  color: #00FF41;
  text-shadow: 0 0 6px rgba(0, 255, 65, 0.4);
}

.stat-label {
  font-size: 9px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: #6B7280;
}

/* Sections */
.section {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.section-title {
  font-size: 12px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 1.5px;
  color: #9CA3AF;
  margin: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  padding-bottom: 6px;
  border-bottom: 2px solid #1a1a2e;
}

/* Face Grid */
.face-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 10px;
}

.face-card {
  background: #1a1a2e;
  border: 3px solid #000;
  box-shadow: 3px 3px 0 0 #000;
  padding: 12px 6px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  transition: all 0.15s;
}
.face-card:hover {
  transform: translate(-1px, -1px);
  box-shadow: 4px 4px 0 0 #000;
}
.face-card.selected {
  border-color: #A855F7;
  box-shadow: 0 0 12px rgba(168, 85, 247, 0.4);
}

.face-avatar {
  width: 52px;
  height: 52px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 3px solid #000;
}

.avatar-icon { color: rgba(0, 0, 0, 0.4); }

.face-name {
  font-size: 10px;
  font-weight: 700;
  text-align: center;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  width: 100%;
  color: #F5F5F4;
}

.face-count {
  display: flex;
  align-items: center;
  gap: 3px;
  font-size: 9px;
  color: #6B7280;
}

.face-cohesion {
  font-size: 8px;
  color: #00FF41;
  font-weight: 600;
}

/* Face List */
.face-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.face-list-item {
  background: #1a1a2e;
  border: 3px solid #000;
  border-left: 5px solid;
  box-shadow: 3px 3px 0 0 #000;
  padding: 8px 10px;
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  transition: all 0.15s;
}
.face-list-item:hover {
  transform: translate(-1px, -1px);
  box-shadow: 4px 4px 0 0 #000;
}
.face-list-item.selected {
  border-color: #A855F7;
}

.face-list-avatar {
  width: 30px;
  height: 30px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 2px solid #000;
  flex-shrink: 0;
}

.avatar-icon-sm { color: rgba(0, 0, 0, 0.4); }

.face-list-info {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.face-list-name {
  font-size: 12px;
  font-weight: 700;
}

.face-list-meta {
  font-size: 10px;
  color: #6B7280;
  display: flex;
  align-items: center;
  gap: 6px;
}

.face-algo-badge {
  background: #0a0a0f;
  border: 1px solid #333;
  padding: 0 4px;
  font-size: 8px;
  border-radius: 2px;
  text-transform: uppercase;
}

.face-list-badge {
  background: #0a0a0f;
  border: 2px solid #333;
  color: #9CA3AF;
  font-size: 10px;
  font-weight: 700;
  padding: 2px 6px;
  border-radius: 2px;
}

.face-list-actions {
  display: flex;
  gap: 2px;
}

.icon-btn {
  background: none;
  border: 1px solid transparent;
  color: #6B7280;
  cursor: pointer;
  padding: 3px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
}
.icon-btn:hover { color: #F5F5F4; border-color: #333; }
.icon-btn.danger:hover { color: #FF2D6F; border-color: #FF2D6F; }

/* Rename Dialog */
.rename-dialog {
  background: #1a1a2e;
  border: 3px solid #A855F7;
  box-shadow: 3px 3px 0 0 #000;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.rename-header {
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: #A855F7;
  display: flex;
  align-items: center;
  gap: 6px;
}

.rename-input {
  background: #0a0a0f;
  border: 2px solid #333;
  color: #F5F5F4;
  font-size: 13px;
  font-weight: 600;
  padding: 6px 10px;
  outline: none;
}
.rename-input:focus { border-color: #A855F7; }

.rename-actions {
  display: flex;
  gap: 6px;
  justify-content: flex-end;
}

/* Merge Dialog */
.merge-dialog {
  background: #1a1a2e;
  border: 3px solid #FFB800;
  box-shadow: 3px 3px 0 0 #000;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.merge-header {
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: #FFB800;
  display: flex;
  align-items: center;
  gap: 6px;
}

.merge-targets {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.merge-target-btn {
  background: #0a0a0f;
  border: 2px solid #333;
  color: #F5F5F4;
  font-size: 11px;
  font-weight: 600;
  padding: 6px 10px;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 6px;
  transition: all 0.15s;
  text-align: left;
}
.merge-target-btn:hover {
  border-color: #FFB800;
  background: #252540;
}

/* Similar Panel */
.similar-panel {
  background: #1a1a2e;
  border: 3px solid #00D4FF;
  box-shadow: 3px 3px 0 0 #000;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.similar-header {
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: #00D4FF;
  display: flex;
  align-items: center;
  gap: 6px;
}

.similar-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.similar-item {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
}

.similar-count {
  font-size: 10px;
  color: #6B7280;
  margin-left: auto;
}

/* Empty State */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  padding: 40px 20px;
  text-align: center;
}

.empty-icon { color: #333; }

.empty-state p {
  font-size: 13px;
  color: #6B7280;
  margin: 0;
  line-height: 1.6;
}

.empty-hint {
  font-size: 11px;
  color: #4B5563;
}

/* Processing Indicator */
.processing-indicator {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 16px;
  color: #A855F7;
  font-size: 12px;
  font-weight: 600;
}

.spin {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

/* Footer */
.status-footer {
  margin-top: auto;
  padding-top: 12px;
  border-top: 2px solid #1a1a2e;
  font-size: 9px;
  color: #4B5563;
  text-align: center;
  letter-spacing: 0.3px;
  line-height: 1.6;
}

.footer-sep {
  margin: 0 4px;
  color: #333;
}
</style>
