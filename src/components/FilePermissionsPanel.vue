<template>
  <div class="perms-panel">
    <div class="panel-header">
      <div class="header-left">
        <span class="icon-perms">[!]</span>
        <h2 class="panel-title">FILE PERMISSIONS</h2>
      </div>
      <button class="close-btn" @click="$emit('close')" aria-label="CLOSE">[X]</button>
    </div>

    <div v-if="!store.selectedFile" class="empty-state">
      <p class="text-muted">SELECT A FILE TO VIEW PERMISSIONS</p>
    </div>

    <template v-else>
      <div class="file-info">
        <span class="fi-name">{{ store.selectedFile.name }}</span>
        <span class="fi-path text-muted">{{ store.selectedFile.path }}</span>
      </div>

      <div class="section">
        <h3 class="section-title">[PERMISSIONS] ACCESS CONTROL</h3>
        <div v-if="permissions.length === 0" class="text-muted" style="font-size:10px;">NO PERMISSIONS SET</div>
        <div v-else class="perms-list">
          <div v-for="(perm, idx) in permissions" :key="perm.userId + idx" class="perm-row">
            <span class="perm-user">{{ perm.username }}</span>
            <span class="perm-access">{{ perm.access.toUpperCase() }}</span>
            <button class="perm-revoke" @click="handleRevoke(perm)" title="REVOKE ACCESS">[X]</button>
          </div>
        </div>
      </div>

      <div class="section grant-section">
        <h3 class="section-title">[SHARE] SHARE LINK</h3>
        <div class="share-row">
          <input :value="shareLink" class="bw-input" readonly style="flex:1;" @click="($event.target as HTMLInputElement).select()" />
          <button class="bw-btn" @click="copyShareLink" :disabled="!shareLink">[COPY]</button>
        </div>
        <div v-if="shareCopied" class="share-copied text-muted">LINK COPIED TO CLIPBOARD</div>
      </div>

      <div class="section grant-section">
        <h3 class="section-title">[+] GRANT ACCESS</h3>
        <div class="grant-row">
          <select v-model="grantUserId" class="bw-input" style="flex:1;">
            <option value="" disabled>SELECT USER</option>
            <option v-for="u in store.users" :key="u.id" :value="u.id">{{ u.username }} ({{ u.role }})</option>
          </select>
        </div>
        <div class="grant-row">
          <select v-model="grantAccess" class="bw-input" style="flex:1;">
            <option value="read">READ</option>
            <option value="write">WRITE</option>
            <option value="admin">ADMIN</option>
          </select>
          <button class="bw-btn" @click="handleGrant" :disabled="!grantUserId">[GRANT]</button>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useAppStore } from '@/stores/app'
import { invoke } from '@/composables/useTauri'
import type { FilePermission } from '@/types'

defineEmits<{ close: [] }>()

const store = useAppStore()

const permissions = ref<FilePermission[]>([])
const grantUserId = ref('')
const grantAccess = ref<'read' | 'write' | 'admin'>('read')
const shareCopied = ref(false)

const shareLink = computed(() => {
  if (!store.selectedFile) return ''
  const base = window.location.origin || 'http://localhost:3456'
  return `${base}/share/${store.selectedFile.id}`
})

watch(() => store.selectedFileId, async (id) => {
  if (id) {
    await fetchPermissions(id)
  } else {
    permissions.value = []
  }
})

async function fetchPermissions(fileId: string) {
  try {
    const perms = await invoke<FilePermission[]>('list_file_permissions', { fileId })
    permissions.value = perms
  } catch {
    permissions.value = []
  }
}

async function handleGrant() {
  if (!grantUserId.value || !store.selectedFileId) return
  try {
    await invoke('grant_file_permission', {
      fileId: store.selectedFileId,
      userId: grantUserId.value,
      access: grantAccess.value,
    })
    store.notifySuccess('Permission granted')
    await fetchPermissions(store.selectedFileId)
    grantUserId.value = ''
  } catch (e) {
    store.notifyError('Failed to grant permission', e)
  }
}

async function copyShareLink() {
  if (!shareLink.value) return
  try {
    await navigator.clipboard.writeText(shareLink.value)
    shareCopied.value = true
    setTimeout(() => { shareCopied.value = false }, 2000)
  } catch {
    store.notifyError('Failed to copy link', '')
  }
}

async function handleRevoke(perm: FilePermission) {
  if (!store.selectedFileId) return
  try {
    await invoke('revoke_file_permission', {
      fileId: store.selectedFileId,
      userId: perm.userId,
    })
    store.notifySuccess('Permission revoked')
    await fetchPermissions(store.selectedFileId)
  } catch (e) {
    store.notifyError('Failed to revoke permission', e)
  }
}
</script>

<style scoped>
.perms-panel {
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
.icon-perms { font-size: 16px; }
.panel-title { font-size: 14px; font-weight: 800; letter-spacing: 1px; margin: 0; }

.close-btn {
  background: transparent;
  border: 2px solid #FFFFFF;
  color: #FFFFFF;
  padding: 2px 6px;
  cursor: pointer;
  font-family: 'Courier New', monospace;
  font-size: 9px;
}

.close-btn:hover {
  background: #FFFFFF;
  color: #000;
}

.empty-state {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 120px;
}

.file-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-bottom: 16px;
  padding-bottom: 10px;
  border-bottom: 1px solid rgba(255,255,255,0.2);
}

.fi-name { font-size: 13px; font-weight: 700; }
.fi-path { font-size: 9px; }

.section { margin-bottom: 16px; }

.section-title {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 1px;
  color: rgba(255,255,255,0.6);
  margin: 0 0 8px;
}

.perms-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.perm-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border: 1px solid rgba(255,255,255,0.2);
  font-size: 10px;
}

.perm-user { flex: 1; font-weight: 600; }
.perm-access { font-size: 9px; border: 1px solid #FFFFFF; padding: 0 4px; }

.perm-revoke {
  background: transparent;
  border: 1px solid #FFFFFF;
  color: #FFFFFF;
  padding: 1px 4px;
  cursor: pointer;
  font-family: 'Courier New', monospace;
  font-size: 8px;
}

.perm-revoke:hover {
  background: #FFFFFF;
  color: #000;
}

.grant-section {
  padding-top: 12px;
  border-top: 2px solid #FFFFFF;
}

.grant-row {
  display: flex;
  gap: 6px;
  margin-bottom: 6px;
}

.bw-input {
  background: #000;
  border: 2px solid #FFFFFF;
  color: #FFFFFF;
  font-family: 'Courier New', monospace;
  font-size: 10px;
  padding: 4px 6px;
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
  white-space: nowrap;
}

.bw-btn:hover {
  background: #FFFFFF;
  color: #000;
}

.bw-btn:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

.share-row {
  display: flex;
  gap: 6px;
  margin-bottom: 6px;
}

.share-copied {
  font-size: 9px;
  margin-bottom: 8px;
}

.text-muted { color: rgba(255,255,255,0.5) !important; }
</style>
