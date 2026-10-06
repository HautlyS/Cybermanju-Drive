<template>
  <div class="encryption-panel">
    <div class="panel-header">
      <div class="header-left">
        <span class="icon-shield"><AppIcon name="solar:shield-check-bold" /></span>
        <h2 class="panel-title">QUANTUM SHIELD</h2>
      </div>
      <button class="close-btn" @click="$emit('close')" aria-label="CLOSE"><AppIcon name="solar:close-bold" :size="13" /></button>
    </div>

    <div class="status-card" :class="{ protected: encryptionStatus?.isEncrypted }">
      <div class="status-top">
        <span class="status-badge" :class="encryptionStatus?.isEncrypted ? 'badge-protected' : 'badge-unprotected'">
          {{ encryptionStatus?.isEncrypted ? 'PROTECTED' : 'UNPROTECTED' }}
        </span>
      </div>

      <div class="status-details" v-if="encryptionStatus?.isEncrypted">
        <div class="algo-name">
          <span>{{ encryptionStatus.algorithm || 'UNKNOWN' }}</span>
          <span class="nist-stars">
            <span v-for="n in (encryptionStatus.nistLevel || 0)" :key="n" class="star filled">*</span>
            <span v-for="n in 5 - (encryptionStatus.nistLevel || 0)" :key="'e' + n" class="star empty">o</span>
          </span>
        </div>
        <div class="status-meta">
          <span class="meta-label">KEY ID:</span>
          <span class="mono">{{ encryptionStatus.keyId || '--' }}</span>
        </div>
        <div class="status-meta" v-if="encryptionStatus.encryptedAt">
          <span class="meta-label">ENCRYPTED:</span>
          <span class="mono">{{ formatDate(encryptionStatus.encryptedAt) }}</span>
        </div>
      </div>
      <div class="status-details" v-else>
        <p class="unprotected-msg">NO QUANTUM-RESISTANT ENCRYPTION ACTIVE. GENERATE A KEYPAIR BELOW.</p>
      </div>

      <div class="nist-viz">
        <span class="nist-label">NIST LEVEL</span>
        <div class="nist-circles">
          <div v-for="n in 5" :key="n" class="nist-circle" :class="{ filled: n <= (encryptionStatus?.nistLevel || 0) }">
            <span class="circle-num">{{ n }}</span>
          </div>
        </div>
      </div>
    </div>

    <div class="web-note" :class="{ info: !webLocked }">{{ cryptoNote }}</div>

    <div class="section">
      <h3 class="section-title"><AppIcon name="solar:key-bold" :size="13" /> GENERATE KEYPAIR</h3>
      <div class="algo-buttons">
        <button v-for="(info, algo) in ENCRYPTION_INFO" :key="algo" class="algo-btn" :disabled="webLocked" :title="webLocked ? 'Needs the desktop app or offline build' : 'Generate ' + info.name" @click="handleGenerate(algo as EncryptionAlgo)">
          <div class="algo-top">
            <span class="nist-badge">L{{ info.nistLevel }}</span>
          </div>
          <span class="algo-name">{{ info.name }}</span>
          <span class="algo-desc text-muted">{{ info.description }}</span>
        </button>
      </div>
    </div>

    <div class="section" v-if="encryptionKeys.length > 0">
      <h3 class="section-title"><AppIcon name="solar:key-bold" :size="13" /> ACTIVE KEYS ({{ encryptionKeys.length }})</h3>
      <div class="keys-list">
        <div v-for="key in encryptionKeys" :key="key.id" class="key-card">
          <div class="key-header">
            <span class="key-algo">{{ key.algorithmDisplay }}</span>
            <span class="nist-badge small">L{{ key.nistLevel }}</span>
          </div>
          <div class="key-pub-preview mono">{{ key.publicKeyPreview.slice(0, 16) }}..</div>
          <div class="key-date text-muted">{{ formatDate(key.createdAt) }}</div>
        </div>
      </div>
    </div>

    <div class="section" v-if="selectedFile">
      <h3 class="section-title"><AppIcon name="solar:lock-bold" :size="13" /> ENCRYPT SELECTED FILE</h3>
      <p class="selected-file-name">{{ selectedFile.name }}</p>
      <div class="encrypt-actions">
        <select v-model="selectedAlgo" class="encrypt-select">
          <option v-for="(info, algo) in ENCRYPTION_INFO" :key="algo" :value="algo">{{ info.name }} (L{{ info.nistLevel }})</option>
        </select>
        <button class="encrypt-btn" :disabled="webLocked" :title="webLocked ? 'Needs the desktop app or offline build' : 'Encrypt file'" @click="handleEncrypt"><AppIcon name="solar:lock-bold" :size="14" /></button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import AppIcon from '@/components/AppIcon.vue'
import { ref, computed } from 'vue'
import { useAppStore } from '@/stores/app'
import { isStaticHost } from '@/composables/useTauri'
import type { EncryptionAlgo } from '@/types'
import { ENCRYPTION_INFO } from '@/types'

const store = useAppStore()
const emit = defineEmits<{ close: [] }>()

// Static/offline build: the wasm pack runs the real ciphers, keys live in
// `.cybermanju`. Dashboard build: no crypto endpoint exists there.
const staticWasm = isStaticHost()
const webLocked = computed(() => !staticWasm)
const cryptoNote = computed(() =>
  webLocked
    ? 'ENCRYPTION OPS NEED THE TAURI DESKTOP APP OR THE OFFLINE BROWSER BUILD — THIS DASHBOARD BUILD SERVES NO CRYPTO ENDPOINT. STATUS + KEY LIST ABOVE ARE LIVE.'
    : 'KEYS AND PER-FILE METADATA LIVE INSIDE .CYBERMANJU. BYTES ARE SEALED WITH CHACHA20-POLY1305 (HKDF-DERIVED FROM YOUR KEYPAIR); THE ML-KEM / FRODO / AES SLOTS USE THE NEAREST WASM CIPHER AND SAY SO.'
)

const encryptionStatus = computed(() => store.encryptionStatus)
const encryptionKeys = computed(() => store.encryptionKeys)
const selectedFile = computed(() => store.selectedFile)

const selectedAlgo = ref<EncryptionAlgo>('kyber1024')

function formatDate(iso: string): string {
  if (!iso) return '--'
  const d = new Date(iso)
  return d.toLocaleDateString('en-US', { year: 'numeric', month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })
}

async function handleGenerate(algo: EncryptionAlgo) {
  await store.generateKeypair(algo)
}

async function handleEncrypt() {
  if (!store.selectedFileId) return
  await store.encryptFile(store.selectedFileId, selectedAlgo.value)
}
</script>

<style scoped>
.web-note {
  border: 1px dashed var(--ui-warning);
  color: var(--ui-warning);
  font-size: 9px;
  line-height: 1.5;
  padding: 8px 10px;
  letter-spacing: 0.3px;
}
.web-note.info {
  border-color: color-mix(in srgb, var(--ui-accent) 60%, transparent);
  color: var(--ui-accent);
}

.algo-btn:disabled,
.encrypt-btn:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}
.encryption-panel {
  width: 400px;
  height: 100%;
  background: var(--ui-surface);
  border-left: 1px solid var(--ui-border);
  overflow-y: auto;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  font-family: var(--ui-font);
  color: var(--ui-text);
}

.encryption-panel::-webkit-scrollbar { width: 4px; }
.encryption-panel::-webkit-scrollbar-track { background: var(--ui-surface); }
.encryption-panel::-webkit-scrollbar-thumb { background: var(--ui-glass-2); }

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-bottom: 10px;
  border-bottom: 1px solid var(--ui-border);
}

.header-left {
  display: flex;
  align-items: center;
  gap: 8px;
}

.icon-shield {
  font-family: var(--ui-font);
  font-size: 16px;
  color: var(--ui-text);
}

.panel-title {
  font-size: 14px;
  font-weight: 800;
  letter-spacing: 1px;
  color: var(--ui-text);
  margin: 0;
}

.close-btn {
  background: none;
  border: 1px solid var(--ui-border);
  color: var(--ui-text);
  cursor: pointer;
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 11px;
  font-family: var(--ui-font);
  font-weight: 700;
}

.close-btn:hover {
  background: var(--ui-glass-2);
  color: var(--ui-text);
}

.status-card {
  border: 1px solid var(--ui-border);
  padding: 12px;
  background: var(--ui-surface);
}

.status-card.protected {
  border-width: 3px;
}

.status-top {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 10px;
}

.status-badge {
  font-size: 10px;
  font-weight: 800;
  letter-spacing: 1px;
  padding: 3px 8px;
  border: 1px solid var(--ui-border);
  color: var(--ui-text);
}

.badge-protected {
  background: var(--ui-glass-2);
  color: var(--ui-text);
}

.badge-unprotected {
  background: var(--ui-surface);
  color: var(--ui-text);
}

.status-details {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.algo-name {
  display: flex;
  align-items: center;
  gap: 6px;
  font-weight: 700;
  font-size: 12px;
}

.nist-stars {
  font-size: 11px;
}

.star.filled { color: var(--ui-text); }
.star.empty { color: color-mix(in srgb, var(--ui-text) 35%, transparent); }

.status-meta {
  font-size: 10px;
  display: flex;
  gap: 4px;
  color: color-mix(in srgb, var(--ui-text) 70%, transparent);
}

.meta-label {
  color: color-mix(in srgb, var(--ui-text) 50%, transparent);
  min-width: 50px;
}

.unprotected-msg {
  font-size: 11px;
  color: color-mix(in srgb, var(--ui-text) 70%, transparent);
  margin: 0;
}

.nist-viz {
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px solid var(--ui-hairline);
  display: flex;
  align-items: center;
  gap: 10px;
}

.nist-label {
  font-size: 9px;
  letter-spacing: 1px;
  color: color-mix(in srgb, var(--ui-text) 50%, transparent);
  white-space: nowrap;
}

.nist-circles {
  display: flex;
  gap: 4px;
}

.nist-circle {
  width: 24px;
  height: 24px;
  border: 1px solid var(--ui-hairline);
  background: var(--ui-surface);
  display: flex;
  align-items: center;
  justify-content: center;
}

.nist-circle.filled {
  border-color: var(--ui-border-strong);
  background: var(--ui-glass-2);
}

.nist-circle.filled .circle-num {
  color: var(--ui-text);
}

.circle-num {
  font-size: 10px;
  font-weight: 700;
  color: color-mix(in srgb, var(--ui-text) 50%, transparent);
}

.section {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.section-title {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 1px;
  color: color-mix(in srgb, var(--ui-text) 60%, transparent);
  margin: 0;
  display: flex;
  align-items: center;
  gap: 6px;
  padding-bottom: 4px;
  border-bottom: 1px solid var(--ui-hairline);
}

.algo-buttons {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.algo-btn {
  background: var(--ui-surface);
  border: 1px solid var(--ui-border);
  padding: 8px 10px;
  cursor: pointer;
  text-align: left;
  display: flex;
  flex-direction: column;
  gap: 3px;
  color: var(--ui-text);
  font-family: var(--ui-font);
}

.algo-btn:hover {
  background: var(--ui-glass-2);
  color: var(--ui-text);
}

.algo-btn:hover .algo-desc { color: var(--ui-text) !important; }

.algo-top {
  display: flex;
  align-items: center;
  gap: 6px;
}

.nist-badge {
  font-size: 9px;
  font-weight: 800;
  padding: 1px 4px;
  border: 1px solid var(--ui-border-strong);
  color: var(--ui-text);
}

.nist-badge.small { font-size: 8px; }

.algo-name {
  font-size: 11px;
  font-weight: 700;
}

.algo-desc {
  font-size: 10px;
  line-height: 1.3;
}

.keys-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.key-card {
  border: 1px solid var(--ui-border);
  padding: 8px 10px;
  display: flex;
  flex-direction: column;
  gap: 3px;
  background: var(--ui-surface);
}

.key-header {
  display: flex;
  align-items: center;
  gap: 6px;
}

.key-algo {
  font-size: 11px;
  font-weight: 700;
}

.key-pub-preview {
  font-size: 10px;
  color: color-mix(in srgb, var(--ui-text) 50%, transparent);
  background: color-mix(in srgb, var(--ui-text) 6%, transparent);
  padding: 3px 6px;
  word-break: break-all;
}

.key-date {
  font-size: 10px;
}

.selected-file-name {
  font-size: 11px;
  color: var(--ui-text);
  background: color-mix(in srgb, var(--ui-text) 6%, transparent);
  padding: 4px 8px;
  border: 1px solid var(--ui-hairline);
  word-break: break-all;
  margin: 0;
}

.encrypt-actions {
  display: flex;
  gap: 6px;
}

.encrypt-select {
  flex: 1;
  background: var(--ui-surface);
  color: var(--ui-text);
  border: 1px solid var(--ui-border);
  padding: 6px 8px;
  font-size: 10px;
  font-family: var(--ui-font);
  cursor: pointer;
}

.encrypt-btn {
  background: var(--ui-glass-2);
  color: var(--ui-text);
  border: 1px solid var(--ui-border);
  padding: 6px 12px;
  font-size: 10px;
  font-weight: 800;
  cursor: pointer;
  font-family: var(--ui-font);
}

.encrypt-btn:hover {
  background: var(--ui-surface);
  color: var(--ui-text);
}

.mono { font-family: var(--ui-font); }
.text-muted { color: color-mix(in srgb, var(--ui-text) 50%, transparent) !important; }
</style>
