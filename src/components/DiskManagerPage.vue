<script setup lang="ts">
// Cybermanju Drive — disk / volume manager (AGENT-8, item 9)
//
// One merged `df` bar over every `.cybermanju` disk, per-provider cards with
// an adjustable size, and the attach / detach / resize / check controls.
// Disk rows come from AGENT-6's catalog; the merged bar from `/api/os/df`.
import { computed, onMounted, ref } from 'vue'
import { useAppStore } from '@/stores/app'
import { humanBytes, diskPct } from '@/utils/format'

const store = useAppStore()

const configId = ref('')
const createSizeMb = ref(512)
const passphrase = ref('')
const creating = ref(false)
const resizeTarget = ref<string | null>(null)
const resizeMb = ref(512)
const busyId = ref<string | null>(null)
const note = ref('')

const MIN_MB = 64
const MAX_MB = 8192

const df = computed(() => store.osDf)
const usedPct = computed(() => {
  const d = df.value
  if (!d || d.totalBytes === 0) return 0
  return Math.min(100, (d.usedBytes / d.totalBytes) * 100)
})

async function refresh() {
  await Promise.all([store.fetchDisks(), store.fetchOsDf()])
  if (store.syncConfigs.length === 0) await store.fetchSyncConfigs()
}

async function create() {
  if (!configId.value) {
    note.value = 'create: pick a provider configuration first'
    return
  }
  creating.value = true
  const created = await store.createDisk(configId.value, createSizeMb.value * 1024 * 1024, passphrase.value)
  creating.value = false
  note.value = created ? `created ${created.id}` : ''
  if (created) passphrase.value = ''
}

async function attach(diskId: string) {
  busyId.value = diskId
  await store.attachDisk(diskId, passphrase.value)
  busyId.value = null
}

async function detach(diskId: string) {
  busyId.value = diskId
  await store.detachDisk(diskId)
  busyId.value = null
}

function beginResize(diskId: string, capacityBytes: number) {
  resizeTarget.value = diskId
  resizeMb.value = Math.max(MIN_MB, Math.min(MAX_MB, Math.round(capacityBytes / (1024 * 1024))))
}

async function applyResize(diskId: string) {
  await store.resizeDisk(diskId, resizeMb.value * 1024 * 1024)
  resizeTarget.value = null
}

async function check(diskId: string) {
  busyId.value = diskId
  await store.checkDisk(diskId)
  busyId.value = null
}

onMounted(refresh)
</script>

<template>
  <div class="disk-panel">
    <div class="panel-header">
      <div class="header-left">
        <span class="icon-disks">[=]</span>
        <h2 class="panel-title">DISKS &amp; VOLUME</h2>
        <span class="text-muted">{{ df ? `${df.diskCount} DISKS` : '…' }}</span>
      </div>
      <div class="header-right">
        <button class="ghost-btn" type="button" @click="refresh">REFRESH</button>
      </div>
    </div>

    <div class="section">
      <h3 class="section-title">[DF] MERGED VOLUME</h3>
      <div class="df-bar" role="img" :aria-label="`Volume ${usedPct.toFixed(1)} percent used`">
        <div class="df-used" :style="{ width: `${usedPct}%` }"></div>
      </div>
      <div class="df-legend">
        <span><i class="dot used"></i>USED {{ humanBytes(df?.usedBytes ?? 0) }}</span>
        <span><i class="dot free"></i>FREE {{ humanBytes(df?.freeBytes ?? 0) }}</span>
        <span>TOTAL {{ humanBytes(df?.totalBytes ?? 0) }}</span>
        <span class="text-muted">ATTACHED {{ humanBytes(df?.attachedBytes ?? 0) }} · SCRATCH {{ humanBytes(df?.scratchBytes ?? 0) }}</span>
      </div>
      <p class="df-root text-muted">root {{ df?.root ?? '—' }}</p>
    </div>

    <div class="section">
      <h3 class="section-title">[NEW] CREATE DISK</h3>
      <div class="create-row">
        <label class="field">
          <span class="text-muted">PROVIDER</span>
          <select v-model="configId" class="input" aria-label="Provider configuration">
            <option value="" disabled>select a configuration</option>
            <option v-for="cfg in store.syncConfigs" :key="cfg.id" :value="cfg.id">
              {{ cfg.name || cfg.backendType }} · {{ cfg.id }}
            </option>
          </select>
        </label>
        <label class="field grow">
          <span class="text-muted">SIZE — {{ createSizeMb }} MB</span>
          <input
            v-model.number="createSizeMb"
            class="slider"
            type="range"
            :min="MIN_MB"
            :max="MAX_MB"
            :step="64"
            aria-label="Disk size in megabytes"
          />
        </label>
        <label class="field">
          <span class="text-muted">PASSPHRASE</span>
          <input v-model="passphrase" class="input" type="password" autocomplete="off" aria-label="Passphrase" />
        </label>
        <button class="ghost-btn primary" type="button" :disabled="creating" @click="create">
          {{ creating ? 'CREATING…' : 'CREATE' }}
        </button>
      </div>
      <p v-if="note" class="note">{{ note }}</p>
    </div>

    <div class="section">
      <h3 class="section-title">[CARDS] PER-PROVIDER DISKS ({{ store.disks.length }})</h3>

      <div v-if="store.disks.length" class="cards">
        <article v-for="disk in store.disks" :key="disk.id" class="card">
          <header class="card-head">
            <div>
              <span class="card-name">{{ disk.name || disk.id }}</span>
              <span class="card-provider text-muted">{{ disk.provider }}</span>
            </div>
            <span class="health" :class="`health-${disk.health}`">{{ disk.health.toUpperCase() }}</span>
          </header>

          <div class="card-bar">
            <div class="card-used" :style="{ width: `${diskPct(disk.usedBytes, disk.capacityBytes)}%` }"></div>
          </div>
          <div class="card-figures">
            <span>{{ humanBytes(disk.usedBytes) }} / {{ humanBytes(disk.capacityBytes) }}</span>
            <span class="text-muted">{{ diskPct(disk.usedBytes, disk.capacityBytes).toFixed(0) }}% used</span>
          </div>

          <dl class="card-meta">
            <div><dt class="text-muted">STATE</dt><dd>{{ disk.state }}</dd></div>
            <div><dt class="text-muted">CONFIG</dt><dd class="truncate">{{ disk.configId }}</dd></div>
            <div><dt class="text-muted">PATH</dt><dd class="truncate" :title="disk.containerPath">{{ disk.containerPath }}</dd></div>
          </dl>

          <div class="card-actions">
            <button
              v-if="disk.state !== 'attached'"
              class="ghost-btn primary"
              type="button"
              :disabled="busyId === disk.id"
              @click="attach(disk.id)"
            >ATTACH</button>
            <button
              v-else
              class="ghost-btn"
              type="button"
              :disabled="busyId === disk.id"
              @click="detach(disk.id)"
            >DETACH</button>
            <button class="ghost-btn" type="button" @click="beginResize(disk.id, disk.capacityBytes)">RESIZE</button>
            <button class="ghost-btn" type="button" :disabled="busyId === disk.id" @click="check(disk.id)">CHECK</button>
          </div>

          <div v-if="resizeTarget === disk.id" class="resize-row">
            <input
              v-model.number="resizeMb"
              class="slider"
              type="range"
              :min="MIN_MB"
              :max="MAX_MB"
              :step="64"
              :aria-label="`Resize ${disk.name}`"
            />
            <span class="resize-size">{{ resizeMb }} MB</span>
            <button class="ghost-btn primary" type="button" @click="applyResize(disk.id)">APPLY</button>
            <button class="ghost-btn" type="button" @click="resizeTarget = null">CANCEL</button>
          </div>
        </article>
      </div>

      <p v-else class="text-muted empty">
        No disks yet — create one above. Every attached disk grows the merged volume and adds
        provider compute slots to the fan-out pool.
      </p>
    </div>
  </div>
</template>

<style scoped>
.disk-panel {
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

.icon-disks {
  color: #5af78e;
}

.panel-title {
  margin: 0;
  font-size: 13px;
  letter-spacing: 2px;
}

.ghost-btn {
  background: transparent;
  border: 1px solid rgba(255, 255, 255, 0.3);
  color: rgba(255, 255, 255, 0.75);
  font-family: inherit;
  font-size: 11px;
  padding: 4px 9px;
  cursor: pointer;
}

.ghost-btn:hover:not(:disabled) {
  color: #fff;
  border-color: #fff;
}

.ghost-btn:disabled {
  opacity: 0.4;
  cursor: default;
}

.ghost-btn.primary {
  color: #5af78e;
  border-color: rgba(90, 247, 142, 0.6);
}

.ghost-btn.primary:hover:not(:disabled) {
  color: #000;
  background: #5af78e;
  border-color: #5af78e;
}

.ghost-btn.danger {
  color: #ff5f56;
  border-color: rgba(255, 95, 86, 0.55);
}

.ghost-btn.danger:hover:not(:disabled) {
  color: #000;
  background: #ff5f56;
  border-color: #ff5f56;
}

.section {
  padding: 12px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
}

.section-title {
  margin: 0 0 10px;
  font-size: 11px;
  letter-spacing: 1.5px;
  color: rgba(255, 255, 255, 0.55);
}

.df-bar {
  height: 18px;
  background: rgba(255, 255, 255, 0.1);
  border: 1px solid rgba(255, 255, 255, 0.2);
}

.df-used {
  height: 100%;
  background: linear-gradient(90deg, #5af78e, #57c7ff);
}

.df-legend {
  display: flex;
  flex-wrap: wrap;
  gap: 14px;
  margin-top: 8px;
  font-size: 11px;
}

.dot {
  display: inline-block;
  width: 8px;
  height: 8px;
  margin-right: 5px;
}

.dot.used {
  background: #5af78e;
}

.dot.free {
  background: rgba(255, 255, 255, 0.3);
}

.df-root {
  margin: 6px 0 0;
  font-size: 11px;
}

.create-row {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-end;
  gap: 12px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 11px;
  min-width: 160px;
}

.field.grow {
  flex: 1;
  min-width: 200px;
}

.input {
  background: #000;
  border: 1px solid rgba(255, 255, 255, 0.3);
  color: #fff;
  font-family: inherit;
  font-size: 12px;
  padding: 5px 6px;
  outline: none;
}

.input:focus {
  border-color: #5af78e;
}

.slider {
  width: 100%;
  accent-color: #5af78e;
}

.note {
  margin: 8px 0 0;
  font-size: 11px;
  color: #9aedfe;
}

.cards {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 12px;
}

.card {
  border: 1px solid rgba(255, 255, 255, 0.16);
  padding: 10px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.card-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 8px;
}

.card-name {
  display: block;
  font-size: 13px;
}

.card-provider {
  font-size: 11px;
}

.health {
  font-size: 10px;
  padding: 1px 6px;
  border: 1px solid currentColor;
}

.health-ok {
  color: #5af78e;
}

.health-degraded,
.health-repairing {
  color: #f3f99d;
}

.health-failed {
  color: #ff5f56;
}

.card-bar {
  height: 10px;
  background: rgba(255, 255, 255, 0.12);
}

.card-used {
  height: 100%;
  background: #5af78e;
}

.card-figures {
  display: flex;
  justify-content: space-between;
  font-size: 11px;
}

.card-meta {
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 3px;
  font-size: 11px;
}

.card-meta div {
  display: flex;
  gap: 6px;
  min-width: 0;
}

.card-meta dt {
  width: 52px;
  flex: 0 0 auto;
}

.card-meta dd {
  margin: 0;
  min-width: 0;
}

.card-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.resize-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.resize-size {
  font-size: 11px;
  white-space: nowrap;
}

.empty {
  margin: 0;
  font-size: 12px;
}

.text-muted {
  color: rgba(255, 255, 255, 0.5) !important;
}
</style>
