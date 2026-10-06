<template>
  <Teleport to="body">
    <div v-if="visible" class="upload-overlay" @click.self="$emit('close')">
      <div ref="uploadRef" class="upload-modal">
        <div class="upload-header">
          <span>FILE UPLOAD</span>
          <button class="upload-close" @click="$emit('close')" aria-label="CLOSE" title="CLOSE"><AppIcon name="solar:close-bold" :size="14" /></button>
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
            <span class="uf-size text-muted">{{ humanBytes(f.size) }}</span>
            <span class="uf-status">{{ f.status === 'uploading' ? 'UPLOADING..' : f.status === 'done' ? 'DONE' : f.status === 'error' ? 'FAILED' : 'PENDING' }}</span>
            <span v-if="f.error" class="uf-error text-muted">{{ f.error }}</span>
          </div>
        </div>

        <div class="upload-footer" v-if="files.length > 0">
          <span class="upload-progress-text">{{ completedCount }}/{{ files.length }} FILES</span>
          <button class="bw-btn" @click="startUpload" :disabled="isUploading"><AppIcon name="solar:upload-bold" :size="13" /> UPLOAD</button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import AppIcon from '@/components/AppIcon.vue'
import { ref, toRef, computed, watch, nextTick } from 'vue'
import { useAppStore } from '@/stores/app'
import { humanBytes } from '@/utils/format'
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
  background: color-mix(in srgb, var(--ui-bg-deep) 62%, transparent);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 200;
  font-family: var(--ui-font);
  backdrop-filter: blur(6px);
  -webkit-backdrop-filter: blur(6px);
}

.upload-modal {background: var(--ui-glass-2);
  border: 1px solid var(--ui-border);
  width: 480px;
  max-width: 90vw;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  color: var(--ui-text);
  border-radius: var(--ui-radius-lg);
  backdrop-filter: blur(var(--ui-blur)) saturate(var(--ui-saturate));
  -webkit-backdrop-filter: blur(var(--ui-blur)) saturate(var(--ui-saturate));
}

.upload-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  border-bottom: 1px solid var(--ui-border);
  font-weight: 700;
  font-size: 12px;
  letter-spacing: 1px;
}

.upload-close {
  background: transparent;
  border: 1px solid var(--ui-border);
  color: var(--ui-text);
  padding: 2px 6px;
  cursor: pointer;
  font-family: var(--ui-font);
  font-size: 9px;
}

.upload-close:hover {
  background: var(--ui-glass-2);
  color: var(--ui-text);
}

.drop-zone {
  border: 2px dashed var(--ui-border);
  margin: 12px;
  padding: 32px;
  text-align: center;
  cursor: pointer;
  font-size: 10px;
  color: color-mix(in srgb, var(--ui-text) 50%, transparent);
  transition: border-color 0.15s, background 0.15s;
}

.drop-zone:hover,
.drop-zone.drop-active {
  border-color: var(--ui-border-strong);
  background: color-mix(in srgb, var(--ui-text) 6%, transparent);
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
  border: 1px solid var(--ui-border);
  font-size: 9px;
}

.upload-file-row.done {
  border-color: var(--ui-border);
  opacity: 0.6;
}

.upload-file-row.error {
  border-color: var(--ui-border-strong);
  background: color-mix(in srgb, var(--ui-text) 6%, transparent);
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
  border-top: 1px solid var(--ui-border);
}

.upload-progress-text {
  font-size: 10px;
  color: color-mix(in srgb, var(--ui-text) 50%, transparent);
}

.bw-btn {
  background: transparent;
  border: 1px solid var(--ui-border);
  color: var(--ui-text);
  padding: 4px 12px;
  cursor: pointer;
  font-family: var(--ui-font);
  font-size: 10px;
  font-weight: 700;
}

.bw-btn:hover {
  background: var(--ui-glass-2);
  color: var(--ui-text);
}

.bw-btn:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

.text-muted { color: color-mix(in srgb, var(--ui-text) 50%, transparent) !important; }
.truncate { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
