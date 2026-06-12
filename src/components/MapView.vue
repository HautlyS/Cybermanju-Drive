<template>
  <div class="map-view">
    <div class="panel-header">
      <div class="header-left">
        <span class="icon-map">[@]</span>
        <h2 class="panel-title">GEOGRAPHY VIEW</h2>
      </div>
      <div class="header-actions">
        <button class="refresh-btn" @click="handleRefresh" title="REFRESH">[R]</button>
      </div>
    </div>

    <div class="map-container" v-if="geoMarkers.length > 0">
      <div ref="mapContainer" class="maplibre-map"></div>
      <div class="map-stats-overlay">
        <span>{{ geoMarkers.length }} LOCATIONS</span>
      </div>
    </div>

    <div class="empty-state" v-if="geoMarkers.length === 0 && !isLoading">
      <p>NO GEOTAGGED FILES FOUND. PHOTOS WITH GPS EXIF DATA WILL APPEAR HERE.</p>
    </div>

    <div class="empty-state" v-if="isLoading">
      <div class="loading-spinner"></div>
      <p>LOADING GEO DATA..</p>
    </div>

    <div class="section" v-if="geoMarkers.length > 0">
      <h3 class="section-title">[LIST] GEOTAGGED FILES ({{ geoMarkers.length }})</h3>
      <div class="geo-list">
        <div v-for="marker in geoMarkers" :key="'list-' + marker.fileId" class="geo-list-item" @click="flyToMarker(marker)">
          <span class="geo-list-pin">[@]</span>
          <div class="geo-list-info">
            <span class="geo-list-name">{{ marker.fileName }}</span>
            <span class="geo-list-address text-muted" v-if="marker.address">{{ marker.address }}</span>
          </div>
          <span class="geo-list-coords mono">{{ marker.lat.toFixed(3) }}, {{ marker.lng.toFixed(3) }}</span>
        </div>
      </div>
    </div>

    <div class="status-footer">
      <span>GPS EXTRACTION VIA KAMADAK-EXIF (RUST) | MAPLIBRE GL</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { useAppStore } from '@/stores/app'
import type { GeoMarker } from '@/types'

const store = useAppStore()
const emit = defineEmits<{ close: [] }>()

const geoMarkers = computed(() => store.geoMarkers)
const isLoading = computed(() => store.isLoading)
const mapContainer = ref<HTMLDivElement | null>(null)

let map: any = null
let markers: any[] = []

onMounted(async () => {
  await store.fetchGeoFiles()
  await nextTick()
  if (geoMarkers.value.length > 0) initMap()
})

onUnmounted(() => destroyMap())

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
    const center = getMapCenter()
    map = new maplibregl.Map({
      container: mapContainer.value,
      style: {
        version: 8,
        sources: {
          osm: { type: 'raster', tiles: ['https://tile.openstreetmap.org/{z}/{x}/{y}.png'], tileSize: 256, attribution: '&copy; OpenStreetMap contributors' },
        },
        layers: [{ id: 'osm', type: 'raster', source: 'osm' }],
      },
      center: [center.lng, center.lat],
      zoom: center.zoom,
      attributionControl: false,
    })
    map.on('load', () => addMarkers())
  } catch (e) { console.warn('MapLibre GL failed:', e) }
}

function destroyMap() {
  if (map) {
    markers.forEach((m: any) => m.remove())
    markers = []
    map.remove()
    map = null
  }
}

function getMapCenter() {
  if (geoMarkers.value.length === 0) return { lat: 20, lng: 0, zoom: 2 }
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
  markers.forEach((m: any) => m.remove())
  markers = []
  geoMarkers.value.forEach((marker) => {
    const el = document.createElement('div')
    el.className = 'bw-marker'
    el.style.cssText = 'width:16px;height:16px;border:2px solid #000;background:#fff;cursor:pointer;'
    el.addEventListener('mouseenter', () => { el.style.transform = 'scale(1.4)' })
    el.addEventListener('mouseleave', () => { el.style.transform = 'scale(1)' })
    const m = new (map as any).Marker({ element: el }).setLngLat([marker.lng, marker.lat]).addTo(map)
    markers.push(m)
  })
}

function updateMarkers() { addMarkers() }

function flyToMarker(marker: GeoMarker) {
  if (map) map.flyTo({ center: [marker.lng, marker.lat], zoom: 12, essential: true })
}

async function handleRefresh() { await store.fetchGeoFiles() }
</script>

<style scoped>
.map-view {
  width: 100%;
  height: 100%;
  background: #000;
  border: 2px solid #FFFFFF;
  overflow-y: auto;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  font-family: 'Courier New', monospace;
  color: #FFFFFF;
}

.map-view::-webkit-scrollbar { width: 4px; }
.map-view::-webkit-scrollbar-track { background: #000; }
.map-view::-webkit-scrollbar-thumb { background: #FFFFFF; }

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-bottom: 10px;
  border-bottom: 2px solid #FFFFFF;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 8px;
}

.icon-map { font-size: 16px; color: #FFFFFF; }

.panel-title {
  font-size: 14px;
  font-weight: 800;
  letter-spacing: 1px;
  color: #FFFFFF;
  margin: 0;
}

.header-actions { display: flex; gap: 6px; }

.refresh-btn {
  background: #000;
  border: 2px solid #FFFFFF;
  color: #FFFFFF;
  cursor: pointer;
  padding: 2px 6px;
  font-family: 'Courier New', monospace;
  font-size: 10px;
  font-weight: 700;
}

.refresh-btn:hover { background: #FFFFFF; color: #000; }

.map-container {
  width: 100%;
  position: relative;
  min-height: 300px;
  border: 2px solid #FFFFFF;
  overflow: hidden;
}

.maplibre-map { width: 100%; height: 300px; }

.map-stats-overlay {
  position: absolute;
  bottom: 6px;
  right: 6px;
  background: #000;
  border: 2px solid #FFFFFF;
  padding: 2px 6px;
  font-size: 9px;
  color: #FFFFFF;
  font-family: 'Courier New', monospace;
  z-index: 5;
}

.section { display: flex; flex-direction: column; gap: 8px; }

.section-title {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 1px;
  color: rgba(255,255,255,0.6);
  margin: 0;
  padding-bottom: 4px;
  border-bottom: 2px solid rgba(255,255,255,0.2);
  display: flex;
  align-items: center;
  gap: 6px;
}

.geo-list { display: flex; flex-direction: column; gap: 4px; }

.geo-list-item {
  border: 2px solid #FFFFFF;
  padding: 6px 10px;
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
}

.geo-list-item:hover { background: rgba(255,255,255,0.1); }

.geo-list-pin { flex-shrink: 0; font-size: 11px; }

.geo-list-info {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
}

.geo-list-name { font-size: 11px; font-weight: 700; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.geo-list-address { font-size: 9px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.geo-list-coords { font-size: 9px; color: rgba(255,255,255,0.5); flex-shrink: 0; }

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  padding: 40px;
  text-align: center;
}

.empty-state p { font-size: 11px; color: rgba(255,255,255,0.5); margin: 0; }

.loading-spinner {
  width: 24px;
  height: 24px;
  border: 3px solid rgba(255,255,255,0.2);
  border-top-color: #FFFFFF;
  animation: spin 0.8s linear infinite;
}

@keyframes spin { to { transform: rotate(360deg); } }

.status-footer {
  margin-top: auto;
  padding-top: 10px;
  border-top: 2px solid rgba(255,255,255,0.2);
  font-size: 9px;
  color: rgba(255,255,255,0.3);
  text-align: center;
}

.mono { font-family: 'Courier New', monospace; }
.text-muted { color: rgba(255,255,255,0.5) !important; }
</style>

<style>
.maplibregl-popup-content {
  background: #000 !important;
  border: 2px solid #FFFFFF !important;
  box-shadow: 3px 3px 0 #000 !important;
  padding: 6px 10px !important;
  color: #FFFFFF !important;
  font-family: 'Courier New', monospace !important;
  font-size: 11px !important;
}
.maplibregl-popup-tip { border-top-color: #000 !important; }
</style>
