<template>
  <div class="encryption-panel">
    <!-- Header -->
    <div class="panel-header">
      <div class="header-left">
        <Shield :size="22" class="icon-shield" />
        <h2 class="panel-title">Quantum Shield</h2>
      </div>
      <button class="close-btn" @click="$emit('close')">
        <span class="close-x">✕</span>
      </button>
    </div>

    <!-- Encryption Status Card -->
    <div
      class="status-card"
      :class="{ protected: encryptionStatus?.isEncrypted }"
    >
      <div class="status-top">
        <Shield
          v-if="encryptionStatus?.isEncrypted"
          :size="28"
          class="icon-protected"
        />
        <Unlock v-else :size="28" class="icon-unprotected" />
        <span
          class="status-badge"
          :class="encryptionStatus?.isEncrypted ? 'badge-protected' : 'badge-unprotected'"
        >
          {{ encryptionStatus?.isEncrypted ? 'PROTECTED' : 'UNPROTECTED' }}
        </span>
      </div>

      <div class="status-details" v-if="encryptionStatus?.isEncrypted">
        <div class="algo-name">
          <Lock :size="14" />
          <span>{{ encryptionStatus.algorithm || 'Unknown' }}</span>
          <span class="nist-stars">
            <span v-for="n in (encryptionStatus.nistLevel || 0)" :key="n" class="star filled">★</span>
            <span v-for="n in 5 - (encryptionStatus.nistLevel || 0)" :key="'e' + n" class="star empty">☆</span>
          </span>
        </div>
        <div class="status-meta">
          <span class="meta-label">Key ID:</span>
          <span class="mono">{{ encryptionStatus.keyId || '—' }}</span>
        </div>
        <div class="status-meta" v-if="encryptionStatus.encryptedAt">
          <span class="meta-label">Encrypted:</span>
          <span class="mono">{{ formatDate(encryptionStatus.encryptedAt) }}</span>
        </div>
      </div>
      <div class="status-details" v-else>
        <p class="unprotected-msg">
          <AlertTriangle :size="14" />
          No quantum-resistant encryption active. Generate a keypair below.
        </p>
      </div>

      <!-- NIST Level Visualization -->
      <div class="nist-viz">
        <span class="nist-label">NIST Security Level</span>
        <div class="nist-circles">
          <div
            v-for="n in 5"
            :key="n"
            class="nist-circle"
            :class="{ filled: n <= (encryptionStatus?.nistLevel || 0) }"
          >
            <span class="circle-num">{{ n }}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Keypair Generation -->
    <div class="section">
      <h3 class="section-title">
        <Key :size="16" />
        Generate Keypair
      </h3>
      <div class="algo-buttons">
        <button
          v-for="(info, algo) in ENCRYPTION_INFO"
          :key="algo"
          class="algo-btn"
          :style="{ borderColor: info.color, '--algo-color': info.color }"
          @click="handleGenerate(algo as EncryptionAlgo)"
        >
          <div class="algo-top">
            <Zap :size="14" :style="{ color: info.color }" />
            <span class="nist-badge" :style="{ backgroundColor: info.color + '22', color: info.color }">
              L{{ info.nistLevel }}
            </span>
          </div>
          <span class="algo-name">{{ info.name }}</span>
          <span class="algo-desc">{{ info.description }}</span>
        </button>
      </div>
    </div>

    <!-- Active Keys -->
    <div class="section" v-if="encryptionKeys.length > 0">
      <h3 class="section-title">
        <Key :size="16" />
        Active Keys ({{ encryptionKeys.length }})
      </h3>
      <div class="keys-list">
        <div
          v-for="key in encryptionKeys"
          :key="key.id"
          class="key-card"
          :style="{ borderLeftColor: key.color }"
        >
          <div class="key-header">
            <CheckCircle :size="14" :style="{ color: key.color }" />
            <span class="key-algo">{{ key.algorithmDisplay }}</span>
            <span
              class="nist-badge small"
              :style="{ backgroundColor: key.color + '22', color: key.color }"
            >
              L{{ key.nistLevel }}
            </span>
          </div>
          <div class="key-pub-preview mono">{{ key.publicKeyPreview.slice(0, 16) }}…</div>
          <div class="key-date">{{ formatDate(key.createdAt) }}</div>
        </div>
      </div>
    </div>

    <!-- File Encryption -->
    <div class="section" v-if="selectedFile">
      <h3 class="section-title">
        <Lock :size="16" />
        Encrypt Selected File
      </h3>
      <p class="selected-file-name">{{ selectedFile.name }}</p>
      <div class="encrypt-actions">
        <select v-model="selectedAlgo" class="encrypt-select">
          <option v-for="(info, algo) in ENCRYPTION_INFO" :key="algo" :value="algo">
            {{ info.name }} (Level {{ info.nistLevel }})
          </option>
        </select>
        <button class="encrypt-btn" @click="handleEncrypt">
          <Zap :size="14" />
          Encrypt
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useAppStore } from '@/stores/app'
import type { EncryptionAlgo } from '@/types'
import { ENCRYPTION_INFO } from '@/types'
import {
  Shield,
  Lock,
  Unlock,
  Key,
  Plus,
  AlertTriangle,
  CheckCircle,
  Zap,
} from 'lucide-vue-next'

const store = useAppStore()

const emit = defineEmits<{
  close: []
}>()

const encryptionStatus = computed(() => store.encryptionStatus)
const encryptionKeys = computed(() => store.encryptionKeys)
const selectedFile = computed(() => store.selectedFile)

const selectedAlgo = ref<EncryptionAlgo>('kyber1024')

function formatDate(iso: string): string {
  if (!iso) return '—'
  const d = new Date(iso)
  return d.toLocaleDateString('en-US', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  })
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
.encryption-panel {
  width: 400px;
  height: 100%;
  background: var(--cyber-bg-panel, #12121a);
  border-left: 3px solid #000;
  box-shadow: -4px 0 0 0 #000;
  overflow-y: auto;
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 20px;
  font-family: 'Inter', system-ui, sans-serif;
  color: #F5F5F4;
}

/* Scrollbar */
.encryption-panel::-webkit-scrollbar {
  width: 6px;
}
.encryption-panel::-webkit-scrollbar-track {
  background: #0a0a0f;
}
.encryption-panel::-webkit-scrollbar-thumb {
  background: #333;
  border-radius: 3px;
}

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

.icon-shield {
  color: #00FF41;
  filter: drop-shadow(0 0 6px #00FF41);
}

.panel-title {
  font-size: 18px;
  font-weight: 800;
  text-transform: uppercase;
  letter-spacing: 2px;
  color: #00FF41;
  text-shadow: 0 0 10px #00FF41, 0 0 20px rgba(0, 255, 65, 0.3);
  margin: 0;
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
.close-btn:hover {
  border-color: #FF2D6F;
  color: #FF2D6F;
}

/* Status Card */
.status-card {
  background: #1a1a2e;
  border: 3px solid #000;
  box-shadow: 4px 4px 0 0 #000;
  padding: 16px;
  transition: all 0.3s;
}

.status-card.protected {
  border-color: #00FF41;
  box-shadow: 4px 4px 0 0 #00FF41, 0 0 15px rgba(0, 255, 65, 0.2);
}

.status-top {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 12px;
}

.icon-protected {
  color: #00FF41;
  filter: drop-shadow(0 0 8px #00FF41);
}

.icon-unprotected {
  color: #FF6B2B;
  filter: drop-shadow(0 0 8px #FF6B2B);
}

.status-badge {
  font-size: 11px;
  font-weight: 800;
  text-transform: uppercase;
  letter-spacing: 2px;
  padding: 4px 10px;
  border: 2px solid;
}

.badge-protected {
  color: #00FF41;
  border-color: #00FF41;
  background: rgba(0, 255, 65, 0.1);
  box-shadow: 0 0 8px rgba(0, 255, 65, 0.3);
}

.badge-unprotected {
  color: #FF6B2B;
  border-color: #FF6B2B;
  background: rgba(255, 107, 43, 0.1);
}

/* Status Details */
.status-details {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.algo-name {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 700;
  font-size: 14px;
}

.nist-stars {
  font-size: 13px;
  margin-left: 4px;
}

.star.filled {
  color: #FACC15;
  text-shadow: 0 0 4px rgba(250, 204, 21, 0.5);
}

.star.empty {
  color: #4B5563;
}

.status-meta {
  font-size: 12px;
  display: flex;
  gap: 6px;
  color: #9CA3AF;
}

.meta-label {
  color: #6B7280;
  min-width: 60px;
}

.unprotected-msg {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  color: #FF6B2B;
  margin: 0;
}

/* NIST Visualization */
.nist-viz {
  margin-top: 14px;
  padding-top: 12px;
  border-top: 2px dashed #333;
  display: flex;
  align-items: center;
  gap: 14px;
}

.nist-label {
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 1.5px;
  color: #6B7280;
  white-space: nowrap;
}

.nist-circles {
  display: flex;
  gap: 6px;
}

.nist-circle {
  width: 30px;
  height: 30px;
  border-radius: 50%;
  border: 2px solid #333;
  background: #0a0a0f;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.3s;
}

.nist-circle.filled {
  border-color: #00FF41;
  background: rgba(0, 255, 65, 0.2);
  box-shadow: 0 0 8px rgba(0, 255, 65, 0.4);
}

.circle-num {
  font-size: 11px;
  font-weight: 700;
  color: #6B7280;
}

.nist-circle.filled .circle-num {
  color: #00FF41;
}

/* Sections */
.section {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.section-title {
  font-size: 13px;
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

/* Algorithm Buttons */
.algo-buttons {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.algo-btn {
  background: #1a1a2e;
  border: 3px solid;
  box-shadow: 3px 3px 0 0 #000;
  padding: 10px 12px;
  cursor: pointer;
  text-align: left;
  transition: all 0.15s;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.algo-btn:hover {
  transform: translate(-1px, -1px);
  box-shadow: 4px 4px 0 0 #000;
}

.algo-btn:active {
  transform: translate(1px, 1px);
  box-shadow: 2px 2px 0 0 #000;
}

.algo-top {
  display: flex;
  align-items: center;
  gap: 8px;
}

.nist-badge {
  font-size: 10px;
  font-weight: 800;
  padding: 2px 6px;
  border-radius: 2px;
  letter-spacing: 1px;
}

.nist-badge.small {
  font-size: 9px;
}

.algo-name {
  font-size: 13px;
  font-weight: 700;
  color: #F5F5F4;
}

.algo-desc {
  font-size: 11px;
  color: #6B7280;
  line-height: 1.4;
}

/* Keys List */
.keys-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.key-card {
  background: #1a1a2e;
  border: 3px solid #000;
  border-left: 5px solid;
  box-shadow: 3px 3px 0 0 #000;
  padding: 10px 12px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.key-header {
  display: flex;
  align-items: center;
  gap: 8px;
}

.key-algo {
  font-size: 13px;
  font-weight: 700;
}

.key-pub-preview {
  font-size: 11px;
  color: #6B7280;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  background: #0a0a0f;
  padding: 4px 8px;
  border-radius: 2px;
  word-break: break-all;
}

.key-date {
  font-size: 11px;
  color: #6B7280;
}

/* File Encryption */
.selected-file-name {
  font-size: 12px;
  color: #00D4FF;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  background: #0a0a0f;
  padding: 6px 10px;
  border: 2px solid #1a1a2e;
  word-break: break-all;
  margin: 0;
}

.encrypt-actions {
  display: flex;
  gap: 8px;
}

.encrypt-select {
  flex: 1;
  background: #0a0a0f;
  color: #F5F5F4;
  border: 3px solid #000;
  box-shadow: 3px 3px 0 0 #000;
  padding: 8px 10px;
  font-size: 12px;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  cursor: pointer;
  appearance: none;
}

.encrypt-btn {
  background: #00FF41;
  color: #0a0a0f;
  border: 3px solid #000;
  box-shadow: 3px 3px 0 0 #000;
  padding: 8px 16px;
  font-size: 12px;
  font-weight: 800;
  text-transform: uppercase;
  letter-spacing: 1px;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 6px;
  transition: all 0.15s;
}

.encrypt-btn:hover {
  transform: translate(-1px, -1px);
  box-shadow: 4px 4px 0 0 #000;
  filter: brightness(1.1);
}

.encrypt-btn:active {
  transform: translate(1px, 1px);
  box-shadow: 2px 2px 0 0 #000;
}

/* Utility */
.mono {
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 11px;
}
</style>
