<template>
  <div class="map-view">
    <!-- Header -->
    <div class="panel-header">
      <div class="header-left">
        <MapPin :size="22" class="icon-map" />
        <h2 class="panel-title">Geography View</h2>
      </div>
      <div class="header-actions">
        <button class="refresh-btn" @click="handleRefresh" title="Refresh geo data">
          <Satellite :size="16" />
        </button>
      </div>
    </div>

    <!-- Map Area -->
    <div class="map-container" v-if="geoMarkers.length > 0">
      <div class="map-placeholder">
        <!-- Grid lines -->
        <div class="map-grid">
          <div v-for="n in 8" :key="'h' + n" class="grid-line horizontal" :style="{ top: (n * 12.5) + '%' }" />
          <div v-for="n in 8" :key="'v' + n" class="grid-line vertical" :style="{ left: (n * 12.5) + '%' }" />
        </div>

        <!-- Coordinate labels -->
        <div class="coord-labels">
          <span class="coord-lbl top-lbl">85°N</span>
          <span class="coord-lbl bottom-lbl">85°S</span>
          <span class="coord-lbl left-lbl">180°W</span>
          <span class="coord-lbl right-lbl">180°E</span>
        </div>

        <!-- Markers -->
        <div
          v-for="(marker, idx) in geoMarkers"
          :key="marker.fileId"
          class="geo-marker"
          :style="getMarkerPosition(marker)"
          @mouseenter="hoveredMarker = idx"
          @mouseleave="hoveredMarker = -1"
        >
          <div class="marker-dot" :style="{ '--marker-color': getMarkerColor(idx) }">
            <MapPin :size="12" />
          </div>
          <!-- Tooltip -->
          <div class="marker-tooltip" v-if="hoveredMarker === idx">
            <span class="tooltip-name">{{ marker.fileName }}</span>
            <span class="tooltip-coords">
              {{ marker.lat.toFixed(4) }}, {{ marker.lng.toFixed(4) }}
            </span>
            <span class="tooltip-address" v-if="marker.address">{{ marker.address }}</span>
          </div>
        </div>

        <!-- Map stats overlay -->
        <div class="map-stats-overlay">
          <Globe :size="12" />
          <span>{{ geoMarkers.length }} locations</span>
        </div>
      </div>
    </div>

    <!-- Empty State -->
    <div class="empty-state" v-if="geoMarkers.length === 0">
      <div class="empty-map-visual">
        <Navigation :size="48" class="empty-icon" />
      </div>
      <p>No geotagged files found. Photos with GPS EXIF data will appear here.</p>
    </div>

    <!-- Geo File List -->
    <div class="section" v-if="geoMarkers.length > 0">
      <h3 class="section-title">
        <Navigation :size="16" />
        Geotagged Files ({{ geoMarkers.length }})
      </h3>
      <div class="geo-list">
        <div
          v-for="marker in geoMarkers"
          :key="'list-' + marker.fileId"
          class="geo-list-item"
          @mouseenter="hoveredMarker = geoMarkers.indexOf(marker)"
          @mouseleave="hoveredMarker = -1"
        >
          <MapPin :size="14" class="geo-list-pin" />
          <div class="geo-list-info">
            <span class="geo-list-name">{{ marker.fileName }}</span>
            <span class="geo-list-address" v-if="marker.address">{{ marker.address }}</span>
          </div>
          <span class="geo-list-coords mono">
            {{ marker.lat.toFixed(3) }}, {{ marker.lng.toFixed(3) }}
          </span>
        </div>
      </div>
    </div>

    <!-- Status Footer -->
    <div class="status-footer">
      <span>📍 GPS extraction via kamadak-exif (pure Rust) • MapLibre GL JS rendering</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useAppStore } from '@/stores/app'
import {
  MapPin,
  Globe,
  Navigation,
  Satellite,
} from 'lucide-vue-next'

const store = useAppStore()

const emit = defineEmits<{
  close: []
}>()

const geoMarkers = computed(() => store.geoMarkers)
const hoveredMarker = ref(-1)

onMounted(() => {
  store.fetchGeoFiles()
})

async function handleRefresh() {
  await store.fetchGeoFiles()
}

function getMarkerPosition(marker: { lat: number; lng: number }): Record<string, string> {
  // Mercator-like projection for placeholder map
  const padding = 40 // px from edges
  const top = ((90 - marker.lat) / 180) * (100 - 8) + 4
  const left = ((marker.lng + 180) / 360) * (100 - 8) + 4
  return {
    top: top + '%',
    left: left + '%',
  }
}

const markerColors = [
  '#00FF41', '#00D4FF', '#FF2D6F', '#FFB800',
  '#A855F7', '#FF6B2B', '#FACC15', '#1E3A8A',
  '#DC2626', '#16A34A',
]

function getMarkerColor(idx: number): string {
  return markerColors[idx % markerColors.length]
}
</script>

<style scoped>
.map-view {
  width: 100%;
  height: 100%;
  background: var(--cyber-bg-panel, #12121a);
  border: 3px solid #000;
  box-shadow: 4px 4px 0 0 #000;
  overflow-y: auto;
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  font-family: 'Inter', system-ui, sans-serif;
  color: #F5F5F4;
}

.map-view::-webkit-scrollbar {
  width: 6px;
}
.map-view::-webkit-scrollbar-track {
  background: #0a0a0f;
}
.map-view::-webkit-scrollbar-thumb {
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

.icon-map {
  color: #00D4FF;
  filter: drop-shadow(0 0 6px #00D4FF);
}

.panel-title {
  font-size: 18px;
  font-weight: 800;
  text-transform: uppercase;
  letter-spacing: 2px;
  color: #00D4FF;
  text-shadow: 0 0 10px #00D4FF, 0 0 20px rgba(0, 212, 255, 0.3);
  margin: 0;
}

.header-actions {
  display: flex;
  gap: 8px;
}

.refresh-btn {
  background: #1a1a2e;
  border: 2px solid #333;
  color: #9CA3AF;
  cursor: pointer;
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
}

.refresh-btn:hover {
  border-color: #00D4FF;
  color: #00D4FF;
}

/* Map Container */
.map-container {
  width: 100%;
}

.map-placeholder {
  position: relative;
  width: 100%;
  min-height: 320px;
  background:
    radial-gradient(ellipse at 50% 40%, rgba(0, 212, 255, 0.06) 0%, transparent 70%),
    linear-gradient(180deg, #080810 0%, #0c0c18 40%, #0a0f14 100%);
  border: 3px solid #000;
  box-shadow: 4px 4px 0 0 #000;
  overflow: hidden;
}

/* Grid Lines */
.map-grid {
  position: absolute;
  inset: 0;
}

.grid-line {
  position: absolute;
  background: rgba(0, 212, 255, 0.06);
}

.grid-line.horizontal {
  left: 0;
  right: 0;
  height: 1px;
}

.grid-line.vertical {
  top: 0;
  bottom: 0;
  width: 1px;
}

/* Coordinate Labels */
.coord-labels {
  position: absolute;
  inset: 0;
  pointer-events: none;
}

.coord-lbl {
  position: absolute;
  font-size: 9px;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  color: rgba(0, 212, 255, 0.25);
  letter-spacing: 1px;
}

.top-lbl {
  top: 6px;
  left: 50%;
  transform: translateX(-50%);
}

.bottom-lbl {
  bottom: 6px;
  left: 50%;
  transform: translateX(-50%);
}

.left-lbl {
  left: 6px;
  top: 50%;
  transform: translateY(-50%) rotate(-90deg);
}

.right-lbl {
  right: 6px;
  top: 50%;
  transform: translateY(-50%) rotate(90deg);
}

/* Geo Markers */
.geo-marker {
  position: absolute;
  transform: translate(-50%, -50%);
  z-index: 10;
  cursor: pointer;
}

.marker-dot {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  background: var(--marker-color);
  border: 2px solid #000;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #0a0a0f;
  box-shadow: 0 0 10px var(--marker-color), 0 0 20px var(--marker-color);
  transition: all 0.2s;
  animation: marker-pulse 2s ease-in-out infinite;
}

.geo-marker:hover .marker-dot {
  transform: scale(1.3);
}

@keyframes marker-pulse {
  0%, 100% { box-shadow: 0 0 8px var(--marker-color), 0 0 16px var(--marker-color); }
  50% { box-shadow: 0 0 14px var(--marker-color), 0 0 28px var(--marker-color); }
}

/* Tooltip */
.marker-tooltip {
  position: absolute;
  bottom: calc(100% + 8px);
  left: 50%;
  transform: translateX(-50%);
  background: #1a1a2e;
  border: 2px solid #000;
  box-shadow: 3px 3px 0 0 #000;
  padding: 8px 12px;
  white-space: nowrap;
  z-index: 20;
  display: flex;
  flex-direction: column;
  gap: 3px;
  pointer-events: none;
}

.marker-tooltip::after {
  content: '';
  position: absolute;
  top: 100%;
  left: 50%;
  transform: translateX(-50%);
  border: 6px solid transparent;
  border-top-color: #1a1a2e;
}

.tooltip-name {
  font-size: 12px;
  font-weight: 700;
  color: #F5F5F4;
}

.tooltip-coords {
  font-size: 10px;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  color: #00D4FF;
}

.tooltip-address {
  font-size: 10px;
  color: #6B7280;
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* Map Stats Overlay */
.map-stats-overlay {
  position: absolute;
  bottom: 8px;
  right: 8px;
  background: rgba(10, 10, 15, 0.85);
  border: 1px solid rgba(0, 212, 255, 0.2);
  padding: 4px 8px;
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 10px;
  color: rgba(0, 212, 255, 0.6);
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  z-index: 5;
}

/* Sections */
.section {
  display: flex;
  flex-direction: column;
  gap: 10px;
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

/* Geo List */
.geo-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.geo-list-item {
  background: #1a1a2e;
  border: 3px solid #000;
  box-shadow: 3px 3px 0 0 #000;
  padding: 8px 12px;
  display: flex;
  align-items: center;
  gap: 10px;
  cursor: pointer;
  transition: all 0.15s;
}

.geo-list-item:hover {
  border-color: #00D4FF;
}

.geo-list-pin {
  color: #00D4FF;
  flex-shrink: 0;
}

.geo-list-info {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.geo-list-name {
  font-size: 12px;
  font-weight: 700;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.geo-list-address {
  font-size: 10px;
  color: #6B7280;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.geo-list-coords {
  font-size: 10px;
  color: #4B5563;
  flex-shrink: 0;
}

/* Empty State */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
  padding: 60px 20px;
  text-align: center;
}

.empty-map-visual {
  width: 80px;
  height: 80px;
  border-radius: 50%;
  border: 3px solid #1a1a2e;
  background: rgba(0, 212, 255, 0.03);
  display: flex;
  align-items: center;
  justify-content: center;
}

.empty-icon {
  color: #1a1a2e;
}

.empty-state p {
  font-size: 13px;
  color: #6B7280;
  margin: 0;
  line-height: 1.6;
  max-width: 300px;
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

/* Utility */
.mono {
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
}
</style>
