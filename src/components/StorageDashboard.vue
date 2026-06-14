<template>
  <div class="storage-panel">
    <div class="panel-header">
      <div class="header-left">
        <span class="icon-storage">[$]</span>
        <h2 class="panel-title">STORAGE DASHBOARD</h2>
      </div>
    </div>

    <div class="section">
      <h3 class="section-title">[SUMMARY] FILE COUNTS</h3>
      <div class="stats-grid">
        <div class="stat-card">
          <span class="stat-value">{{ store.files.length }}</span>
          <span class="stat-label text-muted">TOTAL FILES</span>
        </div>
        <div class="stat-card">
          <span class="stat-value">{{ store.folders.length }}</span>
          <span class="stat-label text-muted">FOLDERS</span>
        </div>
        <div class="stat-card">
          <span class="stat-value">{{ store.encryptedFiles.length }}</span>
          <span class="stat-label text-muted">ENCRYPTED</span>
        </div>
        <div class="stat-card">
          <span class="stat-value">{{ store.compressedFiles.length }}</span>
          <span class="stat-label text-muted">COMPRESSED</span>
        </div>
        <div class="stat-card">
          <span class="stat-value">{{ store.starredFiles.length }}</span>
          <span class="stat-label text-muted">STARRED</span>
        </div>
        <div class="stat-card">
          <span class="stat-value">{{ trashCount }}</span>
          <span class="stat-label text-muted">IN TRASH</span>
        </div>
      </div>
    </div>

    <div class="section">
      <h3 class="section-title">[SIZE] TOTAL BY TYPE</h3>
      <div class="type-breakdown">
        <div v-for="entry in byType" :key="entry.label" class="type-row">
          <span class="type-label">{{ entry.label }}</span>
          <span class="type-bar"><span class="type-bar-fill" :style="{ width: entry.percent + '%' }" /></span>
          <span class="type-size">{{ formatSize(entry.totalBytes) }}</span>
        </div>
      </div>
    </div>

    <div class="section">
      <h3 class="section-title">[DATA] STORAGE FOOTPRINT</h3>
      <div class="info-card">
        <div class="info-row"><span class="info-key text-muted">TOTAL SIZE</span><span class="info-value">{{ totalSizeFormatted }}</span></div>
        <div class="info-row"><span class="info-key text-muted">LARGEST FILE</span><span class="info-value">{{ largestFile }}</span></div>
        <div class="info-row"><span class="info-key text-muted">AVG FILE SIZE</span><span class="info-value">{{ avgSizeFormatted }}</span></div>
        <div class="info-row"><span class="info-key text-muted">FILES WITH GPS</span><span class="info-value">{{ gpsCount }}</span></div>
        <div class="info-row"><span class="info-key text-muted">FILES WITH FACES</span><span class="info-value">{{ faceCount }}</span></div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useAppStore } from '@/stores/app'

const store = useAppStore()

const trashCount = computed(() => store.trashItems.length)

const totalSize = computed(() => store.files.reduce((s, f) => s + f.sizeBytes, 0))

const totalSizeFormatted = computed(() => formatSize(totalSize.value))

const largestFile = computed(() => {
  if (store.files.length === 0) return '--'
  const biggest = [...store.files].sort((a, b) => b.sizeBytes - a.sizeBytes)[0]
  return `${biggest.name} (${formatSize(biggest.sizeBytes)})`
})

const avgSizeFormatted = computed(() => {
  if (store.files.length === 0) return '--'
  return formatSize(Math.round(totalSize.value / store.files.length))
})

const gpsCount = computed(() => store.files.filter(f => f.gpsLat).length)
const faceCount = computed(() => store.files.filter(f => f.faceGroupIds && f.faceGroupIds.length > 0).length)

const byType = computed(() => {
  const groups: Record<string, { totalBytes: number; count: number }> = {}
  for (const f of store.files) {
    const type = f.mimeType?.split('/')[0] || f.fileType || 'unknown'
    if (!groups[type]) groups[type] = { totalBytes: 0, count: 0 }
    groups[type].totalBytes += f.sizeBytes
    groups[type].count++
  }
  const total = totalSize.value
  const entries = Object.entries(groups).map(([label, data]) => ({
    label: label.toUpperCase(),
    totalBytes: data.totalBytes,
    count: data.count,
    percent: total > 0 ? (data.totalBytes / total) * 100 : 0,
  }))
  entries.sort((a, b) => b.totalBytes - a.totalBytes)
  return entries
})

function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const k = 1024
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + units[i]
}
</script>

<style scoped>
.storage-panel {
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
.icon-storage { font-size: 16px; }
.panel-title { font-size: 14px; font-weight: 800; letter-spacing: 1px; margin: 0; }

.section { margin-bottom: 16px; }

.section-title {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 1px;
  color: rgba(255,255,255,0.6);
  margin: 0 0 8px;
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 6px;
}

.stat-card {
  border: 2px solid #FFFFFF;
  padding: 10px;
  text-align: center;
}

.stat-value {
  display: block;
  font-size: 18px;
  font-weight: 800;
  margin-bottom: 4px;
}

.stat-label {
  font-size: 8px;
  letter-spacing: 0.5px;
}

.type-breakdown {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.type-row {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 9px;
}

.type-label {
  width: 80px;
  flex-shrink: 0;
  font-weight: 700;
}

.type-bar {
  flex: 1;
  height: 10px;
  border: 1px solid #FFFFFF;
  position: relative;
  background: transparent;
}

.type-bar-fill {
  display: block;
  height: 100%;
  background: #FFFFFF;
}

.type-size {
  width: 70px;
  text-align: right;
  flex-shrink: 0;
  color: rgba(255,255,255,0.6);
}

.info-card {
  border: 2px solid #FFFFFF;
  padding: 10px;
}

.info-row {
  display: flex;
  justify-content: space-between;
  font-size: 10px;
  padding: 3px 0;
  border-bottom: 1px solid rgba(255,255,255,0.1);
}

.info-row:last-child {
  border-bottom: none;
}

.info-key { color: rgba(255,255,255,0.5); }
.info-value { font-weight: 700; }

.text-muted { color: rgba(255,255,255,0.5) !important; }
</style>
