<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore } from '@/stores/app'
import { User, UserFilePermission, AuthResult, CYBER } from '@/types'
import { UserCheck, UserPlus, Shield, Trash2, LogIn, Key, Users } from 'lucide-vue-next'

const store = useAppStore()

// ── State ──
const users = ref<User[]>([])
const filePermissions = ref<UserFilePermission[]>([])
const loading = ref(false)
const message = ref<{ text: string; type: 'success' | 'error' } | null>(null)

// Register form
const regForm = ref({ username: '', password: '', displayName: '', role: 'user' as User['role'] })
const regLoading = ref(false)

// Permission form
const permForm = ref({ userId: '', access: 'read' as UserFilePermission['access'] })
const permLoading = ref(false)

// Auth test
const authForm = ref({ username: '', password: '' })
const authLoading = ref(false)
const authResult = ref<AuthResult | null>(null)

// ── Computed ──
const selectedFile = computed(() =>
  store.files.find(f => f.id === store.selectedFileId)
)

const activeUsers = computed(() => users.value.filter(u => u.isActive))

function roleColor(role: User['role']): string {
  switch (role) {
    case 'admin': return CYBER.matrixGreen
    case 'user': return CYBER.cyberBlue
    case 'viewer': return '#6B7280'
  }
}

function accessColor(access: UserFilePermission['access']): string {
  switch (access) {
    case 'admin': return CYBER.saffronGold
    case 'write': return CYBER.matrixGreen
    case 'read': return CYBER.cyberBlue
  }
}

function formatDate(iso: string): string {
  try { return new Date(iso).toLocaleString() } catch { return iso }
}

function flash(text: string, type: 'success' | 'error' = 'success') {
  message.value = { text, type }
  setTimeout(() => { message.value = null }, 3500)
}

// ── Actions ──
async function loadUsers() {
  loading.value = true
  try {
    users.value = await invoke<User[]>('list_users')
  } catch (e: any) {
    console.error('Failed to load users', e)
    flash(String(e), 'error')
  } finally {
    loading.value = false
  }
}

async function loadPermissions() {
  if (!store.selectedFileId) return
  try {
    filePermissions.value = await invoke<UserFilePermission[]>('get_file_permissions', {
      fileId: store.selectedFileId,
    })
  } catch (e: any) {
    console.error('Failed to load permissions', e)
  }
}

async function registerUser() {
  regLoading.value = true
  try {
    const result = await invoke<User>('register_user', {
      username: regForm.value.username,
      password: regForm.value.password,
      displayName: regForm.value.displayName || regForm.value.username,
      role: regForm.value.role,
    })
    flash(`User "${result.username}" registered successfully`)
    regForm.value = { username: '', password: '', displayName: '', role: 'user' }
    await loadUsers()
  } catch (e: any) {
    flash(String(e), 'error')
  } finally {
    regLoading.value = false
  }
}

async function addPermission() {
  if (!store.selectedFileId || !permForm.value.userId) return
  permLoading.value = true
  try {
    await invoke('grant_file_permission', {
      fileId: store.selectedFileId,
      userId: permForm.value.userId,
      access: permForm.value.access,
      grantedBy: 'current_admin',
    })
    flash('Permission granted')
    permForm.value = { userId: '', access: 'read' }
    await loadPermissions()
  } catch (e: any) {
    flash(String(e), 'error')
  } finally {
    permLoading.value = false
  }
}

async function removePermission(perm: UserFilePermission) {
  try {
    await invoke('revoke_file_permission', {
      permissionId: perm.id,
    })
    flash('Permission revoked')
    await loadPermissions()
  } catch (e: any) {
    flash(String(e), 'error')
  }
}

async function authenticate() {
  authLoading.value = true
  authResult.value = null
  try {
    authResult.value = await invoke<AuthResult>('authenticate_user', {
      username: authForm.value.username,
      password: authForm.value.password,
    })
    flash(`Authenticated as "${authResult.value.username}"`)
  } catch (e: any) {
    flash(String(e), 'error')
  } finally {
    authLoading.value = false
  }
}

onMounted(() => {
  loadUsers()
})
</script>

<template>
  <div class="user-mgmt-root">
    <!-- Header -->
    <div class="panel-header">
      <UserCheck :size="22" class="icon-gold" />
      <h2 class="panel-title">User Access Control</h2>
    </div>

    <!-- Flash message -->
    <Transition name="flash">
      <div v-if="message" :class="['flash-bar', message.type]">
        {{ message.text }}
      </div>
    </Transition>

    <!-- ═══════════ USER LIST ═══════════ -->
    <section class="neo-card">
      <div class="card-head">
        <Users :size="18" />
        <span>Registered Users ({{ activeUsers.length }})</span>
      </div>

      <div v-if="loading" class="mono-sm muted">Loading users...</div>

      <div v-else-if="users.length === 0" class="mono-sm muted">No users registered yet.</div>

      <div v-else class="user-list">
        <div v-for="u in users" :key="u.id" class="user-row" :class="{ inactive: !u.isActive }">
          <div class="user-info">
            <span class="mono-md">{{ u.username }}</span>
            <span v-if="u.displayName" class="subtle">{{ u.displayName }}</span>
          </div>
          <span class="role-badge" :style="{ borderColor: roleColor(u.role), color: roleColor(u.role) }">
            <Shield :size="12" />
            {{ u.role }}
          </span>
        </div>
      </div>
    </section>

    <!-- ═══════════ REGISTER USER ═══════════ -->
    <section class="neo-card">
      <div class="card-head">
        <UserPlus :size="18" />
        <span>Register User</span>
      </div>

      <form class="neo-form" @submit.prevent="registerUser">
        <label class="neo-label">Username</label>
        <input
          v-model="regForm.username"
          class="neo-input"
          placeholder="enter_username"
          required
          spellcheck="false"
        />

        <label class="neo-label">Password</label>
        <input
          v-model="regForm.password"
          type="password"
          class="neo-input"
          placeholder="••••••••"
          required
        />

        <label class="neo-label">Display Name</label>
        <input
          v-model="regForm.displayName"
          class="neo-input"
          placeholder="Optional display name"
          spellcheck="false"
        />

        <label class="neo-label">Role</label>
        <select v-model="regForm.role" class="neo-select">
          <option value="user">User</option>
          <option value="admin">Admin</option>
          <option value="viewer">Viewer</option>
        </select>

        <button class="neo-btn gold" type="submit" :disabled="regLoading">
          <UserPlus :size="16" />
          {{ regLoading ? 'Registering...' : 'Register User' }}
        </button>
      </form>
    </section>

    <!-- ═══════════ FILE PERMISSIONS ═══════════ -->
    <section v-if="selectedFile" class="neo-card">
      <div class="card-head">
        <Key :size="18" />
        <span>Permissions: <span class="mono-sm">{{ selectedFile.name }}</span></span>
      </div>

      <!-- Add permission -->
      <div class="perm-add-row">
        <select v-model="permForm.userId" class="neo-select perm-select">
          <option value="" disabled>Select user</option>
          <option v-for="u in users" :key="u.id" :value="u.id">{{ u.username }}</option>
        </select>

        <select v-model="permForm.access" class="neo-select perm-select">
          <option value="read">Read</option>
          <option value="write">Write</option>
          <option value="admin">Admin</option>
        </select>

        <button class="neo-btn green sm" :disabled="permLoading || !permForm.userId" @click="addPermission">
          <UserPlus :size="14" />
          {{ permLoading ? '...' : 'Grant' }}
        </button>
      </div>

      <!-- Current permissions -->
      <div v-if="filePermissions.length === 0" class="mono-sm muted" style="margin-top: 12px">
        No explicit permissions set.
      </div>

      <div v-else class="perm-list">
        <div v-for="p in filePermissions" :key="p.id" class="perm-row">
          <span class="mono-sm perm-user">
            {{ users.find(u => u.id === p.userId)?.username ?? p.userId }}
          </span>
          <span class="role-badge sm" :style="{ borderColor: accessColor(p.access), color: accessColor(p.access) }">
            {{ p.access }}
          </span>
          <span class="subtle perm-date">{{ formatDate(p.grantedAt) }}</span>
          <button class="icon-btn danger" title="Revoke" @click="removePermission(p)">
            <Trash2 :size="14" />
          </button>
        </div>
      </div>
    </section>

    <div v-else class="neo-card hint-card">
      <Key :size="16" />
      <span class="mono-sm muted">Select a file to manage its permissions.</span>
    </div>

    <!-- ═══════════ AUTH TEST ═══════════ -->
    <section class="neo-card">
      <div class="card-head">
        <LogIn :size="18" />
        <span>Authenticate (Test)</span>
      </div>

      <form class="neo-form" @submit.prevent="authenticate">
        <label class="neo-label">Username</label>
        <input
          v-model="authForm.username"
          class="neo-input"
          placeholder="username"
          required
          spellcheck="false"
        />

        <label class="neo-label">Password</label>
        <input
          v-model="authForm.password"
          type="password"
          class="neo-input"
          placeholder="••••••••"
          required
        />

        <button class="neo-btn blue" type="submit" :disabled="authLoading">
          <LogIn :size="16" />
          {{ authLoading ? 'Authenticating...' : 'Authenticate' }}
        </button>
      </form>

      <!-- Auth result -->
      <div v-if="authResult" class="auth-result-box">
        <div class="auth-ok">✓ Authenticated</div>
        <div class="auth-detail">
          <span class="label">User ID:</span>
          <span class="mono-sm">{{ authResult.userId }}</span>
        </div>
        <div class="auth-detail">
          <span class="label">Username:</span>
          <span class="mono-sm">{{ authResult.username }}</span>
        </div>
        <div class="auth-detail">
          <span class="label">Role:</span>
          <span class="role-badge sm" :style="{ borderColor: roleColor(authResult.role as any), color: roleColor(authResult.role as any) }">
            {{ authResult.role }}
          </span>
        </div>
        <div class="auth-detail">
          <span class="label">Token:</span>
          <span class="mono-sm token">{{ authResult.token.slice(0, 32) }}…</span>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.user-mgmt-root {
  display: flex;
  flex-direction: column;
  gap: 20px;
  padding: 20px;
  height: 100%;
  overflow-y: auto;
  font-family: system-ui, -apple-system, sans-serif;
  color: #F5F5F4;
}

/* ── Header ── */
.panel-header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding-bottom: 12px;
  border-bottom: 3px solid #000;
}
.panel-title {
  font-size: 18px;
  font-weight: 800;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: #FFB800;
  text-shadow: 0 0 12px rgba(255, 184, 0, 0.5);
}
.icon-gold {
  color: #FFB800;
  filter: drop-shadow(0 0 6px rgba(255, 184, 0, 0.6));
}

/* ── Flash ── */
.flash-bar {
  padding: 8px 14px;
  border: 3px solid #000;
  font-family: 'Courier New', monospace;
  font-size: 13px;
  background: #1a1a2e;
  box-shadow: 4px 4px 0 #000;
}
.flash-bar.success {
  border-color: #00FF41;
  color: #00FF41;
}
.flash-bar.error {
  border-color: #FF2D6F;
  color: #FF2D6F;
}
.flash-enter-active, .flash-leave-active { transition: opacity 0.25s, transform 0.25s; }
.flash-enter-from, .flash-leave-to { opacity: 0; transform: translateY(-6px); }

/* ── Neo Card ── */
.neo-card {
  background: #1a1a2e;
  border: 3px solid #000;
  box-shadow: 4px 4px 0 #000;
  padding: 16px;
}
.card-head {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 700;
  font-size: 14px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: 14px;
  color: #F5F5F4;
}
.hint-card {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 14px 18px;
}

/* ── Monospace helpers ── */
.mono-sm {
  font-family: 'Courier New', monospace;
  font-size: 13px;
}
.mono-md {
  font-family: 'Courier New', monospace;
  font-size: 14px;
  font-weight: 600;
}
.muted { color: #6B7280; }
.subtle { color: #9CA3AF; font-size: 12px; }

/* ── User list ── */
.user-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.user-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 10px;
  background: #12121a;
  border: 2px solid #000;
  box-shadow: 3px 3px 0 #000;
}
.user-row.inactive {
  opacity: 0.4;
}
.user-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

/* ── Role / access badges ── */
.role-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-family: 'Courier New', monospace;
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  padding: 2px 8px;
  border: 2px solid;
  background: #12121a;
  letter-spacing: 0.5px;
}
.role-badge.sm {
  font-size: 10px;
  padding: 1px 6px;
}

/* ── Forms ── */
.neo-form {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.neo-label {
  font-size: 12px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: #9CA3AF;
  margin-top: 4px;
}
.neo-input, .neo-select {
  background: #12121a;
  border: 3px solid #000;
  color: #F5F5F4;
  padding: 8px 12px;
  font-family: 'Courier New', monospace;
  font-size: 13px;
  outline: none;
  box-shadow: 3px 3px 0 #000;
  transition: border-color 0.15s;
}
.neo-input:focus, .neo-select:focus {
  border-color: #FFB800;
  box-shadow: 3px 3px 0 #FFB800;
}
.neo-input::placeholder {
  color: #4B5563;
}
.neo-select {
  cursor: pointer;
  appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' fill='%239CA3AF' viewBox='0 0 16 16'%3E%3Cpath d='M8 11L2 5h12z'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 10px center;
  padding-right: 30px;
}

/* ── Buttons ── */
.neo-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 10px 18px;
  font-family: system-ui, sans-serif;
  font-size: 13px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  border: 3px solid #000;
  box-shadow: 4px 4px 0 #000;
  cursor: pointer;
  transition: transform 0.1s, box-shadow 0.1s;
  margin-top: 4px;
}
.neo-btn:hover:not(:disabled) {
  transform: translate(1px, 1px);
  box-shadow: 3px 3px 0 #000;
}
.neo-btn:active:not(:disabled) {
  transform: translate(2px, 2px);
  box-shadow: 2px 2px 0 #000;
}
.neo-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
.neo-btn.gold {
  background: #FFB800;
  color: #000;
}
.neo-btn.green {
  background: #00FF41;
  color: #000;
}
.neo-btn.blue {
  background: #00D4FF;
  color: #000;
}
.neo-btn.sm {
  padding: 6px 12px;
  font-size: 12px;
  box-shadow: 3px 3px 0 #000;
}

.icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: 2px solid #000;
  box-shadow: 2px 2px 0 #000;
  padding: 4px;
  cursor: pointer;
  color: #F5F5F4;
  transition: transform 0.1s, box-shadow 0.1s;
}
.icon-btn:hover {
  transform: translate(1px, 1px);
  box-shadow: 1px 1px 0 #000;
}
.icon-btn.danger {
  border-color: #FF2D6F;
  color: #FF2D6F;
}
.icon-btn.danger:hover {
  background: #FF2D6F;
  color: #000;
}

/* ── Permission section ── */
.perm-add-row {
  display: flex;
  gap: 8px;
  align-items: flex-end;
}
.perm-select {
  flex: 1;
  min-width: 0;
}
.perm-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-top: 12px;
}
.perm-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 10px;
  background: #12121a;
  border: 2px solid #000;
  box-shadow: 2px 2px 0 #000;
}
.perm-user {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.perm-date {
  font-size: 11px;
  white-space: nowrap;
}

/* ── Auth result ── */
.auth-result-box {
  margin-top: 14px;
  padding: 12px;
  background: #12121a;
  border: 3px solid #00FF41;
  box-shadow: 4px 4px 0 #000;
}
.auth-ok {
  font-weight: 800;
  font-size: 14px;
  color: #00FF41;
  margin-bottom: 10px;
  text-transform: uppercase;
  letter-spacing: 1px;
}
.auth-detail {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
}
.auth-detail .label {
  font-size: 11px;
  text-transform: uppercase;
  color: #9CA3AF;
  width: 80px;
  flex-shrink: 0;
}
.token {
  color: #FFB800;
  word-break: break-all;
}

/* ── Scrollbar ── */
.user-mgmt-root::-webkit-scrollbar {
  width: 8px;
}
.user-mgmt-root::-webkit-scrollbar-track {
  background: #0a0a0f;
}
.user-mgmt-root::-webkit-scrollbar-thumb {
  background: #1a1a2e;
  border: 2px solid #000;
}
</style>