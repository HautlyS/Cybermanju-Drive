<template>
  <div class="sync-panel">
    <div class="panel-header">
      <div class="header-left">
        <span class="icon-sync">[~]</span>
        <h2 class="panel-title">STORAGE SYNC</h2>
      </div>
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
          </div>
        </div>
      </div>
    </div>

    <div class="section" v-if="syncProgress">
      <h3 class="section-title">[PROG] SYNC PROGRESS</h3>
      <div class="progress-card">
        <div class="p-row"><span class="p-key text-muted">STATUS</span><span class="p-value">{{ syncProgress.status }}</span></div>
        <div class="p-row"><span class="p-key text-muted">FILES</span><span class="p-value">{{ syncProgress.processedFiles }}/{{ syncProgress.totalFiles }}</span></div>
        <div class="p-row"><span class="p-key text-muted">BYTES</span><span class="p-value">{{ formatSize(syncProgress.bytesUploaded) }}</span></div>
        <div class="p-row" v-if="syncProgress.errors.length"><span class="p-key text-muted">ERRORS</span><span class="p-value">{{ syncProgress.errors.length }}</span></div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useAppStore } from '@/stores/app'

const store = useAppStore()
const syncConfigs = computed(() => store.syncConfigs)
const syncProgress = computed(() => store.syncProgress)

function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const k = 1024
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + units[i]
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

.config-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.config-card {
  border: 2px solid #FFFFFF;
  padding: 8px 10px;
}

.cfg-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
}

.cfg-name { font-size: 12px; font-weight: 700; flex: 1; }
.cfg-type { font-size: 9px; }
.cfg-status { font-size: 9px; font-weight: 700; border: 1px solid #FFFFFF; padding: 0 4px; }
.cfg-status.on { background: #FFFFFF; color: #000; }
.cfg-meta { font-size: 9px; }

.progress-card {
  border: 2px solid #FFFFFF;
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.p-row { display: flex; justify-content: space-between; }
.p-key { font-size: 10px; }
.p-value { font-size: 10px; font-weight: 700; }

.text-muted { color: rgba(255,255,255,0.5) !important; }
</style>
