<template>
  <div class="acct-panel">
    <div class="panel-header">
      <div class="header-left">
        <span class="icon-acct"><AppIcon name="solar:user-circle-bold" /></span>
        <h2 class="panel-title">ACCOUNTS</h2>
        <span class="text-muted">{{ store.syncConfigs.length }} PROVIDERS · {{ store.disks.length }} DISKS</span>
      </div>
      <div class="header-right">
        <button class="ghost-btn" type="button" @click="refresh" :disabled="refreshing">{{ refreshing ? '…' : 'REFRESH' }}</button>
      </div>
    </div>

    <!-- Merged volume: proof that every .cybermanju disk is one volume -->
    <div class="section">
      <h3 class="section-title"><AppIcon name="solar:ssd-square-bold" :size="13" /> ONE MERGED DISK</h3>
      <div v-if="staticHost" class="static-note">
        OFFLINE DEMO VAULT — real redb cybermanju.db in this browser{{ dbBackend ? ` (${dbBackend.toUpperCase()})` : '' }}. ACCOUNTS, PROVIDERS, DISKS, USERS + FILES WORK HERE; ONLY PROVIDER NETWORK SYNC NEEDS THE SERVER.
      </div>
      <div class="df-bar" role="img" :aria-label="`Volume ${volumePct.toFixed(1)} percent used`">
        <div class="df-used" :style="{ width: `${volumePct}%` }"></div>
      </div>
      <div class="df-legend">
        <span>USED {{ humanBytes(volumeView?.usedBytes ?? 0) }}</span>
        <span>FREE {{ humanBytes(volumeView?.freeBytes ?? 0) }}</span>
        <span>TOTAL {{ humanBytes(volumeView?.totalBytes ?? 0) }}</span>
        <span class="text-muted">{{ volumeView?.diskCount ?? 0 }} DISKS ATTACHED</span>
      </div>
    </div>

    <!-- Identity — OAuth is the only sign-in; there is no password form -->
    <div class="section">
      <h3 class="section-title"><AppIcon name="solar:login-bold" :size="13" /> IDENTITY — OAUTH SIGN-IN</h3>
      <div v-if="identity" class="card">
        <div class="row-between">
          <span class="identity-row">
            <img v-if="identity.avatarUrl" class="avatar" :src="identity.avatarUrl" alt="" />
            {{ identity.name }}
            <span class="text-muted small"> · {{ identity.email || identity.provider }}</span>
          </span>
          <span class="row-actions">
            <span class="badge-on">{{ identity.provider.toUpperCase() }}</span>
            <button class="ghost-btn xs" type="button" :disabled="signingOut" @click="signOut">{{ signingOut ? '…' : 'SIGN OUT' }}</button>
          </span>
        </div>
        <p class="text-muted small">Signed in through the Supabase broker — this app stores no password anywhere.</p>
      </div>
      <div v-else class="card">
        <div class="row-between">
          <span class="text-muted">NOT SIGNED IN — PICK A PROVIDER (APPROVE THERE, NOTHING IS TYPED HERE).</span>
        </div>
        <div class="row-actions" style="margin-top:6px;">
          <button
            v-for="p in IDENTITY_PROVIDERS"
            :key="p.id"
            class="ghost-btn xs primary"
            type="button"
            :disabled="signInBusy === p.id || !sbConfigured"
            @click="signIn(p.id)"
          >{{ signInBusy === p.id ? 'OPENING…' : `CONTINUE WITH ${p.label}` }}</button>
        </div>
        <p v-if="signInMsg" class="note">{{ signInMsg }}</p>
        <p v-if="!sbConfigured" class="note">Set SUPABASE URL + KEY in Settings → OAUTH first — the broker runs the sign-in flow (enable the provider under Supabase → Authentication → Sign-in).</p>
      </div>
    </div>

    <!-- This machine: the .cybermanju file the whole vault lives in -->
    <div class="section">
      <h3 class="section-title"><AppIcon name="solar:diskette-bold" :size="13" /> DISK — THIS MACHINE (.CYBERMANJU FILE)</h3>
      <div class="card">
        <div class="row-between">
          <span>
            <template v-if="disk.attached">
              <i class="dot" :class="{ on: !disk.dirty }"></i>{{ disk.name }}
              <span class="text-muted small"> · {{ humanBytes(disk.savedBytes) }}<template v-if="disk.dirty"> · UNSAVED CHANGES</template><template v-else> · SAVED {{ timeOf(disk.savedAt) }}</template></span>
            </template>
            <span v-else class="text-muted">NO FILE ATTACHED — THE VAULT LIVES IN THIS BROWSER ONLY.</span>
          </span>
          <span class="row-actions">
            <button class="ghost-btn xs primary" type="button" :disabled="disk.busy" @click="openDiskFile">{{ disk.busy ? '…' : 'OPEN' }}</button>
            <button class="ghost-btn xs" type="button" :disabled="disk.busy" @click="createDiskFile">CREATE</button>
            <button class="ghost-btn xs" type="button" :disabled="disk.busy || !disk.bound" @click="saveDiskFile">SAVE NOW</button>
            <button class="ghost-btn xs" type="button" :disabled="disk.busy" @click="exportDiskFile">EXPORT</button>
            <button class="ghost-btn xs" type="button" :disabled="disk.busy" @click="pickImport">IMPORT</button>
            <button v-if="disk.bound" class="ghost-btn xs danger" type="button" :disabled="disk.busy" @click="detachDiskFile">DETACH</button>
          </span>
        </div>
        <input ref="importInput" type="file" accept=".cybermanju,application/octet-stream" class="hidden-input" @change="onImportFile" />

        <div class="form-row" style="margin-top:6px;">
          <input
            v-model="diskPassphrase"
            class="input"
            type="password"
            placeholder="PASSPHRASE (OPTIONAL — ENCRYPTS THE FILE, NEVER STORED)"
            autocomplete="new-password"
            aria-label="File passphrase"
          />
          <span class="text-muted small">{{ disk.supported ? 'FILE SYSTEM ACCESS API — SAVES STRAIGHT TO YOUR DISK' : 'NO FILE SYSTEM ACCESS API — USE EXPORT / IMPORT' }}</span>
        </div>

        <div v-if="disk.needsPassphrase" class="oauth-block">
          <div class="row-between">
            <span class="small">{{ disk.name }} IS ENCRYPTED — ENTER ITS PASSPHRASE TO OPEN</span>
            <button class="ghost-btn xs primary" type="button" :disabled="disk.busy" @click="unlockDisk">UNLOCK & OPEN</button>
          </div>
        </div>
        <div v-else-if="disk.needsPermission" class="oauth-block">
          <div class="row-between">
            <span class="small">{{ disk.name }} IS REMEMBERED — THE BROWSER WANTS ONE CLICK TO RE-OPEN IT</span>
            <button class="ghost-btn xs primary" type="button" :disabled="disk.busy" @click="unlockDisk">ALLOW & OPEN</button>
          </div>
        </div>

        <p v-if="disk.lastMessage" class="note">{{ disk.lastMessage }}</p>
        <p v-if="disk.lastError" class="note err">{{ disk.lastError }}</p>
        <p v-if="disk.bound && disk.dirty" class="note">Changes are written back automatically — SAVE NOW forces it immediately.</p>
      </div>
    </div>

    <!-- Providers -->
    <div class="section">
      <h3 class="section-title"><AppIcon name="solar:cloud-bold" :size="13" /> CLOUD + LOCAL CONNECTIONS</h3>
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
            <button class="ghost-btn xs" type="button" :disabled="probing.has(cfg.id)" @click="probe(cfg)">TEST</button>
            <button class="ghost-btn xs danger" type="button" @click="removeProvider(cfg.id)">DEL</button>
          </span>
        </header>

        <!-- auth status -->
        <div class="auth-row">
          <span class="auth-badge" :class="authClass(cfg.id)">{{ authLabel(cfg.id) }}</span>
          <span v-if="authDetail(cfg.id)" class="text-muted small" :title="authHint(cfg.id)">{{ authDetail(cfg.id) }}</span>
        </div>

        <!-- PKCE OAuth -->
        <div v-if="isOauthCapable(cfg.backendType) && !staticHost" class="oauth-block">
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

        <!-- Supabase-brokered OAuth (static/offline builds: no dashboard) -->
        <div v-if="isOauthCapable(cfg.backendType) && staticHost" class="oauth-block">
          <div class="row-between">
            <span class="small">OAUTH VIA SUPABASE — APPROVE AT THE PROVIDER, TOKEN LANDS HERE</span>
            <button
              class="ghost-btn xs primary"
              type="button"
              :disabled="sbBusy === cfg.id"
              @click="supabaseConnect(cfg)"
            >{{ sbBusy === cfg.id ? 'WAITING…' : 'CONNECT WITH OAUTH' }}</button>
          </div>
          <p v-if="sbMsg[cfg.id]" class="note">{{ sbMsg[cfg.id] }}</p>
          <p v-if="sbBusy === cfg.id" class="note">Approve in the popup — polling for the provider token… <button class="linklike" type="button" @click="cancelSupabase">cancel</button></p>
          <p v-if="!sbConfigured" class="note">Set SUPABASE URL + KEY in Settings → OAUTH first (enable {{ cfg.backendType }} under Supabase → Authentication → Sign-in).</p>
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
            <span class="small">SYSTEM DISKS ON THIS PROVIDER ({{ disksFor(cfg.id).length }}) — EACH IS ONE .CYBERMANJU FILE, MERGED INTO THE VOLUME</span>
          </div>
          <p v-if="disksFor(cfg.id).length === 0" class="note">
            No system disk yet — provision one below and this provider contributes space + compute to the merged volume.
          </p>
          <div v-for="d in disksFor(cfg.id)" :key="d.id" class="disk-row">
            <div class="row-between">
              <span>{{ d.name || d.id.slice(0, 8) }} · <span class="text-muted">{{ d.state }} / {{ d.health }}</span></span>
              <span class="row-actions">
                <button v-if="d.state !== 'attached'" class="ghost-btn xs primary" type="button" :disabled="diskBusy === d.id" @click="attachDisk(d.id, cfg.id)">ATTACH</button>
                <button v-else class="ghost-btn xs" type="button" :disabled="diskBusy === d.id" @click="store.detachDisk(d.id)">DETACH</button>
                <button class="ghost-btn xs" type="button" :disabled="diskBusy === d.id" @click="store.checkDisk(d.id)">CHECK</button>
              </span>
            </div>
            <div class="text-muted small truncate" :title="d.containerPath">FILE {{ d.containerPath }}</div>
            <div class="mini-bar"><div class="mini-used" :style="{ width: `${diskPct(d.usedBytes, d.capacityBytes)}%` }"></div></div>
            <div class="row-between small">
              <span>{{ humanBytes(d.usedBytes) }} / {{ humanBytes(d.capacityBytes) }}</span>
              <span class="row-actions">
                <input v-model.number="resizeMb[d.id]" class="input xs-num" type="number" min="64" max="8192" step="64" :placeholder="String(Math.max(64, Math.round(d.capacityBytes / 1048576)))" :aria-label="`New size MB for ${d.name}`" />
                <span class="text-muted">MB</span>
                <button class="ghost-btn xs" type="button" :disabled="diskBusy === d.id" @click="applyResize(d.id)">APPLY SIZE</button>
              </span>
            </div>
          </div>
          <div class="form-row">
            <label class="small text-muted">{{ disksFor(cfg.id).length === 0 ? 'PROVISION SYSTEM DISK' : 'NEW DISK' }} — {{ newDiskMb[cfg.id] ?? 512 }} MB</label>
            <input v-model.number="newDiskMb[cfg.id]" class="slider" type="range" min="64" max="8192" step="64" :aria-label="`New disk size for ${cfg.name || cfg.backendType}`" />
            <input v-model="newDiskPass[cfg.id]" class="input" type="password" placeholder="PASSPHRASE (ALSO UNLOCKS)" autocomplete="off" :aria-label="`Passphrase for new disk on ${cfg.name || cfg.backendType}`" />
            <button class="ghost-btn xs primary" type="button" :disabled="diskBusy === cfg.id" @click="createDisk(cfg.id)">{{ diskBusy === cfg.id ? 'CREATING…' : disksFor(cfg.id).length === 0 ? 'PROVISION SYSTEM DISK' : 'CREATE & ATTACH' }}</button>
          </div>
        </div>
      </article>
    </div>

    <!-- New provider wizard -->
    <div class="section">
      <h3 class="section-title"><AppIcon name="solar:add-bold" :size="13" /> ADD PROVIDER</h3>
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
import AppIcon from '@/components/AppIcon.vue'
import { computed, onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import { useAppStore } from '@/stores/app'
import { isStaticHost } from '@/composables/useTauri'
import { wasmDbBackend } from '@/composables/useWasmBackend'
import {
  getPendingOAuthConfig,
  identity,
  refreshIdentity,
  setPendingOAuthConfig,
  signInWithPopup,
  signOutIdentity,
  startSupabaseOAuth,
  supabaseConfigured,
  supabaseProviderFor,
  supabaseSession,
  takeProviderTokenStash,
  type OAuthBackend,
} from '@/composables/useSupabase'
import {
  createCybermanjuFile,
  detachCybermanjuFile,
  disk,
  diskSupported,
  exportCybermanjuFile,
  importCybermanjuFile,
  openCybermanjuFile,
  reattachCybermanjuDisk,
  saveCybermanjuFile,
} from '@/composables/useCybermanjuFile'
import { SYNC_BACKEND_INFO, describeSyncError, isOauthCapable } from '@/types'
import type { DiskRow, SyncBackendType, SyncConfig } from '@/types'
import { humanBytes, diskPct } from '@/utils/format'
import {
  authGuidance,
  backendLabel,
  blankCredentialDraft,
  draftToSave,
  needsRepo,
  needsToken,
  overlayDraft,
  refreshDraftFromSaved,
  syncConfigDefaults,
  tokenLabel,
} from '@/utils/providers'
import type { CredentialDraft } from '@/utils/providers'
import { pollUntilTrue } from '@/utils/poll'

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
const probing = ref<Set<string>>(new Set())
const diskBusy = ref<string | null>(null)
const wizBusy = ref(false)
const wizMsg = ref('')
const oauthBusy = ref<string | null>(null)
const oauthAbort = ref<AbortController | null>(null)

const oauthMsg = ref<Record<string, string>>({})
const oauthUrl = ref<Record<string, string>>({})
const quotaMsg = ref<Record<string, string>>({})
const authState = ref<Record<string, { ok: boolean | null; detail: string }>>({})
const sbBusy = ref<string | null>(null)
const sbAbort = ref<AbortController | null>(null)
const sbMsg = ref<Record<string, string>>({})
const sbConfigured = computed(() => supabaseConfigured())

const IDENTITY_PROVIDERS: Array<{ id: OAuthBackend; label: string }> = [
  { id: 'google', label: 'GOOGLE' },
  { id: 'github', label: 'GITHUB' },
  { id: 'gitlab', label: 'GITLAB' },
]
const signInBusy = ref<OAuthBackend | null>(null)
const signInMsg = ref('')
const signingOut = ref(false)
const diskPassphrase = ref('')
const importInput = ref<HTMLInputElement | null>(null)

const newDiskMb = ref<Record<string, number>>({})
const newDiskPass = ref<Record<string, string>>({})
const resizeMb = ref<Record<string, number>>({})

const drafts = reactive<Record<string, CredentialDraft>>({})

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

function draft(cfg: SyncConfig): CredentialDraft {
  let d = drafts[cfg.id]
  if (!d) {
    d = blankCredentialDraft({
      repoName: cfg.repoName ?? '',
      branch: cfg.branch ?? 'main',
      folderId: cfg.folderId ?? '',
      albumId: cfg.albumId ?? '',
      chatId: cfg.chatId ?? '',
      basePath: cfg.basePath ?? '',
      name: cfg.name ?? '',
    })
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
  if (s.ok) return 'CONNECTED'
  return isUnreachable(s.detail) ? 'UNREACHABLE' : 'AUTH FAILED'
}

function authClass(id: string): string {
  const s = authState.value[id]
  if (!s || s.ok === null) return ''
  if (s.ok) return 'ok'
  return isUnreachable(s.detail) ? 'warn' : 'bad'
}

/** Transport failure (offline / CORS-blocked / timeout) is not a verdict
 * on the token — label it so users don't rotate good credentials. */
function isUnreachable(detail: string): boolean {
  return /(^network:|\bCORS\b|blocked|abort|timed? ?out|Failed to fetch|Load failed)/i.test(detail)
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

async function signIn(provider: OAuthBackend) {
  if (signInBusy.value) return
  signInBusy.value = provider
  signInMsg.value = ''
  try {
    const who = await signInWithPopup(provider)
    store.notifySuccess(`Signed in as ${who.name}`)
  } catch (e) {
    signInMsg.value = e instanceof Error ? e.message : String(e)
  } finally {
    signInBusy.value = null
  }
}

async function signOut() {
  signingOut.value = true
  try {
    await signOutIdentity()
    store.notifySuccess('Signed out')
  } finally {
    signingOut.value = false
  }
}

function timeOf(ts: number): string {
  return ts ? new Date(ts).toLocaleTimeString() : ''
}

async function openDiskFile() {
  await openCybermanjuFile()
}

async function createDiskFile() {
  await createCybermanjuFile(diskPassphrase.value)
}

async function saveDiskFile() {
  const ok = await saveCybermanjuFile()
  if (ok) store.notifySuccess(disk.lastMessage || 'Saved')
}

async function exportDiskFile() {
  await exportCybermanjuFile(diskPassphrase.value || undefined)
}

function pickImport() {
  importInput.value?.click()
}

async function onImportFile(ev: Event) {
  const input = ev.target as HTMLInputElement
  const file = input.files?.[0] ?? null
  input.value = ''
  if (!file) return
  await importCybermanjuFile(file)
  if (disk.attached) store.notifySuccess(`${disk.name} imported`)
}

async function unlockDisk() {
  await reattachCybermanjuDisk(diskPassphrase.value)
}

async function detachDiskFile() {
  if (!window.confirm('Detach this file? The vault keeps living in this browser session until the file is opened again.')) return
  await detachCybermanjuFile()
}

async function toggleEnabled(cfg: SyncConfig) {
  if (saving.value) return
  saving.value = cfg.id
  try {
    await store.saveSyncConfig({ ...cfg, enabled: !cfg.enabled })
  } finally {
    saving.value = null
  }
}

async function probe(cfg: SyncConfig) {
  if (probing.value.has(cfg.id)) return
  probing.value.add(cfg.id)
  authState.value[cfg.id] = { ok: null, detail: 'probing…' }
  try {
    const r = await store.probeSyncConnection(overlayDraft(cfg, drafts[cfg.id]))
    authState.value[cfg.id] = { ok: r.ok, detail: r.detail }
  } finally {
    probing.value.delete(cfg.id)
  }
}

async function removeProvider(id: string) {
  if (!window.confirm('Delete this provider connection? Its disks stay in the catalog.')) return
  delete drafts[id]
  delete authState.value[id]
  await store.deleteSyncConfig(id)
}

async function saveCreds(cfg: SyncConfig) {
  if (saving.value) return
  const d = draft(cfg)
  saving.value = cfg.id
  try {
    const saved = await store.saveSyncConfig(draftToSave(cfg, d))
    if (!saved) return
    refreshDraftFromSaved(d, saved)
    const r = await store.probeSyncConnection(overlayDraft(saved, d))
    authState.value[cfg.id] = { ok: r.ok, detail: r.detail }
    if (r.ok) store.notifySuccess('Provider verified — credentials work')
  } finally {
    saving.value = null
  }
}

async function quota(cfg: SyncConfig) {
  const u = await store.fetchSyncUsage(cfg.id)
  quotaMsg.value[cfg.id] = u
    ? `quota: ${[u.totalBytes != null ? `total ${humanBytes(u.totalBytes)}` : null, u.usedBytes != null ? `used ${humanBytes(u.usedBytes)}` : null, u.remainingRequests != null ? `${u.remainingRequests} req left` : null].filter(Boolean).join(' · ') || u.detail}`
    : 'quota unavailable'
}

/** PKCE OAuth: open the provider approval, then poll until credentials land. */
async function oauthConnect(cfg: SyncConfig) {
  // No dashboard behind the static build means no server-side callback to
  // land credentials in — say so immediately instead of polling for 2 min.
  // (If Settings → Remote Dashboard points at a server, this build is a
  // REST client and this branch never runs.)
  if (staticHost) {
    oauthMsg.value[cfg.id] =
      'OAuth needs a dashboard for the server callback — set REMOTE DASHBOARD in Settings to your server for full OAuth here, or paste a token below for the offline vault.'
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
  const abort = new AbortController()
  oauthAbort.value = abort
  try {
    const ok = await pollUntilTrue(
      async () => (await store.probeSyncConnection(cfg)).ok,
      {
        intervalMs: 3000,
        maxAttempts: 40,
        signal: abort.signal,
        onAttempt: (n) => {
          oauthMsg.value[cfg.id] = `Approve in the browser tab — waiting… (${n * 3}s)`
        },
      },
    )
    if (ok) {
      authState.value[cfg.id] = { ok: true, detail: 'OAuth credentials verified' }
      oauthMsg.value[cfg.id] = 'Connected — OAuth credentials verified.'
      store.notifySuccess(`${cfg.name || cfg.backendType}: OAuth connected`)
    } else {
      oauthMsg.value[cfg.id] = 'Timed out waiting for approval (2 min). Retry, or paste a token below.'
    }
  } catch {
    oauthMsg.value[cfg.id] = 'Cancelled.'
  } finally {
    oauthAbort.value = null
    oauthBusy.value = null
  }
}

function cancelOauth() {
  oauthAbort.value?.abort()
  oauthAbort.value = null
  oauthBusy.value = null
}

/**
 * Supabase-brokered OAuth (static builds): Supabase's server does the
 * secret-holding exchange; the provider token lands in our session, we save
 * it into the provider config and probe — CONNECTED.
 */
async function supabaseConnect(cfg: SyncConfig) {
  cancelSupabase()
  if (!supabaseConfigured()) {
    sbMsg.value[cfg.id] =
      'Set SUPABASE URL + KEY in Settings → OAUTH first (and enable this provider under Supabase → Authentication → Sign-in).'
    return
  }
  setPendingOAuthConfig(cfg.id)
  let url = ''
  try {
    ;({ url } = await startSupabaseOAuth(cfg.backendType))
  } catch (e) {
    sbMsg.value[cfg.id] = e instanceof Error ? e.message : String(e)
    return
  }
  const popup = window.open(url, 'cyb_sb_oauth', 'width=620,height=720')
  sbBusy.value = cfg.id
  if (!popup) {
    sbMsg.value[cfg.id] = 'Popup blocked — approving in this tab…'
    window.location.href = url
    return
  }
  sbMsg.value[cfg.id] = 'Approve at the provider in the popup — waiting for the token…'
  const abort = new AbortController()
  sbAbort.value = abort
  try {
    const ok = await pollUntilTrue(
      async () => {
        if (popup.closed) throw new Error('popup-closed')
        let token = ''
        try {
          const session = await supabaseSession()
          token = session?.provider_token ?? ''
        } catch {
          token = ''
        }
        if (!token) return false
        try { popup.close() } catch { /* already gone */ }
        await finalizeSupabaseToken(cfg, token)
        return true
      },
      {
        intervalMs: 2000,
        maxAttempts: 90,
        signal: abort.signal,
        onAttempt: (n) => {
          if (n % 10 === 0) sbMsg.value[cfg.id] = `Approve at the provider in the popup — waiting… (${n * 2}s)`
        },
      },
    )
    if (!ok) sbMsg.value[cfg.id] = 'Timed out waiting for approval (3 min). Retry, or paste a token below.'
  } catch (e) {
    sbMsg.value[cfg.id] =
      e instanceof Error && e.message === 'popup-closed'
        ? 'Popup closed before approval — retry, or paste a token below.'
        : 'Cancelled.'
  } finally {
    sbAbort.value = null
    sbBusy.value = null
  }
}

function cancelSupabase() {
  sbAbort.value?.abort()
  sbAbort.value = null
  sbBusy.value = null
}

async function finalizeSupabaseToken(cfg: SyncConfig, token: string) {
  const saved = await store.saveSyncConfig({ ...cfg, token })
  if (!saved) {
    sbMsg.value[cfg.id] = 'Token received, but saving it failed — retry.'
    setPendingOAuthConfig(null)
    return
  }
  const r = await store.probeSyncConnection({ ...saved, token })
  authState.value[cfg.id] = { ok: r.ok, detail: r.detail }
  if (r.ok) {
    sbMsg.value[cfg.id] = 'Connected — provider token verified.'
    store.notifySuccess(`${cfg.name || cfg.backendType}: OAuth connected via Supabase`)
  } else {
    sbMsg.value[cfg.id] = `Token saved, but verification failed: ${r.detail}`
  }
  setPendingOAuthConfig(null)
}

async function attachDisk(diskId: string, configId: string) {
  if (diskBusy.value) return
  diskBusy.value = diskId
  try {
    // The per-provider passphrase field doubles as the unlock secret —
    // no popup prompts. Empty means "no passphrase".
    await store.attachDisk(diskId, newDiskPass.value[configId] ?? '')
  } finally {
    diskBusy.value = null
  }
}

function clampDiskMb(mb: number | undefined, fallback: number): number {
  if (!mb || !Number.isFinite(mb)) return fallback
  return Math.min(8192, Math.max(64, Math.round(mb)))
}

async function applyResize(diskId: string) {
  const disk = store.disks.find(x => x.id === diskId)
  const currentMb = disk ? Math.max(64, Math.round(disk.capacityBytes / (1024 * 1024))) : 512
  const mb = clampDiskMb(resizeMb.value[diskId], currentMb)
  if (diskBusy.value) return
  diskBusy.value = diskId
  try {
    await store.resizeDisk(diskId, mb * 1024 * 1024)
  } finally {
    diskBusy.value = null
  }
}

async function createDisk(configId: string) {
  if (diskBusy.value) return
  diskBusy.value = configId
  try {
    const mb = clampDiskMb(newDiskMb.value[configId], 512)
    await store.createDisk(configId, mb * 1024 * 1024, newDiskPass.value[configId] ?? '')
    newDiskPass.value[configId] = ''
  } finally {
    diskBusy.value = null
  }
}


function wizToConfig(): Omit<SyncConfig, 'id' | 'createdAt' | 'updatedAt'> {
  return {
    ...syncConfigDefaults(),
    backendType: wiz.backendType,
    name: wiz.name.trim() || undefined,
    basePath: wiz.basePath.trim() || undefined,
    repoName: wiz.repoName.trim() || undefined,
    branch: wiz.branch.trim() || undefined,
    token: wiz.token.trim() || undefined,
    folderId: wiz.folderId.trim() || undefined,
    albumId: wiz.albumId.trim() || undefined,
    chatId: wiz.chatId.trim() || undefined,
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
  disk.supported = diskSupported()
  void refreshIdentity()
  void (async () => {
    await refresh()
    if (!staticHost) return
    void wasmDbBackend().then((b) => {
      dbBackend.value = b
    })
    // Full-redirect resume: the return already stashed the provider token
    // (App boot exchanges ?code=) — finish the link now configs are loaded.
    const stash = takeProviderTokenStash()
    const pending = getPendingOAuthConfig()
    if (stash && pending) {
      const cfg = store.syncConfigs.find(c => c.id === pending)
      if (cfg && supabaseProviderFor(cfg.backendType) === stash.backend) {
        sbMsg.value[cfg.id] = 'Approval received — verifying…'
        await finalizeSupabaseToken(cfg, stash.providerToken)
      } else {
        setPendingOAuthConfig(null)
      }
    }
  })()
})

onBeforeUnmount(() => {
  cancelOauth()
  cancelSupabase()
})
</script>

<style scoped>
.acct-panel {
  height: 100%;
  overflow-y: auto;
  background: var(--ui-surface);
  color: var(--ui-text);
  font-family: var(--ui-font);
  font-size: 13px;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  border-bottom: 1px solid var(--ui-border);
}

.header-left {
  display: flex;
  align-items: center;
  gap: 10px;
}

.icon-acct { color: var(--ui-accent); }

.panel-title {
  margin: 0;
  font-size: 13px;
  letter-spacing: 2px;
}

.header-right { display: flex; gap: 6px; }

.ghost-btn {
  background: transparent;
  border: 1px solid var(--ui-border);
  color: color-mix(in srgb, var(--ui-text) 70%, transparent);
  font-family: inherit;
  font-size: 11px;
  padding: 3px 8px;
  cursor: pointer;
}

.ghost-btn:hover:not(:disabled) { color: var(--ui-text); border-color: var(--ui-border-strong); }
.ghost-btn:disabled { opacity: 0.4; cursor: not-allowed; }
.ghost-btn.primary { color: var(--ui-accent); border-color: color-mix(in srgb, var(--ui-accent) 60%, transparent); }
.ghost-btn.primary:hover:not(:disabled) { color: var(--ui-text); background: var(--ui-accent); border-color: var(--ui-accent); }
.ghost-btn.danger { color: var(--ui-danger); border-color: color-mix(in srgb, var(--ui-danger) 55%, transparent); }
.ghost-btn.danger:hover:not(:disabled) { color: var(--ui-text); background: var(--ui-danger); border-color: var(--ui-danger); }
.ghost-btn.xs { font-size: 10px; padding: 2px 6px; }
.ghost-btn.on { color: var(--ui-accent); border-color: color-mix(in srgb, var(--ui-accent) 60%, transparent); }

.section { padding: 12px; border-bottom: 1px solid var(--ui-border); }

.section-title {
  margin: 0 0 10px;
  font-size: 11px;
  letter-spacing: 1.5px;
  color: color-mix(in srgb, var(--ui-text) 50%, transparent);
}

.df-bar { height: 18px; background: color-mix(in srgb, var(--ui-text) 10%, transparent); border: 1px solid var(--ui-border); }
.df-used { height: 100%; background: linear-gradient(90deg, var(--ui-accent), var(--ui-info)); }
.df-legend { display: flex; flex-wrap: wrap; gap: 14px; margin-top: 8px; font-size: 11px; }

.card { border: 1px solid var(--ui-border); padding: 10px; margin-bottom: 8px; }
.card.clickable { cursor: pointer; }
.card.clickable:hover { border-color: var(--ui-border); }
.card.active { border-color: color-mix(in srgb, var(--ui-accent) 60%, transparent); }
.card.provider { border-width: 1px; }

.cards { display: flex; flex-direction: column; gap: 6px; margin-bottom: 8px; }

.row-between { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
.row-actions { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }

.dot { display: inline-block; width: 8px; height: 8px; margin-right: 6px; border: 1px solid var(--ui-text-2); border-radius: 50%; }
.dot.on { background: var(--ui-accent); border-color: var(--ui-accent); }
.badge-on { font-size: 10px; color: var(--ui-accent); border: 1px solid color-mix(in srgb, var(--ui-accent) 60%, transparent); padding: 1px 6px; }

.auth-row { display: flex; align-items: center; gap: 8px; margin: 8px 0; flex-wrap: wrap; }
.auth-badge { font-size: 10px; font-weight: 700; border: 1px solid var(--ui-border); color: color-mix(in srgb, var(--ui-text) 60%, transparent); padding: 1px 6px; }
.auth-badge.ok { color: var(--ui-accent); border-color: color-mix(in srgb, var(--ui-accent) 60%, transparent); }
.auth-badge.bad { color: var(--ui-danger); border-color: color-mix(in srgb, var(--ui-danger) 60%, transparent); }
.auth-badge.warn { color: var(--ui-warning); border-color: color-mix(in srgb, var(--ui-warning) 75%, transparent); }

.oauth-block { border: 1px dashed var(--ui-border); padding: 8px; margin: 8px 0; }

.cred-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 8px; margin-top: 8px; }
.field { display: flex; flex-direction: column; gap: 4px; font-size: 11px; }
.field.grow { grid-column: 1 / -1; }

.input {
  background: var(--ui-surface);
  border: 1px solid var(--ui-border);
  color: var(--ui-text);
  font-family: inherit;
  font-size: 12px;
  padding: 5px 6px;
  outline: none;
  min-width: 0;
}
.input:focus { border-color: var(--ui-accent); }
.input.xs-num { width: 76px; }

.form-row { display: flex; gap: 8px; margin-top: 8px; flex-wrap: wrap; align-items: center; }
.form-row .input { flex: 1; min-width: 140px; }

.disks-block { margin-top: 10px; border-top: 1px solid var(--ui-border); padding-top: 8px; }
.disk-row { border: 1px solid var(--ui-border); padding: 6px 8px; margin-top: 6px; display: flex; flex-direction: column; gap: 6px; }
.mini-bar { height: 8px; background: color-mix(in srgb, var(--ui-text) 6%, transparent); }
.mini-used { height: 100%; background: var(--ui-accent); }
.slider { flex: 1; min-width: 140px; accent-color: var(--ui-accent); }

.note { margin: 6px 0 0; font-size: 11px; color: var(--ui-info); white-space: pre-wrap; word-break: break-word; }
.static-note {
  border: 1px dashed var(--ui-warning);
  color: var(--ui-warning);
  font-size: 10px;
  line-height: 1.5;
  padding: 8px 10px;
  margin-bottom: 8px;
  letter-spacing: 0.3px;
}
.mono { font-family: inherit; }
.url { word-break: break-all; color: var(--ui-info); }
.linklike { background: none; border: none; color: var(--ui-info); cursor: pointer; font: inherit; text-decoration: underline; padding: 0; }
.small { font-size: 11px; }
.truncate { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.empty { margin: 0 0 8px; font-size: 12px; }
.text-muted { color: color-mix(in srgb, var(--ui-text) 50%, transparent) !important; }

/* identity + .cybermanju disk block */
.note.err { color: var(--ui-danger); }
.hidden-input { position: absolute; width: 1px; height: 1px; opacity: 0; pointer-events: none; }
.avatar { width: 16px; height: 16px; border-radius: 50%; vertical-align: -3px; margin-right: 6px; border: 1px solid var(--ui-border); }
.identity-row { display: inline-flex; align-items: center; }
</style>
