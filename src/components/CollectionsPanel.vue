<template>
  <div class="collections-panel">
    <div class="panel-header">
      <div class="header-left">
        <span class="icon-col">[*]</span>
        <h2 class="panel-title">COLLECTIONS</h2>
      </div>
    </div>

    <div class="section" v-if="collections.length === 0">
      <p class="text-muted">NO COLLECTIONS YET. CREATE ONE TO GROUP FILES.</p>
    </div>

    <div class="collections-list">
      <div v-for="col in collections" :key="col.id" class="collection-card">
        <div class="col-header">
          <span class="col-name">{{ col.name }}</span>
          <span class="col-type text-muted">{{ col.collectionType }}</span>
        </div>
        <div class="col-meta text-muted">
          {{ col.itemIds.length }} ITEMS
        </div>
      </div>
    </div>

    <div class="section create-section">
      <h3 class="section-title">[+] CREATE COLLECTION</h3>
      <input v-model="newName" class="bw-input" placeholder="COLLECTION NAME" @keyup.enter="handleCreate" />
      <select v-model="newType" class="bw-input" style="appearance:none;">
        <option value="custom">CUSTOM</option>
        <option value="highlights">HIGHLIGHTS</option>
        <option value="best_moments">BEST MOMENTS</option>
      </select>
      <button class="bw-btn" style="width:100%;" @click="handleCreate">[CREATE]</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useAppStore } from '@/stores/app'
import type { CollectionType } from '@/types'

const store = useAppStore()
const collections = computed(() => store.collections)
const newName = ref('')
const newType = ref<CollectionType>('custom')

async function handleCreate() {
  if (!newName.value.trim()) return
  await store.createCollection(newName.value.trim(), newType.value, '#FFFFFF')
  newName.value = ''
}
</script>

<style scoped>
.collections-panel {
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

.header-left {
  display: flex;
  align-items: center;
  gap: 8px;
}

.icon-col { font-size: 16px; }

.panel-title {
  font-size: 14px;
  font-weight: 800;
  letter-spacing: 1px;
  margin: 0;
}

.section { margin-bottom: 16px; }

.collections-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 16px;
}

.collection-card {
  border: 2px solid #FFFFFF;
  padding: 10px;
}

.col-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 4px;
}

.col-name { font-size: 13px; font-weight: 700; }
.col-type { font-size: 9px; }
.col-meta { font-size: 10px; }

.section-title {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 1px;
  color: rgba(255,255,255,0.6);
  margin: 0 0 8px;
}

.create-section {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.bw-input {
  background: #000;
  border: 2px solid #FFFFFF;
  padding: 6px 8px;
  color: #FFFFFF;
  font-family: 'Courier New', monospace;
  font-size: 11px;
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

.text-muted { color: rgba(255,255,255,0.5) !important; }
</style>
