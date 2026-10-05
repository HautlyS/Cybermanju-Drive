<script setup lang="ts">
// Cybermanju Drive — process table (AGENT-8, item 10)
//
// Fed by `GET /api/os/ps` and `GET /api/os/top`; every control goes through
// the same syscall boundary the terminal uses (`kill <id>`, `compute run …`).
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useAppStore } from '@/stores/app'
import type { OsTask } from '@/types'

const store = useAppStore()
const refreshMs = 2000
let timer = 0

const jobPath = ref('/')
const starting = ref(false)
const note = ref('')

const tasks = computed<OsTask[]>(() => store.osPs?.tasks ?? store.osTop?.tasks ?? [])
const counts = computed(() => store.osTop?.counts ?? store.osPs?.counts ?? null)
const load = computed(() => store.osTop?.load ?? null)
const mem = computed(() => store.osTop?.mem ?? null)

function human(bytes: number): string {
  if (!bytes) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let value = bytes
  let unit = 0
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024
    unit += 1
  }
  return `${value >= 10 || unit === 0 ? value.toFixed(0) : value.toFixed(1)} ${units[unit]}`
}

function clock(ms: number): string {
  const total = Math.floor(ms / 1000)
  const h = Math.floor(total / 3600)
  const m = Math.floor((total % 3600) / 60)
  const s = total % 60
  return h > 0 ? `${h}h${m}m` : m > 0 ? `${m}m${s}s` : `${s}s`
}

function pct(task: OsTask): number {
  return Math.max(0, Math.min(100, Math.round(task.progress * 100)))
}

function kill(task: OsTask) {
  void store.killOsTask(task.id)
}

async function startJob(job: string) {
  starting.value = true
  const result = await store.runComputeJob(job, jobPath.value || '/')
  starting.value = false
  note.value = result.output
}

async function refresh() {
  await Promise.all([store.fetchOsPs(), store.fetchOsTop(), store.fetchOsWorkers()])
}

onMounted(() => {
  void refresh()
  if (store.osJobs.length === 0) void store.fetchOsJobs()
  timer = window.setInterval(() => {
    void refresh()
  }, refreshMs)
})

onBeforeUnmount(() => {
  if (timer) window.clearInterval(timer)
})
</script>

<template>
  <div class="process-panel">
    <div class="panel-header">
      <div class="header-left">
        <span class="icon-processes">[%]</span>
        <h2 class="panel-title">TASKS</h2>
        <span class="text-muted">{{ counts ? `${counts.running} RUNNING / ${counts.total} TOTAL` : '…' }}</span>
      </div>
      <div class="header-right">
        <button class="ghost-btn" type="button" @click="refresh">REFRESH</button>
      </div>
    </div>

    <div class="stats-row">
      <div class="stat">
        <span class="stat-key">LOAD</span>
        <span class="stat-val">
          {{ load && load.source === 'proc' ? `${load.load1} ${load.load5} ${load.load15}` : 'n/a' }}
        </span>
      </div>
      <div class="stat">
        <span class="stat-key">CPU</span>
        <span class="stat-val">{{ store.osTop ? store.osTop.cpuPercent.toFixed(1) : '0.0' }}%</span>
      </div>
      <div class="stat">
        <span class="stat-key">RSS</span>
        <span class="stat-val">{{ mem ? human(mem.rssBytes) : '—' }}</span>
      </div>
      <div class="stat">
        <span class="stat-key">UP</span>
        <span class="stat-val">{{ store.osTop ? clock(store.osTop.uptimeMs) : '—' }}</span>
      </div>
      <div class="stat">
        <span class="stat-key">WORKERS</span>
        <span class="stat-val">
          {{ store.osWorkers ? `${store.osWorkers.total} (${store.osWorkers.localThreads} local + ${store.osWorkers.providerSlots} provider)` : '—' }}
        </span>
      </div>
    </div>

    <div class="section">
      <h3 class="section-title">[PS] PROCESS TABLE</h3>
      <table v-if="tasks.length" class="task-table">
        <thead>
          <tr>
            <th>PID</th>
            <th>KIND</th>
            <th>NAME</th>
            <th>STATE</th>
            <th>PROGRESS</th>
            <th>PROVIDER</th>
            <th>BYTES</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="task in tasks" :key="task.id">
            <td>{{ task.id }}</td>
            <td>{{ task.kind }}</td>
            <td class="truncate name-cell" :title="task.name">{{ task.name }}</td>
            <td>
              <span class="state" :class="`state-${task.state}`">{{ task.state.toUpperCase() }}</span>
            </td>
            <td class="progress-cell">
              <span class="bar"><span class="bar-fill" :style="{ width: `${pct(task)}%` }"></span></span>
              <span class="pct">{{ pct(task) }}%</span>
            </td>
            <td>{{ task.provider }}</td>
            <td>{{ human(task.bytes) }}</td>
            <td>
              <button
                v-if="task.state === 'running' || task.state === 'pending'"
                class="ghost-btn danger"
                type="button"
                :aria-label="`Kill task ${task.id}`"
                @click="kill(task)"
              >KILL</button>
              <span v-else class="text-muted">—</span>
            </td>
          </tr>
        </tbody>
      </table>
      <p v-else class="text-muted empty">No tasks yet — start one below, or run `compute run …` in cybsh.</p>
    </div>

    <div class="section">
      <h3 class="section-title">[FANOUT] COMPUTE JOBS</h3>
      <div class="jobs">
        <div v-for="job in store.osJobs" :key="job.name" class="job">
          <div class="job-info">
            <span class="job-name">{{ job.name }}</span>
            <span class="text-muted">{{ job.description }}</span>
          </div>
          <button
            class="ghost-btn"
            type="button"
            :disabled="starting"
            :aria-label="`Run ${job.name}`"
            @click="startJob(job.name)"
          >RUN</button>
        </div>
        <p v-if="!store.osJobs.length" class="text-muted empty">Catalogue not loaded.</p>
      </div>
      <label class="path-row">
        <span class="text-muted">PATH</span>
        <input v-model="jobPath" class="path-input" type="text" spellcheck="false" aria-label="Job path" />
      </label>
      <p v-if="note" class="note">{{ note }}</p>
    </div>
  </div>
</template>

<style scoped>
.process-panel {
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

.icon-processes {
  color: #9aedfe;
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
  padding: 3px 8px;
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

.ghost-btn.danger {
  color: #ff5f56;
  border-color: rgba(255, 95, 86, 0.6);
}

.ghost-btn.danger:hover {
  color: #000;
  background: #ff5f56;
  border-color: #ff5f56;
}

.stats-row {
  display: flex;
  flex-wrap: wrap;
  gap: 18px;
  padding: 10px 12px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
}

.stat {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.stat-key {
  font-size: 10px;
  color: rgba(255, 255, 255, 0.45);
  letter-spacing: 1px;
}

.stat-val {
  font-size: 13px;
}

.section {
  padding: 12px;
}

.section-title {
  margin: 0 0 8px;
  font-size: 11px;
  letter-spacing: 1.5px;
  color: rgba(255, 255, 255, 0.55);
}

.task-table {
  width: 100%;
  border-collapse: collapse;
}

.task-table th {
  text-align: left;
  font-size: 10px;
  color: rgba(255, 255, 255, 0.45);
  border-bottom: 1px solid rgba(255, 255, 255, 0.15);
  padding: 4px 6px;
}

.task-table td {
  padding: 5px 6px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.07);
  vertical-align: middle;
}

.name-cell {
  max-width: 220px;
}

.state {
  font-size: 11px;
}

.state-running {
  color: #5af78e;
}

.state-pending {
  color: #f3f99d;
}

.state-done {
  color: rgba(255, 255, 255, 0.55);
}

.state-failed {
  color: #ff5f56;
}

.state-killed {
  color: #ff6ac1;
}

.progress-cell {
  display: flex;
  align-items: center;
  gap: 6px;
}

.bar {
  display: inline-block;
  width: 80px;
  height: 8px;
  background: rgba(255, 255, 255, 0.12);
}

.bar-fill {
  display: block;
  height: 100%;
  background: #5af78e;
}

.pct {
  font-size: 11px;
  color: rgba(255, 255, 255, 0.65);
}

.jobs {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.job {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  border: 1px solid rgba(255, 255, 255, 0.12);
  padding: 6px 8px;
}

.job-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.job-name {
  font-size: 12px;
}

.path-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 10px;
}

.path-input {
  flex: 1;
  background: #000;
  border: 1px solid rgba(255, 255, 255, 0.3);
  color: #fff;
  font-family: inherit;
  font-size: 12px;
  padding: 4px 6px;
  outline: none;
}

.path-input:focus {
  border-color: #5af78e;
}

.note {
  margin: 8px 0 0;
  font-size: 11px;
  color: #9aedfe;
  white-space: pre-wrap;
}

.empty {
  margin: 4px 0 0;
  font-size: 12px;
}

.text-muted {
  color: rgba(255, 255, 255, 0.5) !important;
}
</style>
