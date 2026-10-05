<template>
  <div class="acct-panel">
    <div class="panel-header">
      <div class="header-left">
        <span class="icon-acct">[@]</span>
        <h2 class="panel-title">ACCOUNTS</h2>
        <span class="text-muted">{{ store.syncConfigs.length }} PROVIDERS · {{ store.disks.length }} DISKS</span>
      </div>
      <div class="header-right">
        <button class="ghost-btn" type="button" @click="refresh" :disabled="refreshing">{{ refreshing ? '…' : 'REFRESH' }}</button>
      </div>
    </div>

    <!-- Merged volume: proof that every .cybermanju disk is one volume -->
    <div class="section">
      <h3 class="section-title">[VOLUME] ONE MERGED DISK</h3>
      <div v-if="staticHost" class="static-note">
        OFFLINE DEMO VAULT — real redb cybermanju.db in this browser{{ dbBackend ? ` (${dbBackend.toUpperCase()})` : '' }}. ACCOUNTS, PROVIDERS, DISKS, USERS + FILES WORK HERE; ONLY PROVIDER NETWORK SYNC NEEDS THE SERVER.
      </div>
      <div class="df-bar" role="img" :aria-label="`Volume ${volumePct.toFixed(1)} percent used`">
        <div class="df-used" :style="{ width: `${volumePct}%` }"></div>
      </div>
      <div class="df-legend">
        <span>USED {{ human(volumeView?.usedBytes ?? 0) }}</span>
        <span>FREE {{ human(volumeView?.freeBytes ?? 0) }}</span>
        <span>TOTAL {{ human(volumeView?.totalBytes ?? 0) }}</span>
        <span class="text-muted">{{ volumeView?.diskCount ?? 0 }} DISKS ATTACHED</span>
      </div>
    </div>

    <!-- App session -->
    <div class="section">
      <h3 class="section-title">[SESSION] APP LOGIN</h3>
      <div class="card">
        <div v-if="store.currentUser" class="row-between">
          <span>{{ store.currentUser.username }} · {{ store.currentUser.role }}</span>
          <button class="ghost-btn" type="button" @click="store.logout()">LOGOUT</button>
        </div>
        <div v-else class="row-between">
          <span class="text-muted">NOT LOGGED IN — USERNAME + PASSWORD (ARGON2).</span>
          <button class="ghost-btn primary" type="button" @click="store.showLoginPopup = true">LOGIN / REGISTER</button>
        </div>
      </div>
    </div>

    <!-- Local accounts -->
    <div class="section">
      <h3 class="section-title">[LOCAL] DEVICE ACCOUNTS ({{ store.accounts.length }})</h3>
      <div v-if="store.accounts.length" class="cards">
        <div
          v-for="a in store.accounts"
          :key="a.id"
          class="card clickable"
          :class="{ active: a.isActive }"
          @click="store.switchAccount(a.id)"
          :title="a.isActive ? 'Active account' : 'Click to switch'"
        >
          <div class="row-between">
            <span><i class="dot" :class="{ on: a.isActive }"></i>{{ a.name }}</span>
            <span class="row-actions" @click.stop>
              <span v-if="a.isActive" class="badge-on">ACTIVE</span>
              <button v-else class="ghost-btn xs" type="button" @click="store.switchAccount(a.id)">SWITCH</button>
              <button class="ghost-btn xs danger" type="button" :disabled="!!a.isActive" title="The active account cannot be deleted" @click="removeAccount(a.id)">DEL</button>
            </span>
          </div>
          <div class="text-muted small">{{ a.accountType }}{{ a.path ? ' · ' + a.path : '' }}</div>
        </div>
      </div>
      <p v-else class="text-muted empty">No device accounts yet.</p>
      <div class="form-row">
        <input v-model="newAcctName" class="input" placeholder="ACCOUNT NAME" aria-label="Account name" />
        <select v-model="newAcctType" class="input" aria-label="Account type">
          <option value="local">LOCAL</option>
          <option value="cloud">CLOUD</option>
          <option value="network">NETWORK</option>
        </select>
        <input v-model="newAcctPath" class="input" placeholder="PATH (optional)" aria-label="Account path" />
        <button class="ghost-btn primary" type="button" :disabled="!newAcctName.trim()" @click="addAccount">+ ADD</button>
      </div>
    </div>

    <!-- Providers -->
    <div class="section">
      <h3 class="section-title">[PROVIDERS] CLOUD + LOCAL CONNECTIONS</h3>
      <div v-if="!store.syncConfigs.length" class="card">
        <span class="text-muted">No providers connected — add one below.</span>
      </div>
      <article v-for="cfg in store.syncConfigs" :key="cfg.id" class="card provider">
        <header class="row-between">
          <div>
            <span class="p-name">{{ cfg.name || cfg.backendType }}</span>
            <span class="text-muted small"> · {{ backendLabel(cfg.backendType) }}</span>
          </div>
          <span class="row-actions">
            <button
              class="ghost-btn xs"
              type="button"
              :class="{ on: cfg.enabled }"
              @click="toggleEnabled(cfg)"
              :title="cfg.enabled ? 'Disable provider' : 'Enable provider'"
            >{{ cfg.enabled ? 'ON' : 'OFF' }}</button>
            <button class="ghost-btn xs" type="button" @click="probe(cfg)">TEST</button>
            <button class="ghost-btn xs danger" type="button" @click="removeProvider(cfg.id)">DEL</button>
          </span>
        </header>

        <!-- auth status -->
        <div class="auth-row">
          <span class="auth-badge" :class="authClass(cfg.id)">{{ authLabel(cfg.id) }}</span>
          <span v-if="authDetail(cfg.id)" class="text-muted small" :title="authHint(cfg.id)">{{ authDetail(cfg.id) }}</span>
        </div>

        <!-- PKCE OAuth -->
        <div v-if="isOauthCapable(cfg.backendType)" class="oauth-block">
          <div class="row-between">
            <span class="small">PKCE OAUTH — BROWSER APPROVAL, NO PASSWORD TYPED HERE</span>
            <button
              class="ghost-btn xs primary"
              type="button"
              :disabled="oauthBusy === cfg.id"
              @click="oauthConnect(cfg)"
            >{{ oauthBusy === cfg.id ? 'WAITING…' : 'CONNECT WITH OAUTH' }}</button>
          </div>
          <p v-if="oauthMsg[cfg.id]" class="note">{{ oauthMsg[cfg.id] }}</p>
          <p v-if="oauthBusy === cfg.id" class="note">Approve in the opened browser tab — this panel polls the provider until credentials land. <button class="linklike" type="button" @click="cancelOauth">cancel</button></p>
          <p v-if="oauthUrl[cfg.id]" class="note">Popup blocked? Open manually: <span class="mono url">{{ oauthUrl[cfg.id] }}</span></p>
        </div>

        <!-- credentials -->
        <div class="cred-grid">
          <label v-if="needsRepo(cfg.backendType)" class="field">
            <span class="text-muted small">{{ cfg.backendType === 'gitlab' ? 'PROJECT ID' : 'REPO (owner/repo)' }}</span>
            <input v-model="draft(cfg).repoName" class="input" placeholder="owner/repo" autocomplete="off" />
          </label>
          <label v-if="needsRepo(cfg.backendType)" class="field">
            <span class="text-muted small">BRANCH</span>
            <input v-model="draft(cfg).branch" class="input" placeholder="main" autocomplete="off" />
          </label>
          <label v-if="cfg.backendType === 'googleDrive'" class="field">
            <span class="text-muted small">DRIVE FOLDER ID</span>
            <input v-model="draft(cfg).folderId" class="input" placeholder="folder id (optional)" autocomplete="off" />
          </label>
          <label v-if="cfg.backendType === 'googlePhotos'" class="field">
            <span class="text-muted small">PHOTOS ALBUM ID</span>
            <input v-model="draft(cfg).albumId" class="input" placeholder="album id (optional)" autocomplete="off" />
          </label>
          <label v-if="cfg.backendType === 'telegram'" class="field">
            <span class="text-muted small">CHAT ID</span>
            <input v-model="draft(cfg).chatId" class="input" placeholder="chat id" autocomplete="off" />
          </label>
          <label v-if="cfg.backendType === 'gitlab'" class="field grow">
            <span class="text-muted small">INSTANCE URL (SELF-HOSTED — EMPTY = GITLAB.COM)</span>
            <input v-model="draft(cfg).basePath" class="input" placeholder="https://gitlab.example.com" autocomplete="off" />
          </label>
          <label v-if="cfg.backendType === 'local'" class="field grow">
            <span class="text-muted small">LOCAL PATH</span>
            <input v-model="draft(cfg).basePath" class="input" placeholder="/DATA/SYNC" autocomplete="off" />
          </label>
          <label v-if="needsToken(cfg.backendType)" class="field grow">
            <span class="text-muted small">{{ tokenLabel(cfg.backendType) }}</span>
            <input v-model="draft(cfg).token" class="input" type="password" placeholder="paste — never shown back" autocomplete="off" />
          </label>
        </div>
        <p class="text-muted small">{{ authGuidance(cfg.backendType) }}</p>
        <div class="row-actions" style="margin-top:6px;">
          <button class="ghost-btn xs primary" type="button" :disabled="saving === cfg.id" @click="saveCreds(cfg)">{{ saving === cfg.id ? 'SAVING…' : 'SAVE & VERIFY' }}</button>
          <button class="ghost-btn xs" type="button" title="Uses saved credentials — SAVE & VERIFY first if you just pasted a token" @click="quota(cfg)">QUOTA</button>
          <span v-if="quotaMsg[cfg.id]" class="text-muted small">{{ quotaMsg[cfg.id] }}</span>
        </div>

        <!-- disks bound to this provider: the .cybermanju file size lives here -->
        <div class="disks-block">
          <div class="row-between">
            <span class="small">DISKS ON THIS PROVIDER ({{ disksFor(cfg.id).length }}) — EACH IS ONE .CYBERMANJU FILE, MERGED INTO THE VOLUME</span>
          </div>
          <div v-for="d in disksFor(cfg.id)" :key="d.id" class="disk-row">
            <div class="row-between">
              <span>{{ d.name || d.id.slice(0, 8) }} · <span class="text-muted">{{ d.state }} / {{ d.health }}</span></span>
              <span class="row-actions">
                <button v-if="d.state !== 'attached'" class="ghost-btn xs primary" type="button" :disabled="diskBusy === d.id" @click="attachDisk(d.id)">ATTACH</button>
                <button v-else class="ghost-btn xs" type="button" :disabled="diskBusy === d.id" @click="store.detachDisk(d.id).then(afterDiskChange)">DETACH</button>
                <button class="ghost-btn xs" type="button" :disabled="diskBusy === d.id" @click="store.checkDisk(d.id)">CHECK</button>
              </span>
            </div>
            <div class="mini-bar"><div class="mini-used" :style="{ width: `${diskPct(d.usedBytes, d.capacityBytes)}%` }"></div></div>
            <div class="row-between small">
              <span>{{ human(d.usedBytes) }} / {{ human(d.capacityBytes) }}</span>
              <span class="row-actions">
                <input v-model.number="resizeMb[d.id]" class="input xs-num" type="number" min="64" max="8192" step="64" :aria-label="`New size MB for ${d.name}`" />
                <span class="text-muted">MB</span>
                <button class="ghost-btn xs" type="button" :disabled="diskBusy === d.id" @click="applyResize(d.id)">APPLY SIZE</button>
              </span>
            </div>
          </div>
          <div class="form-row">
            <label class="small text-muted">NEW DISK — {{ newDiskMb[cfg.id] ?? 512 }} MB</label>
            <input v-model.number="newDiskMb[cfg.id]" class="slider" type="range" min="64" max="8192" step="64" :aria-label="`New disk size for ${cfg.name || cfg.backendType}`" />
            <input v-model="newDiskPass[cfg.id]" class="input" type="password" placeholder="PASSPHRASE" autocomplete="off" :aria-label="`Passphrase for new disk on ${cfg.name || cfg.backendType}`" />
            <button class="ghost-btn xs primary" type="button" :disabled="diskBusy === cfg.id" @click="createDisk(cfg.id)">{{ diskBusy === cfg.id ? 'CREATING…' : 'CREATE & ATTACH' }}</button>
          </div>
        </div>
      </article>
    </div>

    <!-- New provider wizard -->
    <div class="section">
      <h3 class="section-title">[NEW] ADD PROVIDER</h3>
      <div class="card">
        <div class="form-row">
          <select v-model="wiz.backendType" class="input" aria-label="Backend">
            <option v-for="(info, key) in SYNC_BACKEND_INFO" :key="key" :value="key">{{ info.name }}</option>
          </select>
          <input v-model="wiz.name" class="input" placeholder="DISPLAY NAME" aria-label="Display name" />
        </div>
        <div class="cred-grid">
          <label v-if="needsRepo(wiz.backendType)" class="field">
            <span class="text-muted small">{{ wiz.backendType === 'gitlab' ? 'PROJECT ID' : 'REPO (owner/repo)' }}</span>
            <input v-model="wiz.repoName" class="input" placeholder="owner/repo" autocomplete="off" />
          </label>
          <label v-if="needsRepo(wiz.backendType)" class="field">
            <span class="text-muted small">BRANCH</span>
            <input v-model="wiz.branch" class="input" placeholder="main" autocomplete="off" />
          </label>
          <label v-if="wiz.backendType === 'googleDrive'" class="field">
            <span class="text-muted small">DRIVE FOLDER ID</span>
            <input v-model="wiz.folderId" class="input" placeholder="optional" autocomplete="off" />
          </label>
          <label v-if="wiz.backendType === 'googlePhotos'" class="field">
            <span class="text-muted small">PHOTOS ALBUM ID</span>
            <input v-model="wiz.albumId" class="input" placeholder="optional" autocomplete="off" />
          </label>
          <label v-if="wiz.backendType === 'telegram'" class="field">
            <span class="text-muted small">CHAT ID</span>
            <input v-model="wiz.chatId" class="input" placeholder="chat id" autocomplete="off" />
          </label>
          <label v-if="wiz.backendType === 'gitlab'" class="field grow">
            <span class="text-muted small">INSTANCE URL (SELF-HOSTED — EMPTY = GITLAB.COM)</span>
            <input v-model="wiz.basePath" class="input" placeholder="https://gitlab.example.com" autocomplete="off" />
          </label>
          <label v-if="wiz.backendType === 'local'" class="field grow">
            <span class="text-muted small">LOCAL PATH</span>
            <input v-model="wiz.basePath" class="input" placeholder="/DATA/SYNC" autocomplete="off" />
          </label>
          <label v-if="needsToken(wiz.backendType)" class="field grow">
            <span class="text-muted small">{{ tokenLabel(wiz.backendType) }}</span>
            <input v-model="wiz.token" class="input" type="password" placeholder="paste token — optional when using OAuth" autocomplete="off" />
          </label>
        </div>
        <p class="text-muted small">{{ authGuidance(wiz.backendType) }}</p>
        <div class="row-actions" style="margin-top:6px;">
          <button class="ghost-btn xs primary" type="button" :disabled="wizBusy" @click="addProvider(true)">{{ wizBusy ? 'VERIFYING…' : 'SAVE & VERIFY' }}</button>
          <button class="ghost-btn xs" type="button" :disabled="wizBusy" @click="addProvider(false)">SAVE</button>
          <span v-if="wizMsg" class="text-muted small">{{ wizMsg }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import { useAppStore } from '@/stores/app'
import { isStaticHost } from '@/composables/useTauri'
import { wasmDbBackend } from '@/composables/useWasmBackend'
import { SYNC_BACKEND_INFO, describeSyncError, isOauthCapable } from '@/types'
import type { DiskRow, SyncBackendType, SyncConfig } from '@/types'

const store = useAppStore()

/**
 * Static WASM pack (GitHub Pages): no dashboard behind the page. Accounts,
 * providers, disks, users and files run offline against a real redb
 * `cybermanju.db` in this browser (OPFS-durable, in-memory fallback) via
 * the DB worker. Only provider *network* calls need the server.
 */
const staticHost = isStaticHost()
const dbBackend = ref<string | null>(null)

const refreshing = ref(false)
const saving = ref<string | null>(null)
const diskBusy = ref<string | null>(null)
const wizBusy = ref(false)
const wizMsg = ref('')
const oauthBusy = ref<string | null>(null)
const oauthTimer = ref(0)

const oauthMsg = ref<Record<string, string>>({})
const oauthUrl = ref<Record<string, string>>({})
const quotaMsg = ref<Record<string, string>>({})
const authState = ref<Record<string, { ok: boolean | null; detail: string }>>({})

const newAcctName = ref('')
const newAcctType = ref('local')
const newAcctPath = ref('')

const newDiskMb = ref<Record<string, number>>({})
const newDiskPass = ref<Record<string, string>>({})
const resizeMb = ref<Record<string, number>>({})

interface Draft {
  repoName: string
  branch: string
  token: string
  folderId: string
  albumId: string
  chatId: string
  basePath: string
  name: string
}
const drafts = reactive<Record<string, Draft>>({})

const wiz = reactive({
  backendType: 'local' as SyncBackendType,
  name: '',
  repoName: '',
  branch: 'main',
  token: '',
  folderId: '',
  albumId: '',
  chatId: '',
  basePath: '',
})

const df = computed(() => store.osDf)
const usedPct = computed(() => {
  const d = df.value
  if (!d || d.totalBytes === 0) return 0
  return Math.min(100, (d.usedBytes / d.totalBytes) * 100)
})

// In the static build the OS `df` covers the terminal's virtual volume, so
// the strip instead sums the real attached `.cybermanju` disks — the same
// merge the server reports, computed client-side from the same rows.
const diskVolume = computed(() => {
  const attached = store.disks.filter(d => d.state === 'attached')
  const total = attached.reduce((s, d) => s + (d.capacityBytes || 0), 0)
  const used = attached.reduce((s, d) => s + (d.usedBytes || 0), 0)
  return {
    totalBytes: total,
    usedBytes: used,
    freeBytes: Math.max(0, total - used),
    diskCount: attached.length,
    attachedBytes: total,
    scratchBytes: 0,
    root: '/',
  }
})
const volumeView = computed(() => (staticHost ? diskVolume.value : df.value))
const volumePct = computed(() => {
  const d = volumeView.value
  if (!d || d.totalBytes === 0) return 0
  return Math.min(100, (d.usedBytes / d.totalBytes) * 100)
})

function human(bytes: number): string {
  if (!bytes) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let v = bytes
  let u = 0
  while (v >= 1024 && u < units.length - 1) { v /= 1024; u += 1 }
  return `${v >= 10 || u === 0 ? v.toFixed(0) : v.toFixed(1)} ${units[u]}`
}

function diskPct(used: number, capacity: number): number {
  if (!capacity) return 0
  return Math.min(100, (used / capacity) * 100)
}

function backendLabel(b: SyncBackendType): string {
  return SYNC_BACKEND_INFO[b]?.name ?? b
}

function needsRepo(b: SyncBackendType): boolean {
  return b === 'github' || b === 'gitlab'
}

function needsToken(b: SyncBackendType): boolean {
  return b === 'github' || b === 'gitlab' || b === 'telegram' || b === 'googleDrive' || b === 'googlePhotos'
}

function tokenLabel(b: SyncBackendType): string {
  if (b === 'telegram') return 'BOT TOKEN (PASSWORD FOR THIS CHAT)'
  if (b === 'github') return 'TOKEN — PERSONAL ACCESS TOKEN (USED AS THE PASSWORD)'
  if (b === 'gitlab') return 'TOKEN — PERSONAL ACCESS TOKEN (USED AS THE PASSWORD)'
  return 'TOKEN — OPTIONAL WHEN USING OAUTH'
}

function authGuidance(b: SyncBackendType): string {
  switch (b) {
    case 'github':
      return 'GitHub removed account passwords: sign in with OAUTH above, or paste a personal access token (repo scope) as the password.'
    case 'gitlab':
      return 'GitLab sign-in is OAUTH, or a personal access token (api scope) pasted as the password. Self-hosted? Set the instance URL too.'
    case 'googleDrive':
    case 'googlePhotos':
      return 'Google accepts OAUTH only — there is no password login. CONNECT WITH OAUTH above.'
    case 'telegram':
      return 'Telegram uses a bot token from @BotFather plus the chat id — no OAuth, no password.'
    default:
      return 'Local directory needs no login — just the path.'
  }
}

function draft(cfg: SyncConfig): Draft {
  let d = drafts[cfg.id]
  if (!d) {
    d = {
      repoName: cfg.repoName ?? '',
      branch: cfg.branch ?? 'main',
      token: '',
      folderId: cfg.folderId ?? '',
      albumId: cfg.albumId ?? '',
      chatId: cfg.chatId ?? '',
      basePath: cfg.basePath ?? '',
      name: cfg.name ?? '',
    }
    drafts[cfg.id] = d
  }
  return d
}

function disksFor(configId: string): DiskRow[] {
  return store.disks.filter(d => d.configId === configId)
}

function authLabel(id: string): string {
  const s = authState.value[id]
  if (!s || s.ok === null) return 'UNTESTED'
  return s.ok ? 'CONNECTED' : 'AUTH FAILED'
}

function authClass(id: string): string {
  const s = authState.value[id]
  if (!s || s.ok === null) return ''
  return s.ok ? 'ok' : 'bad'
}

function authDetail(id: string): string {
  return authState.value[id]?.detail ?? ''
}

function authHint(id: string): string {
  const d = authDetail(id)
  if (!d) return ''
  const { prefix, hint } = describeSyncError(d)
  return `${prefix}: ${hint}`
}

async function refresh() {
  refreshing.value = true
  await Promise.allSettled([
    store.fetchAccounts(),
    store.fetchSyncConfigs(),
    store.fetchDisks(),
    store.fetchOsDf(),
  ])
  refreshing.value = false
}

async function addAccount() {
  if (!newAcctName.value.trim()) return
  await store.createAccount(newAcctName.value.trim(), newAcctType.value, newAcctPath.value.trim() || undefined)
  newAcctName.value = ''
  newAcctPath.value = ''
}

async function removeAccount(id: string) {
  if (!window.confirm('Delete this device account?')) return
  await store.deleteAccount(id)
}

async function toggleEnabled(cfg: SyncConfig) {
  await store.saveSyncConfig({ ...cfg, enabled: !cfg.enabled })
}

/**
 * The config as the user currently sees it: stored row overlaid with any
 * draft edits (notably a freshly pasted token). Probing this — instead of
 * the bare stored row — is what makes "paste token, hit TEST" actually
 * verify the token before it is saved.
 */
function mergedForProbe(cfg: SyncConfig): SyncConfig {
  const d = drafts[cfg.id]
  if (!d) return cfg
  const merged: SyncConfig = { ...cfg }
  if (d.token.trim()) merged.token = d.token.trim()
  if (d.repoName.trim()) merged.repoName = d.repoName.trim()
  if (d.branch.trim()) merged.branch = d.branch.trim()
  if (d.folderId.trim()) merged.folderId = d.folderId.trim()
  if (d.albumId.trim()) merged.albumId = d.albumId.trim()
  if (d.chatId.trim()) merged.chatId = d.chatId.trim()
  if (d.basePath.trim()) merged.basePath = d.basePath.trim()
  return merged
}

async function probe(cfg: SyncConfig) {
  authState.value[cfg.id] = { ok: null, detail: 'probing…' }
  const r = await store.probeSyncConnection(mergedForProbe(cfg))
  authState.value[cfg.id] = { ok: r.ok, detail: r.detail }
}

async function removeProvider(id: string) {
  if (!window.confirm('Delete this provider connection? Its disks stay in the catalog.')) return
  delete drafts[id]
  delete authState.value[id]
  await store.deleteSyncConfig(id)
}

async function saveCreds(cfg: SyncConfig) {
  const d = draft(cfg)
  saving.value = cfg.id
  const updated: SyncConfig = {
    ...cfg,
    name: d.name.trim() || cfg.name,
    repoName: d.repoName.trim() || undefined,
    branch: d.branch.trim() || undefined,
    folderId: d.folderId.trim() || undefined,
    albumId: d.albumId.trim() || undefined,
    chatId: d.chatId.trim() || undefined,
    basePath: d.basePath.trim() || undefined,
  }
  if (d.token.trim()) updated.token = d.token.trim()
  const saved = await store.saveSyncConfig(updated)
  d.token = ''
  if (saved) {
    const r = await store.probeSyncConnection({ ...saved, ...(updated.token ? { token: updated.token } : {}) })
    authState.value[cfg.id] = { ok: r.ok, detail: r.detail }
    if (r.ok) store.notifySuccess('Provider verified — credentials work')
  }
  saving.value = null
}

async function quota(cfg: SyncConfig) {
  const u = await store.fetchSyncUsage(cfg.id)
  quotaMsg.value[cfg.id] = u
    ? `quota: ${[u.totalBytes != null ? `total ${human(u.totalBytes)}` : null, u.usedBytes != null ? `used ${human(u.usedBytes)}` : null, u.remainingRequests != null ? `${u.remainingRequests} req left` : null].filter(Boolean).join(' · ') || u.detail}`
    : 'quota unavailable'
}

/** PKCE OAuth: open the provider approval, then poll until credentials land. */
async function oauthConnect(cfg: SyncConfig) {
  // No dashboard behind the static build means no server-side callback to
  // land credentials in — say so immediately instead of polling for 2 min.
  if (staticHost) {
    oauthMsg.value[cfg.id] =
      'OAuth needs the dashboard (or desktop app) for the server callback — in this offline demo, paste a token below instead.'
    return
  }
  cancelOauth()
  oauthBusy.value = cfg.id
  oauthMsg.value[cfg.id] = 'Opening provider approval…'
  const res = await store.oauthStart(cfg.backendType, cfg.id)
  if (!res?.authorizeUrl) {
    oauthMsg.value[cfg.id] = 'OAuth did not start — paste a token below instead.'
    oauthBusy.value = null
    return
  }
  const popup = window.open(res.authorizeUrl, 'cyb_oauth', 'width=620,height=720')
  if (!popup) oauthUrl.value[cfg.id] = res.authorizeUrl
  oauthMsg.value[cfg.id] = 'Approve in the browser tab — waiting for the callback…'
  let attempts = 0
  oauthTimer.value = window.setInterval(async () => {
    attempts += 1
    const r = await store.probeSyncConnection(cfg)
    if (r.ok) {
      cancelOauth()
      authState.value[cfg.id] = { ok: true, detail: 'OAuth credentials verified' }
      oauthMsg.value[cfg.id] = 'Connected — OAuth credentials verified.'
      store.notifySuccess(`${cfg.name || cfg.backendType}: OAuth connected`)
      return
    }
    if (attempts >= 40) {
      cancelOauth()
      oauthMsg.value[cfg.id] = 'Timed out waiting for approval (2 min). Retry, or paste a token below.'
    } else {
      oauthMsg.value[cfg.id] = `Approve in the browser tab — waiting… (${attempts * 3}s)`
    }
  }, 3000)
}

function cancelOauth() {
  if (oauthTimer.value) window.clearInterval(oauthTimer.value)
  oauthTimer.value = 0
  oauthBusy.value = null
}

async function attachDisk(diskId: string) {
  diskBusy.value = diskId
  const pass = promptPass(diskId)
  await store.attachDisk(diskId, pass)
  await afterDiskChange()
  diskBusy.value = null
}

function promptPass(diskId: string): string {
  const d = store.disks.find(x => x.id === diskId)
  const v = window.prompt(`Passphrase to unlock disk "${d?.name || diskId}" (empty = none):`, '')
  return v ?? ''
}

async function applyResize(diskId: string) {
  const mb = resizeMb.value[diskId]
  if (!mb || mb < 64) return
  diskBusy.value = diskId
  await store.resizeDisk(diskId, Math.round(mb) * 1024 * 1024)
  await afterDiskChange()
  diskBusy.value = null
}

async function createDisk(configId: string) {
  diskBusy.value = configId
  const mb = newDiskMb.value[configId] ?? 512
  await store.createDisk(configId, Math.round(mb) * 1024 * 1024, newDiskPass.value[configId] ?? '')
  newDiskPass.value[configId] = ''
  await afterDiskChange()
  diskBusy.value = null
}

async function afterDiskChange() {
  await Promise.allSettled([store.fetchDisks(), store.fetchOsDf()])
}

function wizToConfig(): Omit<SyncConfig, 'id' | 'createdAt' | 'updatedAt'> {
  return {
    backendType: wiz.backendType,
    enabled: true,
    name: wiz.name.trim() || undefined,
    basePath: wiz.basePath.trim() || undefined,
    repoName: wiz.repoName.trim() || undefined,
    branch: wiz.branch.trim() || undefined,
    token: wiz.token.trim() || undefined,
    folderId: wiz.folderId.trim() || undefined,
    albumId: wiz.albumId.trim() || undefined,
    chatId: wiz.chatId.trim() || undefined,
    autoSync: false,
    compressBeforeUpload: false,
    createPreviews: false,
    deleteRawAfterSync: false,
    maxConcurrentUploads: 1,
    encryptBeforeUpload: true,
    conflictPolicy: 'skip',
    placement: 'whole',
    parity: 1,
  }
}

async function addProvider(verify: boolean) {
  wizBusy.value = true
  wizMsg.value = ''
  const token = wiz.token.trim()
  const saved = await store.saveSyncConfig({ ...(wizToConfig() as SyncConfig), id: '' })
  wiz.token = ''
  if (!saved) { wizBusy.value = false; return }
  if (verify) {
    wizMsg.value = 'Verifying…'
    const r = await store.probeSyncConnection(token ? { ...saved, token } : saved)
    authState.value[saved.id] = { ok: r.ok, detail: r.detail }
    wizMsg.value = r.ok ? 'Verified — provider connected.' : `Saved, but verification failed: ${r.detail}`
  } else {
    wizMsg.value = 'Saved.'
  }
  wiz.name = ''
  wizBusy.value = false
}

onMounted(() => {
  void refresh()
  if (staticHost) {
    void wasmDbBackend().then((b) => {
      dbBackend.value = b
    })
  }
})

onBeforeUnmount(() => {
  cancelOauth()
})
</script>

<style scoped>
.acct-panel {
  height: 100%;
  overflow-y: auto;
  background: #000;
  color: #fff;
  font-family: 'Courier New', monospace;
  font-size: 13px;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.15);
}

.header-left {
  display: flex;
  align-items: center;
  gap: 10px;
}

.icon-acct { color: #5af78e; }

.panel-title {
  margin: 0;
  font-size: 13px;
  letter-spacing: 2px;
}

.header-right { display: flex; gap: 6px; }

.ghost-btn {
  background: transparent;
  border: 1px solid rgba(255, 255, 255, 0.3);
  color: rgba(255, 255, 255, 0.75);
  font-family: inherit;
  font-size: 11px;
  padding: 3px 8px;
  cursor: pointer;
}

.ghost-btn:hover:not(:disabled) { color: #fff; border-color: #fff; }
.ghost-btn:disabled { opacity: 0.4; cursor: not-allowed; }
.ghost-btn.primary { color: #5af78e; border-color: rgba(90, 247, 142, 0.6); }
.ghost-btn.primary:hover:not(:disabled) { color: #000; background: #5af78e; border-color: #5af78e; }
.ghost-btn.danger { color: #ff5f56; border-color: rgba(255, 95, 86, 0.55); }
.ghost-btn.danger:hover:not(:disabled) { color: #000; background: #ff5f56; border-color: #ff5f56; }
.ghost-btn.xs { font-size: 10px; padding: 2px 6px; }
.ghost-btn.on { color: #5af78e; border-color: rgba(90, 247, 142, 0.6); }

.section { padding: 12px; border-bottom: 1px solid rgba(255, 255, 255, 0.08); }

.section-title {
  margin: 0 0 10px;
  font-size: 11px;
  letter-spacing: 1.5px;
  color: rgba(255, 255, 255, 0.55);
}

.df-bar { height: 18px; background: rgba(255, 255, 255, 0.1); border: 1px solid rgba(255, 255, 255, 0.2); }
.df-used { height: 100%; background: linear-gradient(90deg, #5af78e, #57c7ff); }
.df-legend { display: flex; flex-wrap: wrap; gap: 14px; margin-top: 8px; font-size: 11px; }

.card { border: 1px solid rgba(255, 255, 255, 0.16); padding: 10px; margin-bottom: 8px; }
.card.clickable { cursor: pointer; }
.card.clickable:hover { border-color: rgba(255, 255, 255, 0.4); }
.card.active { border-color: rgba(90, 247, 142, 0.6); }
.card.provider { border-width: 1px; }

.cards { display: flex; flex-direction: column; gap: 6px; margin-bottom: 8px; }

.row-between { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
.row-actions { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }

.dot { display: inline-block; width: 8px; height: 8px; margin-right: 6px; border: 1px solid #888; border-radius: 50%; }
.dot.on { background: #5af78e; border-color: #5af78e; }
.badge-on { font-size: 10px; color: #5af78e; border: 1px solid rgba(90, 247, 142, 0.6); padding: 1px 6px; }

.auth-row { display: flex; align-items: center; gap: 8px; margin: 8px 0; flex-wrap: wrap; }
.auth-badge { font-size: 10px; font-weight: 700; border: 1px solid rgba(255, 255, 255, 0.3); color: rgba(255, 255, 255, 0.6); padding: 1px 6px; }
.auth-badge.ok { color: #5af78e; border-color: rgba(90, 247, 142, 0.6); }
.auth-badge.bad { color: #ff5f56; border-color: rgba(255, 95, 86, 0.6); }

.oauth-block { border: 1px dashed rgba(255, 255, 255, 0.25); padding: 8px; margin: 8px 0; }

.cred-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 8px; margin-top: 8px; }
.field { display: flex; flex-direction: column; gap: 4px; font-size: 11px; }
.field.grow { grid-column: 1 / -1; }

.input {
  background: #000;
  border: 1px solid rgba(255, 255, 255, 0.3);
  color: #fff;
  font-family: inherit;
  font-size: 12px;
  padding: 5px 6px;
  outline: none;
  min-width: 0;
}
.input:focus { border-color: #5af78e; }
.input.xs-num { width: 76px; }

.form-row { display: flex; gap: 8px; margin-top: 8px; flex-wrap: wrap; align-items: center; }
.form-row .input { flex: 1; min-width: 140px; }

.disks-block { margin-top: 10px; border-top: 1px solid rgba(255, 255, 255, 0.1); padding-top: 8px; }
.disk-row { border: 1px solid rgba(255, 255, 255, 0.12); padding: 6px 8px; margin-top: 6px; display: flex; flex-direction: column; gap: 6px; }
.mini-bar { height: 8px; background: rgba(255, 255, 255, 0.12); }
.mini-used { height: 100%; background: #5af78e; }
.slider { flex: 1; min-width: 140px; accent-color: #5af78e; }

.note { margin: 6px 0 0; font-size: 11px; color: #9aedfe; white-space: pre-wrap; word-break: break-word; }
.static-note {
  border: 1px dashed #f3f99d;
  color: #f3f99d;
  font-size: 10px;
  line-height: 1.5;
  padding: 8px 10px;
  margin-bottom: 8px;
  letter-spacing: 0.3px;
}
.mono { font-family: inherit; }
.url { word-break: break-all; color: #9aedfe; }
.linklike { background: none; border: none; color: #9aedfe; cursor: pointer; font: inherit; text-decoration: underline; padding: 0; }
.small { font-size: 11px; }
.empty { margin: 0 0 8px; font-size: 12px; }
.text-muted { color: rgba(255, 255, 255, 0.5) !important; }
</style>
