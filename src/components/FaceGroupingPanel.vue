<template>
  <div class="face-panel">
    <div class="panel-header">
      <div class="header-left">
        <span class="icon-face">[+]</span>
        <h2 class="panel-title">FACE GROUPING</h2>
      </div>
    </div>

    <div class="section">
      <h3 class="section-title">[SCAN] DETECT FACES</h3>
      <button class="bw-btn" style="width:100%;" @click="handleBatchDetect">[BATCH DETECT]</button>
    </div>

    <div class="section" v-if="lastResult">
      <h3 class="section-title">[STATS] LAST SCAN</h3>
      <div class="stats-card">
        <div class="stat-row"><span class="stat-key text-muted">CLUSTERS</span><span class="stat-value">{{ lastResult.clustersCreated }}</span></div>
        <div class="stat-row"><span class="stat-key text-muted">FACES</span><span class="stat-value">{{ lastResult.totalFaces }}</span></div>
        <div class="stat-row"><span class="stat-key text-muted">COHESION</span><span class="stat-value">{{ lastResult.avgCohesion.toFixed(3) }}</span></div>
        <div class="stat-row"><span class="stat-key text-muted">STRATEGY</span><span class="stat-value">{{ lastResult.strategyUsed }}</span></div>
      </div>
    </div>

    <div class="section">
      <h3 class="section-title">[GROUP] PEOPLE ({{ faceGroups.length }})</h3>
      <div class="group-list">
        <div v-for="group in faceGroups" :key="group.id" class="group-card">
          <div class="group-header">
            <div class="avatar">++</div>
            <div class="group-info">
              <span class="group-name">{{ group.name }}</span>
              <span class="group-meta text-muted">{{ group.fileIds.length }} FILES</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useAppStore } from '@/stores/app'

const store = useAppStore()
const faceGroups = computed(() => store.faceGroups)
const lastResult = ref<{ clustersCreated: number; totalFaces: number; noiseFaces: number; avgCohesion: number; strategyUsed: string } | null>(null)

async function handleBatchDetect() {
  const result = await store.detectFacesBatch()
  if (result) lastResult.value = result
}
</script>

<style scoped>
.face-panel {
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
.icon-face { font-size: 16px; }
.panel-title { font-size: 14px; font-weight: 800; letter-spacing: 1px; margin: 0; }

.section { margin-bottom: 16px; }

.section-title {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 1px;
  color: rgba(255,255,255,0.6);
  margin: 0 0 8px;
}

.bw-btn {
  padding: 6px 12px;
  background: #FFFFFF;
  color: #000;
  border: 2px solid #FFFFFF;
  font-family: 'Courier New', monospace;
  font-size: 11px;
  font-weight: 700;
  cursor: pointer;
}

.bw-btn:hover { background: #000; color: #FFFFFF; }

.stats-card {
  border: 2px solid #FFFFFF;
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.stat-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.stat-key { font-size: 10px; }
.stat-value { font-size: 11px; font-weight: 700; font-family: 'Courier New', monospace; }

.group-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.group-card {
  border: 2px solid #FFFFFF;
  padding: 10px;
}

.group-header {
  display: flex;
  align-items: center;
  gap: 10px;
}

.avatar {
  width: 28px;
  height: 28px;
  border: 2px solid #FFFFFF;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  font-size: 10px;
}

.group-info { flex: 1; min-width: 0; }
.group-name { font-size: 12px; font-weight: 700; }
.group-meta { font-size: 10px; }

.text-muted { color: rgba(255,255,255,0.5) !important; }
</style>
