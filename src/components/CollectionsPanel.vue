<template>
  <div class="collections-panel">
    <!-- Header -->
    <div class="panel-header">
      <div class="header-left">
        <BookMarked :size="22" class="icon-bookmark" />
        <h2 class="panel-title">Collections</h2>
      </div>
      <button class="close-btn" @click="$emit('close')">
        <span class="close-x">✕</span>
      </button>
    </div>

    <!-- New Collection Button -->
    <button class="new-collection-btn" @click="handleCreateCollection">
      <Plus :size="16" />
      New Collection
    </button>

    <!-- Pre-defined Collection Types -->
    <div class="predefined-section">
      <div
        class="predefined-card gold-card"
        @click="handlePredefined('highlights')"
      >
        <div class="predefined-icon">✨</div>
        <div class="predefined-info">
          <span class="predefined-name">Highlights</span>
          <span class="predefined-desc">Quick captures — your brightest moments</span>
        </div>
        <div class="predefined-arrow">→</div>
      </div>

      <div
        class="predefined-card pink-card"
        @click="handlePredefined('best_moments')"
      >
        <div class="predefined-icon">🏆</div>
        <div class="predefined-info">
          <span class="predefined-name">Best Moments</span>
          <span class="predefined-desc">Curated picks — your finest selections</span>
        </div>
        <div class="predefined-arrow">→</div>
      </div>
    </div>

    <!-- Custom Collections -->
    <div class="section" v-if="collections.length > 0">
      <h3 class="section-title">
        <FolderHeart :size="16" />
        Custom Collections ({{ collections.length }})
      </h3>
      <div class="collections-list">
        <div
          v-for="col in collections"
          :key="col.id"
          class="collection-card"
          :style="{ borderLeftColor: col.color }"
          @click="handleSelectCollection(col.id)"
        >
          <div class="collection-top">
            <div
              class="collection-dot"
              :style="{ backgroundColor: col.color }"
            />
            <span class="collection-name">{{ col.name }}</span>
            <span class="collection-type-badge" :style="getTypeBadgeStyle(col.collectionType)">
              {{ col.collectionType }}
            </span>
          </div>
          <p class="collection-desc" v-if="col.description">{{ col.description }}</p>
          <div class="collection-meta">
            <span class="collection-count">
              <Sparkles :size="12" />
              {{ col.itemIds.length }} items
            </span>
          </div>
        </div>
      </div>
    </div>

    <!-- Empty State -->
    <div class="empty-state" v-if="collections.length === 0">
      <BookMarked :size="40" class="empty-icon" />
      <p>No collections yet. Start curating your files.</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useAppStore } from '@/stores/app'
import type { CollectionType } from '@/types'
import {
  BookMarked,
  Plus,
  Sparkles,
  Trophy,
  FolderHeart,
} from 'lucide-vue-next'

const store = useAppStore()

const emit = defineEmits<{
  close: []
}>()

const collections = computed(() => store.collections)

function getTypeBadgeStyle(type: CollectionType): Record<string, string> {
  const styles: Record<CollectionType, Record<string, string>> = {
    highlights: { color: '#FFB800', backgroundColor: 'rgba(255, 184, 0, 0.15)', borderColor: '#FFB800' },
    best_moments: { color: '#FF2D6F', backgroundColor: 'rgba(255, 45, 111, 0.15)', borderColor: '#FF2D6F' },
    custom: { color: '#00D4FF', backgroundColor: 'rgba(0, 212, 255, 0.15)', borderColor: '#00D4FF' },
  }
  return styles[type] || styles.custom
}

async function handleCreateCollection() {
  const name = 'New Collection'
  await store.createCollection(name, 'custom', '#00D4FF', 'A custom collection')
}

async function handlePredefined(type: string) {
  const predefinedNames: Record<string, string> = {
    highlights: '✨ Highlights',
    best_moments: '🏆 Best Moments',
  }
  const predefinedColors: Record<string, string> = {
    highlights: '#FFB800',
    best_moments: '#FF2D6F',
  }
  const name = predefinedNames[type] || type
  const color = predefinedColors[type] || '#00D4FF'
  await store.createCollection(name, type as CollectionType, color)
}

function handleSelectCollection(collectionId: string) {
  // Conceptual: could open collection view or filter files
  console.log('Selected collection:', collectionId)
}
</script>

<style scoped>
.collections-panel {
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

.collections-panel::-webkit-scrollbar {
  width: 6px;
}
.collections-panel::-webkit-scrollbar-track {
  background: #0a0a0f;
}
.collections-panel::-webkit-scrollbar-thumb {
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

.icon-bookmark {
  color: #FF2D6F;
  filter: drop-shadow(0 0 6px #FF2D6F);
}

.panel-title {
  font-size: 18px;
  font-weight: 800;
  text-transform: uppercase;
  letter-spacing: 2px;
  color: #FF2D6F;
  text-shadow: 0 0 10px #FF2D6F, 0 0 20px rgba(255, 45, 111, 0.3);
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

/* New Collection Button */
.new-collection-btn {
  width: 100%;
  background: #FFB800;
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

.new-collection-btn:hover {
  transform: translate(-1px, -1px);
  box-shadow: 4px 4px 0 0 #000;
  filter: brightness(1.1);
}

.new-collection-btn:active {
  transform: translate(1px, 1px);
  box-shadow: 2px 2px 0 0 #000;
}

/* Predefined */
.predefined-section {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.predefined-card {
  background: #1a1a2e;
  border: 3px solid #000;
  box-shadow: 3px 3px 0 0 #000;
  padding: 14px 16px;
  display: flex;
  align-items: center;
  gap: 12px;
  cursor: pointer;
  transition: all 0.15s;
}

.predefined-card:hover {
  transform: translate(-1px, -1px);
  box-shadow: 4px 4px 0 0 #000;
}

.predefined-card:active {
  transform: translate(1px, 1px);
  box-shadow: 2px 2px 0 0 #000;
}

.gold-card {
  border-left: 5px solid #FFB800;
}

.pink-card {
  border-left: 5px solid #FF2D6F;
}

.predefined-icon {
  font-size: 24px;
  line-height: 1;
}

.predefined-info {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.predefined-name {
  font-size: 14px;
  font-weight: 700;
}

.predefined-desc {
  font-size: 11px;
  color: #6B7280;
}

.predefined-arrow {
  font-size: 16px;
  color: #4B5563;
  font-weight: 700;
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

/* Collections List */
.collections-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.collection-card {
  background: #1a1a2e;
  border: 3px solid #000;
  border-left: 5px solid;
  box-shadow: 3px 3px 0 0 #000;
  padding: 12px 14px;
  cursor: pointer;
  display: flex;
  flex-direction: column;
  gap: 6px;
  transition: all 0.15s;
}

.collection-card:hover {
  transform: translate(-1px, -1px);
  box-shadow: 4px 4px 0 0 #000;
}

.collection-card:active {
  transform: translate(1px, 1px);
  box-shadow: 2px 2px 0 0 #000;
}

.collection-top {
  display: flex;
  align-items: center;
  gap: 8px;
}

.collection-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}

.collection-name {
  font-size: 13px;
  font-weight: 700;
  flex: 1;
}

.collection-type-badge {
  font-size: 9px;
  font-weight: 800;
  text-transform: uppercase;
  letter-spacing: 1px;
  padding: 2px 6px;
  border: 1px solid;
  border-radius: 2px;
}

.collection-desc {
  font-size: 11px;
  color: #6B7280;
  margin: 0;
  line-height: 1.4;
}

.collection-meta {
  display: flex;
  align-items: center;
  gap: 4px;
}

.collection-count {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;
  color: #9CA3AF;
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
</style>
