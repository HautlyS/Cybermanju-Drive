<template>
  <Teleport to="body">
    <div v-if="visible" class="upload-overlay" @click.self="$emit('close')">
      <div ref="uploadRef" class="upload-modal">
        <div class="upload-header">
          <span>FILE UPLOAD</span>
          <button class="upload-close" @click="$emit('close')">[X]</button>
        </div>

        <div
          class="drop-zone"
          :class="{ 'drop-active': isDragging }"
          @dragover.prevent="isDragging = true"
          @dragleave.prevent="isDragging = false"
          @drop.prevent="handleDrop"
        >
          <span v-if="!isDragging">DROP FILES HERE OR CLICK TO BROWSE</span>
          <span v-else>RELEASE TO UPLOAD</span>
          <input ref="fileInput" type="file" multiple class="file-input-hidden" @change="handleFileInput" />
        </div>

        <div v-if="files.length > 0" class="upload-files">
          <div v-for="(f, idx) in files" :key="idx" class="upload-file-row" :class="{ done: f.status === 'done', error: f.status === 'error' }">
            <span class="uf-name truncate">{{ f.name }}</span>
            <span class="uf-size text-muted">{{ formatSize(f.size) }}</span>
            <span class="uf-status">{{ f.status === 'uploading' ? 'UPLOADING..' : f.status === 'done' ? 'DONE' : f.status === 'error' ? 'FAILED' : 'PENDING' }}</span>
            <span v-if="f.error" class="uf-error text-muted">{{ f.error }}</span>
          </div>
        </div>

        <div class="upload-footer" v-if="files.length > 0">
          <span class="upload-progress-text">{{ completedCount }}/{{ files.length }} FILES</span>
          <button class="bw-btn" @click="startUpload" :disabled="isUploading">[UPLOAD]</button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, toRef, computed, watch, nextTick } from 'vue'
import { useAppStore } from '@/stores/app'
import { invoke } from '@/composables/useTauri'
import { useFocusTrap } from '@/composables/useFocusTrap'

const props = defineProps<{ visible: boolean }>()
const emit = defineEmits<{ close: [] }>()
const uploadRef = ref<HTMLElement | null>(null)
useFocusTrap(uploadRef, toRef(props, 'visible'))

const store = useAppStore()
const fileInput = ref<HTMLInputElement | null>(null)
const isDragging = ref(false)
const isUploading = ref(false)

interface UploadFile {
  name: string
  size: number
  data: ArrayBuffer
  status: 'pending' | 'uploading' | 'done' | 'error'
  error?: string
}

const files = ref<UploadFile[]>([])

const completedCount = computed(() => files.value.filter(f => f.status === 'done').length)

function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const k = 1024
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + units[i]
}

async function readFile(file: File): Promise<ArrayBuffer> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => resolve(reader.result as ArrayBuffer)
    reader.onerror = reject
    reader.readAsArrayBuffer(file)
  })
}

async function handleDrop(e: DragEvent) {
  isDragging.value = false
  const droppedFiles = Array.from(e.dataTransfer?.files || [])
  for (const f of droppedFiles) {
    const data = await readFile(f)
    files.value.push({ name: f.name, size: f.size, data, status: 'pending' })
  }
}

function handleFileInput(e: Event) {
  const input = e.target as HTMLInputElement
  const selectedFiles = Array.from(input.files || [])
  Promise.all(selectedFiles.map(async f => {
    const data = await readFile(f)
    files.value.push({ name: f.name, size: f.size, data, status: 'pending' })
  }))
  if (input) input.value = ''
}

async function startUpload() {
  isUploading.value = true
  for (const f of files.value) {
    if (f.status === 'done' || f.status === 'uploading') continue
    f.status = 'uploading'
    try {
      const uint8 = new Uint8Array(f.data)
      await invoke('upload_file', { fileName: f.name, fileData: Array.from(uint8), parentPath: store.currentPath })
      f.status = 'done'
    } catch (e) {
      f.status = 'error'
      f.error = e instanceof Error ? e.message : String(e)
    }
  }
  isUploading.value = false
  await store.fetchFiles()
  emit('close')
}
</script>

<style scoped>
.upload-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0,0,0,0.8);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 200;
  font-family: 'Courier New', monospace;
}

.upload-modal {
  background: #000;
  border: 2px solid #FFFFFF;
  width: 480px;
  max-width: 90vw;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  color: #FFFFFF;
}

.upload-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  border-bottom: 2px solid #FFFFFF;
  font-weight: 700;
  font-size: 12px;
  letter-spacing: 1px;
}

.upload-close {
  background: transparent;
  border: 2px solid #FFFFFF;
  color: #FFFFFF;
  padding: 2px 6px;
  cursor: pointer;
  font-family: 'Courier New', monospace;
  font-size: 9px;
}

.upload-close:hover {
  background: #FFFFFF;
  color: #000;
}

.drop-zone {
  border: 2px dashed rgba(255,255,255,0.3);
  margin: 12px;
  padding: 32px;
  text-align: center;
  cursor: pointer;
  font-size: 10px;
  color: rgba(255,255,255,0.5);
  transition: border-color 0.15s, background 0.15s;
}

.drop-zone:hover,
.drop-zone.drop-active {
  border-color: #FFFFFF;
  background: rgba(255,255,255,0.05);
}

.file-input-hidden {
  display: none;
}

.upload-files {
  flex: 1;
  overflow-y: auto;
  padding: 0 12px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  max-height: 240px;
}

.upload-file-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 8px;
  border: 1px solid rgba(255,255,255,0.2);
  font-size: 9px;
}

.upload-file-row.done {
  border-color: rgba(255,255,255,0.5);
  opacity: 0.6;
}

.upload-file-row.error {
  border-color: #FFFFFF;
  background: rgba(255,255,255,0.08);
}

.uf-name { flex: 1; }
.uf-size { flex-shrink: 0; }
.uf-status { flex-shrink: 0; font-weight: 700; }
.uf-error { flex: 1; text-align: right; font-size: 8px; }

.upload-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  border-top: 2px solid #FFFFFF;
}

.upload-progress-text {
  font-size: 10px;
  color: rgba(255,255,255,0.5);
}

.bw-btn {
  background: transparent;
  border: 2px solid #FFFFFF;
  color: #FFFFFF;
  padding: 4px 12px;
  cursor: pointer;
  font-family: 'Courier New', monospace;
  font-size: 10px;
  font-weight: 700;
}

.bw-btn:hover {
  background: #FFFFFF;
  color: #000;
}

.bw-btn:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

.text-muted { color: rgba(255,255,255,0.5) !important; }
.truncate { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
