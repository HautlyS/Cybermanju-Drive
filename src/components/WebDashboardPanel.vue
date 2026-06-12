<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { DashboardStatus, ApiEndpoint, CYBER } from '@/types'
import { Globe, Server, Copy, QrCode, Wifi, Shield, ExternalLink, Terminal } from 'lucide-vue-next'

// ── State ──
const status = reactive<DashboardStatus>({
  running: false,
  port: 3456,
  url: window.location.port === '3456' ? window.location.origin : 'http://localhost:3456',
  activeConnections: 0,
})
const loading = ref(false)
const copied = ref(false)
const endpointsOpen = ref(false)
const message = ref<{ text: string; type: 'success' | 'error' } | null>(null)

const endpoints = reactive<ApiEndpoint[]>([
  { method: 'GET',    path: '/api/files',               description: 'List all indexed files and folders' },
  { method: 'GET',    path: '/api/files/:id',           description: 'Get single file metadata by ID' },
  { method: 'GET',    path: '/api/accounts',             description: 'List configured storage accounts' },
  { method: 'GET',    path: '/api/collections',          description: 'List all collections' },
  { method: 'GET',    path: '/api/search?q=<query>',     description: 'Full-text search across files' },
  { method: 'GET',    path: '/api/geo-files',            description: 'Files with geolocation data' },
  { method: 'POST',   path: '/api/users/login',          description: 'Authenticate and receive token' },
  { method: 'POST',   path: '/api/users/register',       description: 'Register a new user account' },
  { method: 'GET',    path: '/api/encryption-keys',      description: 'List quantum-safe encryption keys' },
  { method: 'GET',    path: '/api/face-groups',          description: 'List detected face groups' },
  { method: 'GET',    path: '/api/stats',                description: 'Index statistics and health' },
])

function methodColor(method: string): string {
  switch (method) {
    case 'GET':    return '#00FF41'
    case 'POST':   return '#FFB800'
    case 'PUT':    return '#00D4FF'
    case 'DELETE': return '#FF2D6F'
    default:       return '#9CA3AF'
  }
}

function flash(text: string, type: 'success' | 'error' = 'success') {
  message.value = { text, type }
  setTimeout(() => { message.value = null }, 3500)
}

// ── Actions ──
async function checkStatus() {
  try {
    const s = await invoke<DashboardStatus>('dashboard_status')
    Object.assign(status, s)
  } catch (e: any) {
    status.running = false
    flash(String(e) || 'Failed to check dashboard status', 'error')
  }
}

async function startDashboard() {
  loading.value = true
  try {
    const s = await invoke<DashboardStatus>('start_dashboard')
    Object.assign(status, s)
    flash('Web dashboard started')
  } catch (e: any) {
    flash(String(e), 'error')
  } finally {
    loading.value = false
  }
}

async function stopDashboard() {
  loading.value = true
  try {
    await invoke('stop_dashboard')
    status.running = false
    status.activeConnections = 0
    flash('Web dashboard stopped')
  } catch (e: any) {
    flash(String(e), 'error')
  } finally {
    loading.value = false
  }
}

async function copyUrl() {
  try {
    await navigator.clipboard.writeText(status.url)
    copied.value = true
    setTimeout(() => { copied.value = false }, 1500)
  } catch {
    // fallback
    const ta = document.createElement('textarea')
    ta.value = status.url
    document.body.appendChild(ta)
    ta.select()
    document.execCommand('copy')
    document.body.removeChild(ta)
    copied.value = true
    setTimeout(() => { copied.value = false }, 1500)
  }
}

onMounted(() => {
  checkStatus()
})
</script>

<template>
  <div class="webdash-root">
    <!-- Header -->
    <div class="panel-header">
      <Globe :size="22" class="icon-cyan" />
      <h2 class="panel-title">Web Dashboard</h2>
    </div>

    <!-- Flash -->
    <Transition name="flash">
      <div v-if="message" :class="['flash-bar', message.type]">
        {{ message.text }}
      </div>
    </Transition>

    <!-- ═══════════ STATUS CARD ═══════════ -->
    <section class="neo-card">
      <div class="card-head">
        <Server :size="18" />
        <span>Server Status</span>
      </div>

      <div class="status-row">
        <div :class="['status-dot', status.running ? 'online' : 'offline']" />
        <span class="mono-md">{{ status.running ? 'Running' : 'Stopped' }}</span>
        <span class="subtle">on port {{ status.port }}</span>
      </div>

      <!-- URL -->
      <div class="url-row">
        <span class="mono-md url-text">{{ status.url }}</span>
        <button class="icon-btn" title="Copy URL" @click="copyUrl">
          <Copy :size="16" />
        </button>
        <Transition name="flash">
          <span v-if="copied" class="copied-label">Copied!</span>
        </Transition>
      </div>

      <!-- QR placeholder -->
      <div class="qr-area">
        <QrCode :size="20" class="qr-icon" />
        <div class="qr-grid">
          <div v-for="n in 49" :key="n" :class="['qr-cell', (n % 3 === 0 || n % 7 === 0) ? 'filled' : '']" />
        </div>
        <div class="qr-label">Scan to access</div>
      </div>

      <!-- Network info -->
      <div class="network-note">
        <Wifi :size="16" />
        <span>Accessible from any device on your network</span>
      </div>

      <div class="conn-count">
        <span class="subtle">Active connections:</span>
        <span class="mono-sm" :style="{ color: status.activeConnections > 0 ? '#00FF41' : '#6B7280' }">
          {{ status.activeConnections }}
        </span>
      </div>

      <!-- Controls -->
      <div class="dash-controls">
        <button
          v-if="!status.running"
          class="neo-btn green"
          :disabled="loading"
          @click="startDashboard"
        >
          <Server :size="16" />
          {{ loading ? 'Starting...' : 'Start Dashboard' }}
        </button>
        <button
          v-else
          class="neo-btn danger"
          :disabled="loading"
          @click="stopDashboard"
        >
          <Server :size="16" />
          {{ loading ? 'Stopping...' : 'Stop Dashboard' }}
        </button>
      </div>
    </section>

    <!-- ═══════════ API ENDPOINTS ═══════════ -->
    <section class="neo-card">
      <div class="card-head clickable" @click="endpointsOpen = !endpointsOpen">
        <Terminal :size="18" />
        <span>API Endpoints Reference</span>
        <span class="toggle-arrow" :class="{ open: endpointsOpen }">▾</span>
      </div>

      <Transition name="slide">
        <div v-if="endpointsOpen" class="endpoint-list">
          <div v-for="(ep, i) in endpoints" :key="i" class="ep-row">
            <span class="ep-method" :style="{ color: methodColor(ep.method), borderColor: methodColor(ep.method) }">
              {{ ep.method }}
            </span>
            <code class="ep-path">{{ ep.path }}</code>
            <span class="ep-desc">{{ ep.description }}</span>
          </div>
        </div>
      </Transition>
    </section>

    <!-- ═══════════ CONNECTION INFO ═══════════ -->
    <section class="neo-card">
      <div class="card-head">
        <Shield :size="18" />
        <span>Connection Info</span>
      </div>

      <div class="info-grid">
        <div class="info-item">
          <span class="info-label">CORS</span>
          <span class="info-val on">Enabled</span>
        </div>
        <div class="info-item">
          <span class="info-label">Format</span>
          <span class="info-val">JSON API</span>
        </div>
        <div class="info-item">
          <span class="info-label">Local Auth</span>
          <span class="info-val off">Not Required</span>
        </div>
        <div class="info-item">
          <span class="info-label">Protocol</span>
          <span class="info-val">HTTP</span>
        </div>
      </div>

      <div class="security-note">
        <ExternalLink :size="14" />
        <span>For remote access, use a reverse proxy with TLS termination (e.g. Caddy, Nginx + Let's Encrypt).</span>
      </div>
    </section>
  </div>
</template>

<style scoped>
.webdash-root {
  display: flex;
  flex-direction: column;
  gap: 20px;
  padding: 20px;
  height: 100%;
  overflow-y: auto;
  font-family: system-ui, -apple-system, sans-serif;
  color: #F5F5F4;
}

/* ── Header ── */
.panel-header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding-bottom: 12px;
  border-bottom: 3px solid #000;
}
.panel-title {
  font-size: 18px;
  font-weight: 800;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: #00D4FF;
  text-shadow: 0 0 12px rgba(0, 212, 255, 0.5);
}
.icon-cyan {
  color: #00D4FF;
  filter: drop-shadow(0 0 6px rgba(0, 212, 255, 0.6));
}

/* ── Flash ── */
.flash-bar {
  padding: 8px 14px;
  border: 3px solid #000;
  font-family: 'Courier New', monospace;
  font-size: 13px;
  background: #1a1a2e;
  box-shadow: 4px 4px 0 #000;
}
.flash-bar.success { border-color: #00FF41; color: #00FF41; }
.flash-bar.error   { border-color: #FF2D6F; color: #FF2D6F; }
.flash-enter-active, .flash-leave-active { transition: opacity 0.25s, transform 0.25s; }
.flash-enter-from, .flash-leave-to { opacity: 0; transform: translateY(-6px); }

/* ── Neo Card ── */
.neo-card {
  background: #1a1a2e;
  border: 3px solid #000;
  box-shadow: 4px 4px 0 #000;
  padding: 16px;
}
.card-head {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 700;
  font-size: 14px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: 14px;
}
.card-head.clickable {
  cursor: pointer;
  user-select: none;
}
.card-head .toggle-arrow {
  margin-left: auto;
  font-size: 16px;
  transition: transform 0.2s;
  color: #9CA3AF;
}
.card-head .toggle-arrow.open {
  transform: rotate(0deg);
}

/* ── Helpers ── */
.mono-sm { font-family: 'Courier New', monospace; font-size: 13px; }
.mono-md { font-family: 'Courier New', monospace; font-size: 14px; font-weight: 600; }
.muted  { color: #6B7280; }
.subtle { color: #9CA3AF; font-size: 12px; }

/* ── Status ── */
.status-row {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 16px;
}
.status-dot {
  width: 12px;
  height: 12px;
  border: 2px solid #000;
  box-shadow: 2px 2px 0 #000;
}
.status-dot.online {
  background: #00FF41;
  box-shadow: 2px 2px 0 #000, 0 0 8px rgba(0, 255, 65, 0.5);
}
.status-dot.offline {
  background: #FF2D6F;
  box-shadow: 2px 2px 0 #000, 0 0 8px rgba(255, 45, 111, 0.4);
}

/* ── URL row ── */
.url-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  background: #12121a;
  border: 3px solid #000;
  box-shadow: 4px 4px 0 #000;
  margin-bottom: 16px;
}
.url-text {
  flex: 1;
  color: #00D4FF;
  word-break: break-all;
}
.copied-label {
  font-size: 11px;
  font-weight: 700;
  color: #00FF41;
  text-transform: uppercase;
}
.flash-enter-active, .flash-leave-active { transition: opacity 0.2s; }
.flash-enter-from, .flash-leave-to { opacity: 0; }

/* ── QR area ── */
.qr-area {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  margin-bottom: 16px;
  padding: 16px;
  background: #F5F5F4;
  border: 3px solid #000;
  box-shadow: 4px 4px 0 #000;
}
.qr-icon {
  color: #000;
}
.qr-grid {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  gap: 3px;
  width: 140px;
  height: 140px;
}
.qr-cell {
  background: #e5e5e5;
  border: 1px solid #d4d4d4;
  border-radius: 1px;
}
.qr-cell.filled {
  background: #000;
  border-color: #000;
}
.qr-label {
  font-family: 'Courier New', monospace;
  font-size: 11px;
  color: #000;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 1px;
}

/* ── Network note ── */
.network-note {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  background: #12121a;
  border: 2px solid #00D4FF;
  box-shadow: 3px 3px 0 #000;
  margin-bottom: 10px;
  font-size: 13px;
  color: #00D4FF;
}
.conn-count {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 16px;
}

/* ── Buttons ── */
.neo-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 10px 18px;
  font-family: system-ui, sans-serif;
  font-size: 13px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  border: 3px solid #000;
  box-shadow: 4px 4px 0 #000;
  cursor: pointer;
  transition: transform 0.1s, box-shadow 0.1s;
}
.neo-btn:hover:not(:disabled) {
  transform: translate(1px, 1px);
  box-shadow: 3px 3px 0 #000;
}
.neo-btn:active:not(:disabled) {
  transform: translate(2px, 2px);
  box-shadow: 2px 2px 0 #000;
}
.neo-btn:disabled { opacity: 0.4; cursor: not-allowed; }
.neo-btn.green  { background: #00FF41; color: #000; }
.neo-btn.danger { background: #FF2D6F; color: #fff; }

.dash-controls {
  margin-top: 4px;
}

.icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: 2px solid #000;
  box-shadow: 2px 2px 0 #000;
  padding: 6px;
  cursor: pointer;
  color: #F5F5F4;
  transition: transform 0.1s, box-shadow 0.1s;
}
.icon-btn:hover {
  transform: translate(1px, 1px);
  box-shadow: 1px 1px 0 #000;
  color: #00D4FF;
  border-color: #00D4FF;
}

/* ── Endpoints ── */
.slide-enter-active, .slide-leave-active {
  transition: max-height 0.3s ease, opacity 0.25s;
  overflow: hidden;
}
.slide-enter-from, .slide-leave-to {
  max-height: 0;
  opacity: 0;
}
.slide-enter-to, .slide-leave-from {
  max-height: 800px;
  opacity: 1;
}

.endpoint-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.ep-row {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 8px 10px;
  background: #12121a;
  border: 2px solid #000;
  box-shadow: 2px 2px 0 #000;
}
.ep-method {
  font-family: 'Courier New', monospace;
  font-size: 11px;
  font-weight: 800;
  padding: 2px 6px;
  border: 2px solid;
  background: #1a1a2e;
  flex-shrink: 0;
  width: 48px;
  text-align: center;
  letter-spacing: 0.5px;
}
.ep-path {
  font-family: 'Courier New', monospace;
  font-size: 13px;
  color: #00D4FF;
  white-space: nowrap;
  flex-shrink: 0;
}
.ep-desc {
  font-size: 12px;
  color: #9CA3AF;
}

/* ── Connection info ── */
.info-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
  margin-bottom: 16px;
}
.info-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  background: #12121a;
  border: 2px solid #000;
  box-shadow: 2px 2px 0 #000;
}
.info-label {
  font-size: 12px;
  text-transform: uppercase;
  color: #9CA3AF;
  font-weight: 600;
}
.info-val {
  font-family: 'Courier New', monospace;
  font-size: 13px;
  font-weight: 700;
}
.info-val.on  { color: #00FF41; }
.info-val.off { color: #FFB800; }

/* ── Security note ── */
.security-note {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 10px 14px;
  background: #12121a;
  border: 2px solid #FFB800;
  box-shadow: 3px 3px 0 #000;
  font-size: 12px;
  color: #FFB800;
  line-height: 1.5;
}
.security-note svg {
  flex-shrink: 0;
  margin-top: 2px;
}

/* ── Scrollbar ── */
.webdash-root::-webkit-scrollbar { width: 8px; }
.webdash-root::-webkit-scrollbar-track { background: #0a0a0f; }
.webdash-root::-webkit-scrollbar-thumb { background: #1a1a2e; border: 2px solid #000; }
</style>