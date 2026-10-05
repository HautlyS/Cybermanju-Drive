<template>
  <div class="sync-panel">
    <div class="panel-header">
      <div class="header-left">
        <span class="icon-sync">[~]</span>
        <h2 class="panel-title">STORAGE SYNC</h2>
      </div>
      <button class="bw-btn small" @click="showWizard = !showWizard">{{ showWizard ? 'CLOSE' : '+ CONFIG' }}</button>
    </div>

    <!-- Config wizard -->
    <div v-if="showWizard" class="section wizard">
      <h3 class="section-title">[NEW] PROVIDER CONFIG</h3>
      <label class="w-label">BACKEND
        <select v-model="form.backendType" class="bw-input">
          <option v-for="(info, key) in SYNC_BACKEND_INFO" :key="key" :value="key">{{ info.name }}</option>
        </select>
      </label>
      <label class="w-label">NAME <input v-model="form.name" class="bw-input" placeholder="MY PROVIDER" /></label>
      <label v-if="needsBasePath" class="w-label">LOCAL PATH <input v-model="form.basePath" class="bw-input" placeholder="/DATA/SYNC" /></label>
      <label v-if="needsRepo" class="w-label">REPO (owner/repo) <input v-model="form.repoName" class="bw-input" placeholder="OWNER/REPO" /></label>
      <label v-if="needsRepo" class="w-label">BRANCH <input v-model="form.branch" class="bw-input" placeholder="MAIN" /></label>
      <label v-if="needsFolder" class="w-label">DRIVE FOLDER ID <input v-model="form.folderId" class="bw-input" placeholder="FOLDER ID" /></label>
      <label v-if="needsAlbum" class="w-label">PHOTOS ALBUM ID <input v-model="form.albumId" class="bw-input" placeholder="ALBUM ID" /></label>
      <label v-if="needsChat" class="w-label">TELEGRAM CHAT ID <input v-model="form.chatId" class="bw-input" placeholder="CHAT ID" /></label>
      <label class="w-label">TOKEN (PAT / OAuth, never shown back)
        <input v-model="form.token" type="password" class="bw-input" placeholder="PASTE TOKEN" autocomplete="off" />
      </label>
      <div class="w-row">
        <label class="w-check"><input type="checkbox" v-model="form.enabled" /> ENABLED</label>
        <label class="w-check"><input type="checkbox" v-model="form.encryptBeforeUpload" /> ENCRYPT</label>
        <label class="w-check"><input type="checkbox" v-model="form.compressBeforeUpload" /> COMPRESS</label>
      </div>
      <div class="w-row">
        <label class="w-label">PLACEMENT
          <select v-model="form.placement" class="bw-input">
            <option value="whole">WHOLE</option>
            <option value="striped">STRIPED (≥2 providers)</option>
          </select>
        </label>
        <label class="w-label">POLICY
          <select v-model="form.conflictPolicy" class="bw-input">
            <option value="skip">SKIP</option>
            <option value="overwrite">OVERWRITE</option>
            <option value="keepBoth">KEEP-BOTH</option>
          </select>
        </label>
      </div>
      <div class="w-actions">
        <button class="bw-btn small" :disabled="busy" @click="testCurrent">TEST</button>
        <button class="bw-btn small primary" :disabled="busy" @click="saveConfig">SAVE</button>
        <button v-if="oauthable" class="bw-btn small" :disabled="busy" @click="oauthConnect">OAUTH CONNECT</button>
      </div>
      <div v-if="testMsg" class="w-msg">{{ testMsg }}</div>
    </div>

    <div class="section">
      <h3 class="section-title">[CFG] SYNC CONFIGS ({{ syncConfigs.length }})</h3>
      <div class="config-list">
        <div v-for="cfg in syncConfigs" :key="cfg.id" class="config-card">
          <div class="cfg-header">
            <span class="cfg-name">{{ cfg.name || cfg.backendType }}</span>
            <span class="cfg-type text-muted">{{ cfg.backendType }}</span>
            <span class="cfg-status" :class="{ on: cfg.enabled }">{{ cfg.enabled ? 'ON' : 'OFF' }}</span>
          </div>
          <div class="cfg-meta text-muted">
            <span v-if="cfg.basePath">PATH: {{ cfg.basePath }}</span>
            <span v-if="cfg.repoName">REPO: {{ cfg.repoName }}</span>
            <span v-if="cfg.placement"> · {{ cfg.placement }}</span>
          </div>
          <div class="cfg-actions">
            <button class="bw-btn xs" @click="testCfg(cfg)">TEST</button>
            <button class="bw-btn xs" @click="startCfg(cfg)">START</button>
            <button class="bw-btn xs" @click="usageCfg(cfg)">QUOTA</button>
            <button class="bw-btn xs danger" @click="removeCfg(cfg.id)">DEL</button>
          </div>
          <div v-if="quotaMsg[cfg.id]" class="cfg-msg">{{ quotaMsg[cfg.id] }}</div>
        </div>
      </div>
      <div v-if="!syncConfigs.length" class="empty text-muted">No providers connected — add one above.</div>
    </div>

    <div class="section">
      <h3 class="section-title">[RUN] START / MONITOR</h3>
      <div class="w-row">
        <select v-model="runConfigId" class="bw-input">
          <option value="">SELECT CONFIG</option>
          <option v-for="c in syncConfigs" :key="c.id" :value="c.id">{{ c.name || c.backendType }}</option>
        </select>
        <button class="bw-btn small" :disabled="!runConfigId || busy" @click="startRun">START</button>
        <button class="bw-btn small" @click="cancelRun">CANCEL</button>
        <button class="bw-btn small" @click="refreshRuns">RUNS</button>
      </div>
      <div v-if="jobMsg" class="w-msg">{{ jobMsg }}</div>
    </div>

    <div class="section" v-if="syncProgress">
      <h3 class="section-title">[PROG] SYNC PROGRESS</h3>
      <div class="progress-card">
        <div class="p-row"><span class="p-key text-muted">STATUS</span><span class="p-value">{{ syncProgress.status }}</span></div>
        <div class="p-row"><span class="p-key text-muted">FILES</span><span class="p-value">{{ syncProgress.processedFiles }}/{{ syncProgress.totalFiles }}</span></div>
        <div class="p-row"><span class="p-key text-muted">BYTES</span><span class="p-value">{{ formatSize(syncProgress.bytesUploaded) }}</span></div>
        <div v-if="syncProgress.errors.length" class="p-errors">
          <div v-for="(e, i) in syncProgress.errors.slice(0, 5)" :key="i" class="p-err" :title="hintFor(e)">{{ e }}</div>
        </div>
      </div>
    </div>

    <div v-if="syncRuns.length" class="section">
      <h3 class="section-title">[HIST] LAST RUNS ({{ syncRuns.length }})</h3>
      <div v-for="r in syncRuns.slice(0, 5)" :key="r.runId" class="run-card">
        <span class="text-muted">{{ r.runId.slice(0, 18) }}</span>
        <span>{{ r.status }}</span>
        <span class="text-muted">{{ r.filesSynced }} files / {{ formatSize(r.bytesUploaded) }}</span>
      </div>
    </div>

    <div class="section">
      <h3 class="section-title">[RESTORE] DOWNLOAD / DELETE REMOTE</h3>
      <div class="w-row">
        <input v-model="restoreFileId" class="bw-input" placeholder="FILE ID (optional if path given)" />
      </div>
      <div class="w-row">
        <input v-model="restoreRemotePath" class="bw-input" placeholder="REMOTE PATH" />
      </div>
      <div class="w-actions">
        <button class="bw-btn small" :disabled="!runConfigId" @click="doRestore">RESTORE</button>
        <button class="bw-btn small danger" :disabled="!runConfigId || !restoreRemotePath" @click="doRemoteDelete">DELETE REMOTE</button>
        <button class="bw-btn small" :disabled="!runConfigId" @click="browseRemote">BROWSE</button>
      </div>
      <div v-if="remoteFiles.length" class="remote-list">
        <div v-for="f in remoteFiles.slice(0, 20)" :key="f.path" class="remote-row">
          <span>{{ f.name }}</span><span class="text-muted">{{ formatSize(f.sizeBytes) }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import { useAppStore } from '@/stores/app'
import { SYNC_BACKEND_INFO, describeSyncError } from '@/types'
import type { SyncConfig } from '@/types'

const store = useAppStore()
const syncConfigs = computed(() => store.syncConfigs)
const syncProgress = computed(() => store.syncProgress)
const syncRuns = computed(() => store.syncRuns)

const showWizard = ref(false)
const busy = ref(false)
const testMsg = ref('')
const jobMsg = ref('')
const quotaMsg = ref<Record<string, string>>({})
const runConfigId = ref('')
const restoreFileId = ref('')
const restoreRemotePath = ref('')
const remoteFiles = ref<{ name: string; path: string; sizeBytes: number }[]>([])

const form = reactive({
  backendType: 'local' as SyncConfig['backendType'],
  name: '',
  basePath: '',
  repoName: '',
  branch: 'main',
  token: '',
  folderId: '',
  albumId: '',
  chatId: '',
  enabled: true,
  encryptBeforeUpload: true,
  compressBeforeUpload: true,
  placement: 'whole' as 'whole' | 'striped',
  conflictPolicy: 'skip' as 'skip' | 'overwrite' | 'keepBoth',
})

const needsBasePath = computed(() => form.backendType === 'local')
const needsRepo = computed(() => form.backendType === 'github' || form.backendType === 'gitlab')
const needsFolder = computed(() => form.backendType === 'googleDrive')
const needsAlbum = computed(() => form.backendType === 'googlePhotos')
const needsChat = computed(() => form.backendType === 'telegram')
const oauthable = computed(() => form.backendType === 'github' || form.backendType === 'gitlab' || form.backendType === 'googleDrive' || form.backendType === 'googlePhotos')

function hintFor(e: string) {
  const d = describeSyncError(e)
  return `${d.prefix}: ${d.hint}`
}

function formatSize(bytes: number): string {
  if (!bytes) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const k = 1024
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + units[i]
}

function toConfig(): Omit<SyncConfig, 'id' | 'createdAt' | 'updatedAt'> {
  return {
    backendType: form.backendType,
    enabled: form.enabled,
    name: form.name || undefined,
    basePath: form.basePath || undefined,
    repoName: form.repoName || undefined,
    branch: form.branch || undefined,
    token: form.token || undefined,
    folderId: form.folderId || undefined,
    albumId: form.albumId || undefined,
    chatId: form.chatId || undefined,
    autoSync: false,
    compressBeforeUpload: form.compressBeforeUpload,
    createPreviews: false,
    deleteRawAfterSync: false,
    maxConcurrentUploads: 1,
    encryptBeforeUpload: form.encryptBeforeUpload,
    conflictPolicy: form.conflictPolicy,
    placement: form.placement,
    parity: 1,
  }
}

async function testCurrent() {
  busy.value = true
  testMsg.value = 'Testing…'
  try {
    const ok = await store.testSyncConnection(toConfig() as SyncConfig)
    testMsg.value = ok ? 'Connection OK' : 'Connection failed — see toast for prefix + hint.'
  } finally {
    busy.value = false
  }
}

async function saveConfig() {
  busy.value = true
  try {
    await store.createSyncConfig(toConfig())
    showWizard.value = false
    testMsg.value = ''
  } finally {
    busy.value = false
  }
}

async function oauthConnect() {
  testMsg.value = 'Opening provider authorize URL… (paste token if the route 404s)'
  const cfg = syncConfigs.value[0]
  await store.oauthStart(form.backendType, cfg?.id ?? '')
}

async function testCfg(cfg: SyncConfig) {
  await store.testSyncConnection(cfg)
}

async function startCfg(cfg: SyncConfig) {
  runConfigId.value = cfg.id
  await startRun()
}

async function startRun() {
  if (!runConfigId.value) return
  jobMsg.value = 'Starting… (202 job, polling progress)'
  await store.startSync(runConfigId.value, [])
  await store.fetchSyncRuns()
  jobMsg.value = ''
}

async function cancelRun() {
  await store.cancelSync()
}

async function refreshRuns() {
  await store.fetchSyncRuns()
  await store.fetchSyncStatus()
}

async function removeCfg(id: string) {
  await store.deleteSyncConfig(id)
}

async function usageCfg(cfg: SyncConfig) {
  const u = await store.fetchSyncUsage(cfg.id)
  quotaMsg.value[cfg.id] = u ? `quota: ${JSON.stringify(u).slice(0, 160)}` : 'quota unavailable'
}

async function doRestore() {
  if (!runConfigId.value) return
  await store.restoreSyncFile(runConfigId.value, restoreFileId.value || undefined, restoreRemotePath.value || undefined)
}

async function doRemoteDelete() {
  if (!runConfigId.value || !restoreRemotePath.value) return
  await store.deleteRemoteFile(runConfigId.value, restoreRemotePath.value)
}

async function browseRemote() {
  const cfg = syncConfigs.value.find(c => c.id === runConfigId.value)
  if (!cfg) return
  remoteFiles.value = await store.listRemoteFiles(cfg, restoreRemotePath.value || '')
}
</script>

<style scoped>
.sync-panel {
  width: 100%;
  height: 100%;
  background: #000;
  overflow-y: auto;
  padding: 16px;
  font-family: 'Courier New', monospace;
  color: #FFFFFF;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-bottom: 10px;
  border-bottom: 2px solid #FFFFFF;
  margin-bottom: 16px;
}

.header-left { display: flex; align-items: center; gap: 8px; }
.icon-sync { font-size: 16px; }
.panel-title { font-size: 14px; font-weight: 800; letter-spacing: 1px; margin: 0; }

.section { margin-bottom: 16px; }

.section-title {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 1px;
  color: rgba(255,255,255,0.6);
  margin: 0 0 8px;
}

.wizard { border: 2px dashed #FFFFFF; padding: 10px; }
.w-label { display: flex; flex-direction: column; gap: 4px; font-size: 10px; margin-bottom: 8px; }
.bw-input { background: #000; color: #FFF; border: 1px solid #FFF; padding: 6px 8px; font-size: 11px; font-family: inherit; }
.w-row { display: flex; gap: 8px; margin-bottom: 8px; flex-wrap: wrap; }
.w-row .bw-input { flex: 1; min-width: 140px; }
.w-check { font-size: 10px; display: flex; gap: 4px; align-items: center; }
.w-actions { display: flex; gap: 8px; flex-wrap: wrap; }
.w-msg, .cfg-msg { font-size: 10px; margin-top: 6px; color: rgba(255,255,255,0.75); }
.bw-btn { background: #000; color: #FFF; border: 2px solid #FFF; padding: 6px 10px; font-size: 10px; font-weight: 700; cursor: pointer; font-family: inherit; }
.bw-btn.small { font-size: 10px; }
.bw-btn.xs { font-size: 9px; padding: 3px 6px; border-width: 1px; }
.bw-btn.primary { background: #FFF; color: #000; }
.bw-btn.danger { border-color: #F66; color: #F66; }
.bw-btn:disabled { opacity: 0.4; cursor: default; }

.config-list { display: flex; flex-direction: column; gap: 6px; }
.config-card { border: 2px solid #FFFFFF; padding: 8px 10px; }
.cfg-header { display: flex; align-items: center; gap: 8px; margin-bottom: 4px; }
.cfg-name { font-size: 12px; font-weight: 700; flex: 1; }
.cfg-type { font-size: 9px; }
.cfg-status { font-size: 9px; font-weight: 700; border: 1px solid #FFFFFF; padding: 0 4px; }
.cfg-status.on { background: #FFFFFF; color: #000; }
.cfg-meta { font-size: 9px; }
.cfg-actions { display: flex; gap: 6px; margin-top: 6px; flex-wrap: wrap; }
.empty { font-size: 10px; }

.progress-card { border: 2px solid #FFFFFF; padding: 8px; display: flex; flex-direction: column; gap: 4px; }
.p-row { display: flex; justify-content: space-between; }
.p-key { font-size: 10px; }
.p-value { font-size: 10px; font-weight: 700; }
.p-errors { display: flex; flex-direction: column; gap: 2px; }
.p-err { font-size: 9px; color: #F99; word-break: break-all; }
.run-card { display: flex; gap: 8px; font-size: 10px; border: 1px solid #444; padding: 4px 6px; margin-bottom: 4px; }
.remote-list { margin-top: 6px; }
.remote-row { display: flex; justify-content: space-between; font-size: 10px; border-bottom: 1px solid #222; padding: 2px 0; }
.text-muted { color: rgba(255,255,255,0.5) !important; }
</style>
