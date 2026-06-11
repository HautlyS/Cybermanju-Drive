<!-- Cybermanju Drive — Sync Panel -->
<!-- Storage sync backends: Local, GitHub, Google Drive, Google Photos -->
<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useAppStore } from '@/stores/app'
import { CYBER, SYNC_BACKEND_INFO } from '@/types'
import type { SyncBackendType, SyncConfig } from '@/types'
import {
  HardDrive, Github, FolderSync, Camera, Plus, Trash2, Play, X,
  CheckCircle, AlertCircle, RefreshCw, Zap, Image, FileTrash2,
  CloudUpload, Wifi, WifiOff, Loader2,
} from 'lucide-vue-next'

const store = useAppStore()

// ── Backend icon map ──
const backendIcons: Record<SyncBackendType, any> = {
  local: HardDrive,
  github: Github,
  googleDrive: FolderSync,
  googlePhotos: Camera,
}

// ── Form state ──
const showForm = ref(false)
const formBackendType = ref<SyncBackendType>('local')
const formBasePath = ref('')
const formRepoName = ref('')
const formBranch = ref('main')
const formToken = ref('')
const formFolderId = ref('')
const formAlbumId = ref('')
const formAutoSync = ref(true)
const formCompressBeforeUpload = ref(true)
const formCreatePreviews = ref(false)
const formDeleteRawAfterSync = ref(false)
const formMaxConcurrentUploads = ref(4)

// ── Connection test state ──
const testingConnection = ref<string | null>(null)
const connectionResults = ref<Record<string, boolean | null>>({})

// ── Syncing state ──
const syncingConfigId = ref<string | null>(null)

// ── Sync status labels ──
const statusLabels: Record<string, { label: string; color: string }> = {
  idle: { label: 'IDLE', color: CYBER.textMuted },
  scanning: { label: 'SCANNING', color: CYBER.cyberBlue },
  compressing: { label: 'COMPRESSING', color: CYBER.neonYellow },
  uploading: { label: 'UPLOADING', color: CYBER.matrixGreen },
  linking: { label: 'LINKING', color: CYBER.cyberPurple },
  cleaning: { label: 'CLEANING', color: CYBER.templeOrange },
  error: { label: 'ERROR', color: CYBER.prayerRed },
  done: { label: 'DONE', color: CYBER.matrixGreen },
}

// ── Computed ──
const progressPercent = computed(() => {
  if (!store.syncProgress || store.syncProgress.totalFiles === 0) return 0
  return Math.round((store.syncProgress.processedFiles / store.syncProgress.totalFiles) * 100)
})

const isActive = computed(() => {
  if (!store.syncProgress) return false
  return ['scanning', 'compressing', 'uploading', 'linking', 'cleaning'].includes(store.syncProgress.status)
})

// ── Helpers ──
function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`
}

function formatDate(iso: string): string {
  if (!iso) return '—'
  try {
    return new Date(iso).toLocaleDateString(undefined, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })
  } catch {
    return iso
  }
}

function resetForm() {
  formBackendType.value = 'local'
  formBasePath.value = ''
  formRepoName.value = ''
  formBranch.value = 'main'
  formToken.value = ''
  formFolderId.value = ''
  formAlbumId.value = ''
  formAutoSync.value = true
  formCompressBeforeUpload.value = true
  formCreatePreviews.value = false
  formDeleteRawAfterSync.value = false
  formMaxConcurrentUploads.value = 4
}

function openForm() {
  resetForm()
  showForm.value = true
}

function closeForm() {
  showForm.value = false
}

async function submitForm() {
  const config: Omit<SyncConfig, 'id' | 'createdAt' | 'updatedAt'> = {
    backendType: formBackendType.value,
    enabled: true,
    basePath: formBasePath.value || undefined,
    repoName: formRepoName.value || undefined,
    branch: formBranch.value || undefined,
    token: formToken.value || undefined,
    folderId: formFolderId.value || undefined,
    albumId: formAlbumId.value || undefined,
    autoSync: formAutoSync.value,
    compressBeforeUpload: formCompressBeforeUpload.value,
    createPreviews: formCreatePreviews.value,
    deleteRawAfterSync: formDeleteRawAfterSync.value,
    maxConcurrentUploads: formMaxConcurrentUploads.value,
  }
  await store.createSyncConfig(config)
  showForm.value = false
}

async function handleDeleteConfig(configId: string) {
  await store.deleteSyncConfig(configId)
}

async function handleTestConnection(config: SyncConfig) {
  testingConnection.value = config.id
  connectionResults.value[config.id] = null
  const result = await store.testSyncConnection(config)
  connectionResults.value[config.id] = result
  testingConnection.value = null
}

async function handleStartSync(configId: string) {
  syncingConfigId.value = configId
  const fileIds = store.files.map(f => f.id)
  await store.startSync(configId, fileIds)
  syncingConfigId.value = null
}

async function handleCancelSync() {
  await store.cancelSync()
}

function getBackendLabel(type: SyncBackendType): string {
  return SYNC_BACKEND_INFO[type]?.name || type
}

function getBackendColor(type: SyncBackendType): string {
  return SYNC_BACKEND_INFO[type]?.color || CYBER.textMuted
}

onMounted(() => {
  store.fetchSyncConfigs()
  store.getSyncProgress()
})
</script>

<template>
  <div class="sync-panel">
    <!-- Header -->
    <div class="panel-header">
      <div class="header-left">
        <CloudUpload :size="22" class="icon-sync" />
        <h2 class="panel-title">Storage Sync</h2>
      </div>
    </div>

    <!-- Sync Progress (visible during active sync) -->
    <div v-if="store.syncProgress && isActive" class="sync-progress-card">
      <div class="progress-header">
        <Loader2 :size="16" class="spin-icon" />
        <span class="progress-label">Sync in Progress</span>
        <span
          class="status-badge"
          :style="{ background: statusLabels[store.syncProgress.status]?.color || CYBER.textMuted }"
        >
          {{ statusLabels[store.syncProgress.status]?.label || store.syncProgress.status.toUpperCase() }}
        </span>
      </div>

      <!-- Progress bar -->
      <div class="progress-bar-track">
        <div
          class="progress-bar-fill"
          :style="{ width: `${progressPercent}%` }"
        />
        <span class="progress-percent">{{ progressPercent }}%</span>
      </div>

      <!-- Progress details -->
      <div class="progress-details">
        <div class="progress-stat">
          <span class="stat-label">Files:</span>
          <span class="stat-value">{{ store.syncProgress.processedFiles }} / {{ store.syncProgress.totalFiles }}</span>
        </div>
        <div class="progress-stat">
          <span class="stat-label">Uploaded:</span>
          <span class="stat-value">{{ formatBytes(store.syncProgress.bytesUploaded) }}</span>
        </div>
        <div v-if="store.syncProgress.currentFile" class="progress-stat current-file">
          <span class="stat-label">Current:</span>
          <span class="stat-value truncate">{{ store.syncProgress.currentFile }}</span>
        </div>
        <div v-if="store.syncProgress.estimatedRemainingSeconds > 0" class="progress-stat">
          <span class="stat-label">ETA:</span>
          <span class="stat-value">{{ Math.ceil(store.syncProgress.estimatedRemainingSeconds) }}s</span>
        </div>
      </div>

      <!-- Errors -->
      <div v-if="store.syncProgress.errors.length" class="progress-errors">
        <AlertCircle :size="14" style="color: var(--prayer-red); flex-shrink: 0;" />
        <div class="error-list">
          <div v-for="(err, i) in store.syncProgress.errors" :key="i" class="error-item">
            {{ err }}
          </div>
        </div>
      </div>

      <!-- Cancel button -->
      <button class="neobrutalism-btn cancel-btn" @click="handleCancelSync">
        <X :size="14" />
        Cancel Sync
      </button>
    </div>

    <!-- Done status -->
    <div v-else-if="store.syncProgress && store.syncProgress.status === 'done'" class="sync-done-card">
      <CheckCircle :size="20" style="color: var(--matrix-green);" />
      <span class="done-label">Sync Complete</span>
      <span class="done-detail">
        {{ store.syncProgress.processedFiles }} files — {{ formatBytes(store.syncProgress.bytesUploaded) }} uploaded
      </span>
    </div>

    <!-- Error status -->
    <div v-else-if="store.syncProgress && store.syncProgress.status === 'error'" class="sync-error-card">
      <AlertCircle :size="20" style="color: var(--prayer-red);" />
      <span class="error-label">Sync Failed</span>
      <div v-for="(err, i) in store.syncProgress.errors" :key="i" class="error-detail">
        {{ err }}
      </div>
    </div>

    <!-- Add Backend Button -->
    <button class="neobrutalism-btn add-backend-btn" @click="openForm">
      <Plus :size="16" />
      Add Sync Backend
    </button>

    <!-- Backend Configs List -->
    <div class="configs-list">
      <div v-for="config in store.syncConfigs" :key="config.id" class="config-card neobrutalism-card">
        <!-- Card header -->
        <div class="config-header">
          <div class="config-left">
            <div class="config-icon-wrap" :style="{ borderColor: getBackendColor(config.backendType) }">
              <component
                :is="backendIcons[config.backendType]"
                :size="18"
                :style="{ color: getBackendColor(config.backendType) }"
              />
            </div>
            <div class="config-info">
              <div class="config-name">
                {{ getBackendLabel(config.backendType) }}
              </div>
              <div class="config-meta">
                <span class="config-type" :style="{ color: getBackendColor(config.backendType) }">
                  {{ config.backendType }}
                </span>
                <span class="config-path" v-if="config.basePath">{{ config.basePath }}</span>
                <span class="config-path" v-else-if="config.repoName">{{ config.repoName }} ({{ config.branch || 'main' }})</span>
              </div>
            </div>
          </div>
          <div class="config-right">
            <span
              class="enabled-badge"
              :class="config.enabled ? 'badge-on' : 'badge-off'"
            >
              {{ config.enabled ? 'ON' : 'OFF' }}
            </span>
          </div>
        </div>

        <!-- Card details -->
        <div class="config-details">
          <div class="detail-row">
            <span class="detail-label">Created:</span>
            <span class="detail-value">{{ formatDate(config.createdAt) }}</span>
          </div>
          <div class="detail-row">
            <span class="detail-label">Updated:</span>
            <span class="detail-value">{{ formatDate(config.updatedAt) }}</span>
          </div>
          <div class="detail-toggles">
            <span class="toggle-chip" :class="{ active: config.autoSync }">
              <Wifi :size="10" /> Auto
            </span>
            <span class="toggle-chip" :class="{ active: config.compressBeforeUpload }">
              <Zap :size="10" /> Compress
            </span>
            <span class="toggle-chip" :class="{ active: config.createPreviews }">
              <Image :size="10" /> Preview
            </span>
            <span class="toggle-chip danger" :class="{ active: config.deleteRawAfterSync }">
              <FileTrash2 :size="10" /> Delete Raw
            </span>
          </div>
        </div>

        <!-- Connection test result -->
        <div v-if="connectionResults[config.id] !== undefined" class="connection-result">
          <CheckCircle v-if="connectionResults[config.id]" :size="14" style="color: var(--matrix-green);" />
          <AlertCircle v-else :size="14" style="color: var(--prayer-red);" />
          <span :style="{ color: connectionResults[config.id] ? 'var(--matrix-green)' : 'var(--prayer-red)' }">
            {{ connectionResults[config.id] ? 'Connection OK' : 'Connection Failed' }}
          </span>
        </div>

        <!-- Card actions -->
        <div class="config-actions">
          <button
            class="neobrutalism-btn action-btn test-btn"
            :disabled="testingConnection === config.id"
            @click="handleTestConnection(config)"
          >
            <Loader2 v-if="testingConnection === config.id" :size="12" class="spin-icon" />
            <Wifi v-else :size="12" />
            Test
          </button>
          <button
            class="neobrutalism-btn action-btn sync-btn"
            :disabled="syncingConfigId === config.id || (store.syncProgress && isActive)"
            @click="handleStartSync(config.id)"
          >
            <Loader2 v-if="syncingConfigId === config.id" :size="12" class="spin-icon" />
            <Play v-else :size="12" />
            Start Sync
          </button>
          <button
            class="neobrutalism-btn action-btn delete-btn"
            @click="handleDeleteConfig(config.id)"
          >
            <Trash2 :size="12" />
          </button>
        </div>
      </div>

      <!-- Empty state -->
      <div v-if="store.syncConfigs.length === 0 && !showForm" class="empty-state">
        <CloudUpload :size="40" class="empty-icon" />
        <p class="empty-title">No sync backends configured</p>
        <p class="empty-desc">
          Add a backend to sync your files to Local storage, GitHub, Google Drive, or Google Photos.
        </p>
      </div>
    </div>

    <!-- Add Backend Form (modal-like overlay) -->
    <Teleport to="body">
      <div v-if="showForm" class="form-overlay" @click.self="closeForm">
        <div class="form-modal neobrutalism-card">
          <div class="form-header">
            <CloudUpload :size="18" class="form-icon" />
            <h3 class="form-title">Add Sync Backend</h3>
            <button class="close-btn" @click="closeForm">
              <X :size="16" />
            </button>
          </div>

          <!-- Backend type selector -->
          <div class="form-section">
            <label class="form-label">Backend Type</label>
            <div class="backend-selector">
              <button
                v-for="(info, type) in SYNC_BACKEND_INFO"
                :key="type"
                class="backend-option neobrutalism-btn"
                :class="{ selected: formBackendType === type }"
                :style="formBackendType === type ? { borderColor: info.color, color: info.color } : {}"
                @click="formBackendType = type as SyncBackendType"
              >
                <component :is="backendIcons[type as SyncBackendType]" :size="16" />
                <span>{{ info.name }}</span>
              </button>
            </div>
            <p class="form-hint">{{ SYNC_BACKEND_INFO[formBackendType]?.description }}</p>
          </div>

          <!-- Dynamic fields -->
          <div class="form-section">
            <!-- Local: base path -->
            <template v-if="formBackendType === 'local'">
              <label class="form-label">Base Path</label>
              <input
                v-model="formBasePath"
                type="text"
                class="form-input"
                placeholder="/home/user/sync-output"
              />
            </template>

            <!-- GitHub: repo + branch + token -->
            <template v-if="formBackendType === 'github'">
              <label class="form-label">Repository Name</label>
              <input
                v-model="formRepoName"
                type="text"
                class="form-input"
                placeholder="username/my-sync-repo"
              />
              <label class="form-label">Branch</label>
              <input
                v-model="formBranch"
                type="text"
                class="form-input"
                placeholder="main"
              />
              <label class="form-label">Personal Access Token</label>
              <input
                v-model="formToken"
                type="password"
                class="form-input"
                placeholder="ghp_xxxxxxxxxxxx"
              />
            </template>

            <!-- Google Drive: folder ID + token -->
            <template v-if="formBackendType === 'googleDrive'">
              <label class="form-label">Folder ID</label>
              <input
                v-model="formFolderId"
                type="text"
                class="form-input"
                placeholder="1aBcDeFgHiJkLmNoPqRsTuVwXyZ"
              />
              <label class="form-label">OAuth Token</label>
              <input
                v-model="formToken"
                type="password"
                class="form-input"
                placeholder="OAuth2 access token"
              />
            </template>

            <!-- Google Photos: album ID + token -->
            <template v-if="formBackendType === 'googlePhotos'">
              <label class="form-label">Album ID (optional)</label>
              <input
                v-model="formAlbumId"
                type="text"
                class="form-input"
                placeholder="Leave empty for default album"
              />
              <label class="form-label">OAuth Token</label>
              <input
                v-model="formToken"
                type="password"
                class="form-input"
                placeholder="OAuth2 access token"
              />
            </template>
          </div>

          <!-- Toggle options -->
          <div class="form-section">
            <label class="form-label">Options</label>
            <div class="toggle-list">
              <label class="toggle-row">
                <input type="checkbox" v-model="formAutoSync" />
                <span class="toggle-text">
                  <Wifi :size="12" />
                  Auto-sync on file changes
                </span>
              </label>
              <label class="toggle-row">
                <input type="checkbox" v-model="formCompressBeforeUpload" />
                <span class="toggle-text">
                  <Zap :size="12" />
                  Compress before upload
                </span>
              </label>
              <label class="toggle-row">
                <input type="checkbox" v-model="formCreatePreviews" />
                <span class="toggle-text">
                  <Image :size="12" />
                  Create preview thumbnails
                </span>
              </label>
              <label class="toggle-row danger">
                <input type="checkbox" v-model="formDeleteRawAfterSync" />
                <span class="toggle-text">
                  <FileTrash2 :size="12" />
                  Delete raw files after sync
                </span>
              </label>
            </div>
          </div>

          <!-- Concurrency -->
          <div class="form-section">
            <label class="form-label">Max Concurrent Uploads</label>
            <input
              v-model.number="formMaxConcurrentUploads"
              type="number"
              class="form-input"
              min="1"
              max="16"
            />
          </div>

          <!-- Form actions -->
          <div class="form-actions">
            <button class="neobrutalism-btn cancel-btn" @click="closeForm">
              <X :size="14" />
              Cancel
            </button>
            <button class="neobrutalism-btn submit-btn" @click="submitForm">
              <CheckCircle :size="14" />
              Create Backend
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.sync-panel {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  min-height: 0;
}

/* Header */
.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 4px;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 10px;
}

.icon-sync {
  color: var(--cyber-blue);
  filter: drop-shadow(0 0 6px rgba(0, 212, 255, 0.5));
}

.panel-title {
  font-family: monospace;
  font-size: 16px;
  font-weight: 700;
  color: var(--cyber-blue);
  text-transform: uppercase;
  letter-spacing: 1px;
  margin: 0;
}

/* Progress Card */
.sync-progress-card,
.sync-done-card,
.sync-error-card {
  padding: 16px;
  border: 3px solid #000;
  background: var(--cyber-bg-card);
  box-shadow: 4px 4px 0 #000;
}

.sync-progress-card {
  border-color: var(--cyber-blue);
  box-shadow: 4px 4px 0 rgba(0, 212, 255, 0.2);
}

.sync-done-card {
  border-color: var(--matrix-green);
  box-shadow: 4px 4px 0 rgba(0, 255, 65, 0.2);
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
}

.sync-error-card {
  border-color: var(--prayer-red);
  box-shadow: 4px 4px 0 rgba(220, 38, 38, 0.2);
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
}

.progress-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
}

.progress-label {
  font-family: monospace;
  font-size: 13px;
  font-weight: 600;
  color: var(--cyber-blue);
}

.status-badge {
  font-family: monospace;
  font-size: 10px;
  font-weight: 700;
  padding: 2px 8px;
  color: #000;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  border: 2px solid #000;
}

.done-label {
  font-family: monospace;
  font-size: 14px;
  font-weight: 700;
  color: var(--matrix-green);
}

.done-detail {
  font-family: monospace;
  font-size: 12px;
  color: var(--cyber-text-secondary);
  width: 100%;
}

.error-label {
  font-family: monospace;
  font-size: 14px;
  font-weight: 700;
  color: var(--prayer-red);
}

.error-detail {
  font-family: monospace;
  font-size: 12px;
  color: var(--cyber-text-secondary);
  width: 100%;
}

/* Progress bar */
.progress-bar-track {
  position: relative;
  height: 20px;
  background: var(--cyber-bg-deep);
  border: 2px solid #000;
  overflow: hidden;
  margin-bottom: 12px;
}

.progress-bar-fill {
  height: 100%;
  background: linear-gradient(90deg, var(--cyber-blue), var(--matrix-green));
  transition: width 0.5s ease;
  box-shadow: inset 0 0 10px rgba(0, 255, 65, 0.3);
}

.progress-percent {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  font-family: monospace;
  font-size: 11px;
  font-weight: 700;
  color: var(--cyber-text-primary);
  text-shadow: 0 0 4px rgba(0, 0, 0, 0.8);
}

.progress-details {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 6px;
  margin-bottom: 12px;
}

.progress-stat {
  display: flex;
  gap: 6px;
  font-family: monospace;
  font-size: 12px;
}

.progress-stat.current-file {
  grid-column: 1 / -1;
}

.stat-label {
  color: var(--cyber-text-muted);
}

.stat-value {
  color: var(--cyber-text-primary);
}

.truncate {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.progress-errors {
  display: flex;
  gap: 8px;
  padding: 8px;
  background: rgba(220, 38, 38, 0.1);
  border: 2px solid var(--prayer-red);
  margin-bottom: 12px;
}

.error-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
  flex: 1;
}

.error-item {
  font-family: monospace;
  font-size: 11px;
  color: var(--prayer-red);
}

.spin-icon {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

/* Add Button */
.add-backend-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  font-family: monospace;
  font-size: 13px;
  font-weight: 600;
  padding: 10px 16px;
  background: var(--cyber-bg-card);
  border: 3px solid var(--cyber-blue);
  color: var(--cyber-blue);
  cursor: pointer;
  transition: transform 0.1s, box-shadow 0.1s;
  box-shadow: 3px 3px 0 #000;
}

.add-backend-btn:hover {
  transform: translate(2px, 2px);
  box-shadow: 1px 1px 0 #000;
}

/* Config Cards */
.configs-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.config-card {
  padding: 16px;
  transition: transform 0.1s, box-shadow 0.1s;
}

.config-card:hover {
  transform: translate(2px, 2px);
  box-shadow: 2px 2px 0 #000 !important;
}

.config-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.config-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.config-icon-wrap {
  width: 40px;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 2px solid;
  border-radius: 4px;
  background: var(--cyber-bg-deep);
  flex-shrink: 0;
}

.config-info {
  min-width: 0;
}

.config-name {
  font-family: monospace;
  font-size: 14px;
  font-weight: 700;
  color: var(--cyber-text-primary);
}

.config-meta {
  display: flex;
  gap: 8px;
  margin-top: 2px;
}

.config-type {
  font-family: monospace;
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.config-path {
  font-family: monospace;
  font-size: 11px;
  color: var(--cyber-text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.config-right {
  flex-shrink: 0;
}

.enabled-badge {
  font-family: monospace;
  font-size: 10px;
  font-weight: 700;
  padding: 3px 10px;
  border: 2px solid #000;
  letter-spacing: 0.5px;
}

.badge-on {
  background: var(--matrix-green);
  color: #000;
}

.badge-off {
  background: var(--cyber-bg-hover);
  color: var(--cyber-text-muted);
}

/* Config details */
.config-details {
  margin-bottom: 12px;
}

.detail-row {
  display: flex;
  gap: 8px;
  font-family: monospace;
  font-size: 11px;
  margin-bottom: 2px;
}

.detail-label {
  color: var(--cyber-text-muted);
}

.detail-value {
  color: var(--cyber-text-secondary);
}

.detail-toggles {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 8px;
}

.toggle-chip {
  display: flex;
  align-items: center;
  gap: 4px;
  font-family: monospace;
  font-size: 10px;
  padding: 3px 8px;
  border: 1px solid var(--cyber-text-muted);
  color: var(--cyber-text-muted);
  opacity: 0.5;
  transition: all 0.15s;
}

.toggle-chip.active {
  opacity: 1;
  color: var(--matrix-green);
  border-color: var(--matrix-green);
}

.toggle-chip.danger.active {
  color: var(--prayer-red);
  border-color: var(--prayer-red);
}

/* Connection result */
.connection-result {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
  margin-bottom: 12px;
  border: 2px solid var(--cyber-bg-hover);
  background: var(--cyber-bg-deep);
  font-family: monospace;
  font-size: 12px;
}

/* Config actions */
.config-actions {
  display: flex;
  gap: 8px;
}

.action-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  font-family: monospace;
  font-size: 11px;
  font-weight: 600;
  padding: 6px 12px;
  cursor: pointer;
  transition: transform 0.1s, box-shadow 0.1s;
}

.action-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.test-btn {
  background: var(--cyber-bg-deep);
  border: 2px solid var(--cyber-blue);
  color: var(--cyber-blue);
  box-shadow: 2px 2px 0 #000;
}

.test-btn:hover:not(:disabled) {
  transform: translate(1px, 1px);
  box-shadow: 1px 1px 0 #000;
}

.sync-btn {
  background: var(--matrix-green);
  border: 2px solid #000;
  color: #000;
  box-shadow: 2px 2px 0 #000;
}

.sync-btn:hover:not(:disabled) {
  transform: translate(1px, 1px);
  box-shadow: 1px 1px 0 #000;
}

.delete-btn {
  background: var(--cyber-bg-deep);
  border: 2px solid var(--prayer-red);
  color: var(--prayer-red);
  box-shadow: 2px 2px 0 #000;
  margin-left: auto;
}

.delete-btn:hover:not(:disabled) {
  transform: translate(1px, 1px);
  box-shadow: 1px 1px 0 #000;
}

.cancel-btn {
  background: var(--cyber-bg-deep);
  border: 2px solid var(--prayer-red);
  color: var(--prayer-red);
  box-shadow: 2px 2px 0 #000;
  font-family: monospace;
  font-size: 12px;
  font-weight: 600;
  padding: 8px 16px;
  cursor: pointer;
  transition: transform 0.1s, box-shadow 0.1s;
  display: flex;
  align-items: center;
  gap: 6px;
}

.cancel-btn:hover {
  transform: translate(2px, 2px);
  box-shadow: 0 0 0 #000;
}

/* Empty state */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 40px 20px;
}

.empty-icon {
  color: var(--cyber-text-muted);
  opacity: 0.4;
}

.empty-title {
  font-family: monospace;
  font-size: 14px;
  color: var(--cyber-text-muted);
  font-weight: 600;
}

.empty-desc {
  font-size: 12px;
  color: var(--cyber-text-muted);
  text-align: center;
  max-width: 320px;
  line-height: 1.5;
}

/* ── Form Modal ── */
.form-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
  padding: 20px;
  backdrop-filter: blur(4px);
}

.form-modal {
  width: 520px;
  max-width: 100%;
  max-height: 90vh;
  overflow-y: auto;
  padding: 20px;
  border: 3px solid var(--cyber-blue);
  box-shadow: 6px 6px 0 #000;
  background: var(--cyber-bg-panel);
}

.form-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 20px;
  padding-bottom: 12px;
  border-bottom: 2px solid var(--cyber-bg-hover);
}

.form-icon {
  color: var(--cyber-blue);
  filter: drop-shadow(0 0 6px rgba(0, 212, 255, 0.5));
}

.form-title {
  font-family: monospace;
  font-size: 14px;
  font-weight: 700;
  color: var(--cyber-blue);
  text-transform: uppercase;
  letter-spacing: 1px;
  margin: 0;
  flex: 1;
}

.form-header .close-btn {
  background: var(--cyber-bg-card);
  border: 2px solid #000;
  color: var(--cyber-text-muted);
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.1s;
}

.form-header .close-btn:hover {
  color: var(--prayer-red);
  border-color: var(--prayer-red);
}

/* Form sections */
.form-section {
  margin-bottom: 16px;
}

.form-label {
  display: block;
  font-family: monospace;
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--cyber-text-muted);
  margin-bottom: 6px;
}

.form-input {
  width: 100%;
  padding: 8px 12px;
  font-family: monospace;
  font-size: 13px;
  background: var(--cyber-bg-deep);
  border: 2px solid var(--cyber-bg-hover);
  color: var(--cyber-text-primary);
  outline: none;
  transition: border-color 0.15s;
  margin-bottom: 8px;
  box-sizing: border-box;
}

.form-input:focus {
  border-color: var(--cyber-blue);
  box-shadow: 0 0 6px rgba(0, 212, 255, 0.2);
}

.form-input::placeholder {
  color: var(--cyber-text-muted);
  opacity: 0.5;
}

.form-hint {
  font-size: 11px;
  color: var(--cyber-text-secondary);
  line-height: 1.4;
  margin-top: 4px;
}

/* Backend type selector */
.backend-selector {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
  margin-bottom: 8px;
}

.backend-option {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 10px 8px;
  font-family: monospace;
  font-size: 11px;
  font-weight: 600;
  background: var(--cyber-bg-deep);
  border: 2px solid var(--cyber-bg-hover);
  color: var(--cyber-text-secondary);
  cursor: pointer;
  transition: all 0.15s;
}

.backend-option.selected {
  background: rgba(0, 212, 255, 0.08);
}

.backend-option:hover {
  border-color: var(--cyber-blue);
}

/* Toggles */
.toggle-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.toggle-row {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  font-size: 12px;
}

.toggle-row input[type="checkbox"] {
  accent-color: var(--matrix-green);
  width: 14px;
  height: 14px;
}

.toggle-text {
  display: flex;
  align-items: center;
  gap: 6px;
  font-family: monospace;
  font-size: 12px;
  color: var(--cyber-text-secondary);
}

.toggle-row.danger .toggle-text {
  color: var(--prayer-red);
}

/* Form actions */
.form-actions {
  display: flex;
  gap: 12px;
  padding-top: 16px;
  border-top: 2px solid var(--cyber-bg-hover);
}

.form-actions .cancel-btn {
  flex: 0;
}

.submit-btn {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  font-family: monospace;
  font-size: 13px;
  font-weight: 700;
  padding: 10px 16px;
  background: var(--cyber-blue);
  border: 2px solid #000;
  color: #000;
  cursor: pointer;
  transition: transform 0.1s, box-shadow 0.1s;
  box-shadow: 3px 3px 0 #000;
}

.submit-btn:hover {
  transform: translate(2px, 2px);
  box-shadow: 1px 1px 0 #000;
}

/* Scrollbar */
.sync-panel::-webkit-scrollbar,
.form-modal::-webkit-scrollbar {
  width: 6px;
}

.sync-panel::-webkit-scrollbar-track,
.form-modal::-webkit-scrollbar-track {
  background: var(--cyber-bg-deep);
}

.sync-panel::-webkit-scrollbar-thumb,
.form-modal::-webkit-scrollbar-thumb {
  background: var(--cyber-matrix-dark-green);
  border: 1px solid #000;
}
</style>
