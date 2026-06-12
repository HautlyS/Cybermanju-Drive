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
      <div ref="mapContainer" class="maplibre-map"></div>
      <!-- Map stats overlay -->
      <div class="map-stats-overlay">
        <Globe :size="12" />
        <span>{{ geoMarkers.length }} locations</span>
      </div>
    </div>

    <!-- Empty State -->
    <div class="empty-state" v-if="geoMarkers.length === 0 && !isLoading">
      <div class="empty-map-visual">
        <Navigation :size="48" class="empty-icon" />
      </div>
      <p>No geotagged files found. Photos with GPS EXIF data will appear here.</p>
    </div>

    <!-- Loading State -->
    <div class="empty-state" v-if="isLoading">
      <div class="loading-spinner"></div>
      <p>Loading geo data...</p>
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
          @click="flyToMarker(marker)"
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
      <span>GPS extraction via kamadak-exif (pure Rust) • MapLibre GL JS rendering</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { useAppStore } from '@/stores/app'
import type { GeoMarker } from '@/types'
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
const isLoading = computed(() => store.isLoading)
const mapContainer = ref<HTMLDivElement | null>(null)

let map: any = null
let markers: any[] = []

const MARKER_COLORS = [
  '#00FF41', '#00D4FF', '#FF2D6F', '#FFB800',
  '#A855F7', '#FF6B2B', '#FACC15', '#1E3A8A',
  '#DC2626', '#16A34A',
]

onMounted(async () => {
  await store.fetchGeoFiles()
  await nextTick()
  if (geoMarkers.value.length > 0) {
    initMap()
  }
})

onUnmounted(() => {
  destroyMap()
})

watch(geoMarkers, async (newMarkers) => {
  if (newMarkers.length > 0 && !map) {
    await nextTick()
    initMap()
  } else if (map) {
    updateMarkers()
  }
})

async function initMap() {
  if (!mapContainer.value || map) return

  try {
    const maplibregl = await import('maplibre-gl')
    await import('maplibre-gl/dist/maplibre-gl.css')

    const center = getMapCenter()

    map = new maplibregl.Map({
      container: mapContainer.value,
      style: {
        version: 8,
        sources: {
          osm: {
            type: 'raster',
            tiles: ['https://tile.openstreetmap.org/{z}/{x}/{y}.png'],
            tileSize: 256,
            attribution: '&copy; OpenStreetMap contributors',
          },
        },
        layers: [
          {
            id: 'osm',
            type: 'raster',
            source: 'osm',
          },
        ],
      },
      center: [center.lng, center.lat],
      zoom: center.zoom,
      attributionControl: false,
    })

    map.addControl(new maplibregl.NavigationControl(), 'top-right')
    map.addControl(new maplibregl.ScaleControl(), 'bottom-left')

    map.on('load', () => {
      addMarkers()
    })
  } catch (e) {
    console.warn('Failed to load MapLibre GL JS:', e)
  }
}

function destroyMap() {
  if (map) {
    markers.forEach(m => m.remove())
    markers = []
    map.remove()
    map = null
  }
}

function getMapCenter() {
  if (geoMarkers.value.length === 0) {
    return { lat: 20, lng: 0, zoom: 2 }
  }
  const lats = geoMarkers.value.map(m => m.lat)
  const lngs = geoMarkers.value.map(m => m.lng)
  const avgLat = lats.reduce((a, b) => a + b, 0) / lats.length
  const avgLng = lngs.reduce((a, b) => a + b, 0) / lngs.length
  const latSpan = Math.max(...lats) - Math.min(...lats)
  const lngSpan = Math.max(...lngs) - Math.min(...lngs)
  const span = Math.max(latSpan, lngSpan)
  const zoom = span > 100 ? 2 : span > 50 ? 3 : span > 20 ? 4 : span > 5 ? 6 : 8
  return { lat: avgLat, lng: avgLng, zoom }
}

function addMarkers() {
  if (!map) return
  markers.forEach(m => m.remove())
  markers = []

  geoMarkers.value.forEach((marker, idx) => {
    const color = MARKER_COLORS[idx % MARKER_COLORS.length]

    const el = document.createElement('div')
    el.className = 'maplibre-marker'
    el.style.cssText = `
      width: 20px; height: 20px; border-radius: 50%;
      background: ${color}; border: 2px solid #000;
      box-shadow: 0 0 8px ${color}, 0 0 16px ${color};
      cursor: pointer; transition: transform 0.2s;
    `
    el.addEventListener('mouseenter', () => { el.style.transform = 'scale(1.3)' })
    el.addEventListener('mouseleave', () => { el.style.transform = 'scale(1)' })

    const popup = new (map as any).Popup({
      closeButton: false,
      closeOnClick: false,
      offset: 15,
      className: 'cyber-popup',
    }).setHTML(`
      <div style="font-family: 'Inter', sans-serif; padding: 4px 0;">
        <div style="font-weight: 700; font-size: 12px; margin-bottom: 2px;">${marker.fileName}</div>
        <div style="font-size: 10px; font-family: monospace; color: #00D4FF;">${marker.lat.toFixed(4)}, ${marker.lng.toFixed(4)}</div>
        ${marker.address ? `<div style="font-size: 10px; color: #6B7280; margin-top: 2px;">${marker.address}</div>` : ''}
      </div>
    `)

    const m = new (map as any).Marker({ element: el })
      .setLngLat([marker.lng, marker.lat])
      .setPopup(popup)
      .addTo(map)

    markers.push(m)
  })
}

function updateMarkers() {
  if (!map) return
  addMarkers()
}

function flyToMarker(marker: GeoMarker) {
  if (map) {
    map.flyTo({ center: [marker.lng, marker.lat], zoom: 12, essential: true })
  }
}

async function handleRefresh() {
  await store.fetchGeoFiles()
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

.map-view::-webkit-scrollbar { width: 6px; }
.map-view::-webkit-scrollbar-track { background: #0a0a0f; }
.map-view::-webkit-scrollbar-thumb { background: #333; border-radius: 3px; }

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

.icon-map { color: #00D4FF; filter: drop-shadow(0 0 6px #00D4FF); }

.panel-title {
  font-size: 18px;
  font-weight: 800;
  text-transform: uppercase;
  letter-spacing: 2px;
  color: #00D4FF;
  text-shadow: 0 0 10px #00D4FF, 0 0 20px rgba(0, 212, 255, 0.3);
  margin: 0;
}

.header-actions { display: flex; gap: 8px; }

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
.refresh-btn:hover { border-color: #00D4FF; color: #00D4FF; }

.map-container {
  width: 100%;
  position: relative;
  min-height: 360px;
  border: 3px solid #000;
  box-shadow: 4px 4px 0 0 #000;
  overflow: hidden;
}

.maplibre-map {
  width: 100%;
  height: 360px;
}

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

.section { display: flex; flex-direction: column; gap: 10px; }

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

.geo-list { display: flex; flex-direction: column; gap: 6px; }

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
.geo-list-item:hover { border-color: #00D4FF; }

.geo-list-pin { color: #00D4FF; flex-shrink: 0; }

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

.empty-icon { color: #1a1a2e; }

.empty-state p {
  font-size: 13px;
  color: #6B7280;
  margin: 0;
  line-height: 1.6;
  max-width: 300px;
}

.loading-spinner {
  width: 32px;
  height: 32px;
  border: 3px solid #1a1a2e;
  border-top-color: #00D4FF;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin { to { transform: rotate(360deg); } }

.status-footer {
  margin-top: auto;
  padding-top: 12px;
  border-top: 2px solid #1a1a2e;
  font-size: 10px;
  color: #4B5563;
  text-align: center;
  letter-spacing: 0.5px;
}

.mono { font-family: 'JetBrains Mono', 'Fira Code', monospace; }
</style>

<style>
.cyber-popup .maplibregl-popup-content {
  background: #1a1a2e;
  border: 2px solid #000;
  box-shadow: 3px 3px 0 0 #000;
  padding: 8px 12px;
  color: #F5F5F4;
}
.cyber-popup .maplibregl-popup-tip {
  border-top-color: #1a1a2e;
}
</style>
