<template>
  <div class="face-panel">
    <!-- Header -->
    <div class="panel-header">
      <div class="header-left">
        <Users :size="22" class="icon-users" />
        <h2 class="panel-title">People</h2>
      </div>
      <button class="close-btn" @click="$emit('close')">
        <span class="close-x">✕</span>
      </button>
    </div>

    <!-- Scan Button -->
    <button class="scan-btn" @click="handleScan">
      <Scan :size="16" />
      Scan for Faces
    </button>

    <!-- Face Groups Grid -->
    <div class="section" v-if="faceGroups.length > 0">
      <h3 class="section-title">
        <UserCircle :size="16" />
        Detected People ({{ faceGroups.length }})
      </h3>
      <div class="face-grid">
        <div
          v-for="group in faceGroups"
          :key="group.id"
          class="face-card"
          @click="handleSelectGroup(group.id)"
        >
          <div
            class="face-avatar"
            :style="{
              backgroundColor: group.color,
              boxShadow: `0 0 12px ${group.color}44`,
            }"
          >
            <UserCircle :size="36" class="avatar-icon" />
          </div>
          <span class="face-name">{{ group.name }}</span>
          <span class="face-count">
            <Camera :size="10" />
            {{ group.fileIds.length }} faces
          </span>
        </div>
      </div>

      <!-- Detailed List -->
      <div class="face-list">
        <div
          v-for="group in faceGroups"
          :key="'list-' + group.id"
          class="face-list-item"
          :style="{ borderLeftColor: group.color }"
          @click="handleSelectGroup(group.id)"
        >
          <div
            class="face-list-avatar"
            :style="{ backgroundColor: group.color }"
          >
            <UserCircle :size="20" class="avatar-icon-sm" />
          </div>
          <div class="face-list-info">
            <span class="face-list-name">{{ group.name }}</span>
            <span class="face-list-files">{{ group.fileIds.length }} files</span>
          </div>
          <div class="face-list-badge">
            {{ group.fileIds.length }}
          </div>
        </div>
      </div>
    </div>

    <!-- Empty State -->
    <div class="empty-state" v-if="faceGroups.length === 0">
      <Brain :size="40" class="empty-icon" />
      <p>No face groups detected yet. Scan photos to group by person.</p>
    </div>

    <!-- Status Footer -->
    <div class="status-footer">
      <span>🧠 Face detection powered by ONNX Runtime (ort) + ArcFace embeddings + DBSCAN clustering</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useAppStore } from '@/stores/app'
import {
  Users,
  Scan,
  UserCircle,
  Camera,
  Brain,
} from 'lucide-vue-next'

const store = useAppStore()

const emit = defineEmits<{
  close: []
}>()

const faceGroups = computed(() => store.faceGroups)

async function handleScan() {
  await store.fetchFaceGroups()
}

function handleSelectGroup(groupId: string) {
  // Conceptual: could open a filtered view of files for this person
  console.log('Selected face group:', groupId)
}
</script>

<style scoped>
.face-panel {
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

.face-panel::-webkit-scrollbar {
  width: 6px;
}
.face-panel::-webkit-scrollbar-track {
  background: #0a0a0f;
}
.face-panel::-webkit-scrollbar-thumb {
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

.icon-users {
  color: #A855F7;
  filter: drop-shadow(0 0 6px #A855F7);
}

.panel-title {
  font-size: 18px;
  font-weight: 800;
  text-transform: uppercase;
  letter-spacing: 2px;
  color: #A855F7;
  text-shadow: 0 0 10px #A855F7, 0 0 20px rgba(168, 85, 247, 0.3);
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

/* Scan Button */
.scan-btn {
  width: 100%;
  background: #A855F7;
  color: #0a0a0f;
  border: 3px solid #000;
  box-shadow: 3px 3px 0 0 #000;
  padding: 10px 16px;
  font-size: 12px;
  font-weight: 800;
  text-transform: uppercase;
  letter-spacing: 1px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  transition: all 0.15s;
}

.scan-btn:hover {
  transform: translate(-1px, -1px);
  box-shadow: 4px 4px 0 0 #000;
  filter: brightness(1.1);
}

.scan-btn:active {
  transform: translate(1px, 1px);
  box-shadow: 2px 2px 0 0 #000;
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

/* Face Grid */
.face-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 12px;
}

.face-card {
  background: #1a1a2e;
  border: 3px solid #000;
  box-shadow: 3px 3px 0 0 #000;
  padding: 14px 8px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  transition: all 0.15s;
}

.face-card:hover {
  transform: translate(-1px, -1px);
  box-shadow: 4px 4px 0 0 #000;
}

.face-card:active {
  transform: translate(1px, 1px);
  box-shadow: 2px 2px 0 0 #000;
}

.face-avatar {
  width: 56px;
  height: 56px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 3px solid #000;
}

.avatar-icon {
  color: rgba(0, 0, 0, 0.4);
}

.face-name {
  font-size: 11px;
  font-weight: 700;
  text-align: center;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  width: 100%;
  color: #F5F5F4;
}

.face-count {
  display: flex;
  align-items: center;
  gap: 3px;
  font-size: 10px;
  color: #6B7280;
}

/* Face List */
.face-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.face-list-item {
  background: #1a1a2e;
  border: 3px solid #000;
  border-left: 5px solid;
  box-shadow: 3px 3px 0 0 #000;
  padding: 10px 12px;
  display: flex;
  align-items: center;
  gap: 10px;
  cursor: pointer;
  transition: all 0.15s;
}

.face-list-item:hover {
  transform: translate(-1px, -1px);
  box-shadow: 4px 4px 0 0 #000;
}

.face-list-item:active {
  transform: translate(1px, 1px);
  box-shadow: 2px 2px 0 0 #000;
}

.face-list-avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 2px solid #000;
  flex-shrink: 0;
}

.avatar-icon-sm {
  color: rgba(0, 0, 0, 0.4);
}

.face-list-info {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.face-list-name {
  font-size: 13px;
  font-weight: 700;
}

.face-list-files {
  font-size: 11px;
  color: #6B7280;
}

.face-list-badge {
  background: #0a0a0f;
  border: 2px solid #333;
  color: #9CA3AF;
  font-size: 11px;
  font-weight: 700;
  padding: 2px 8px;
  border-radius: 2px;
}

/* Empty State */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  padding: 40px 20px;
  text-align: center;
}

.empty-icon {
  color: #333;
}

.empty-state p {
  font-size: 13px;
  color: #6B7280;
  margin: 0;
  line-height: 1.6;
}

/* Footer */
.status-footer {
  margin-top: auto;
  padding-top: 12px;
  border-top: 2px solid #1a1a2e;
  font-size: 10px;
  color: #4B5563;
  text-align: center;
  letter-spacing: 0.5px;
}
</style>
