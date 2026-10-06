<template>
  <div class="user-panel">
    <div class="panel-header">
      <div class="header-left">
        <span class="icon-user"><AppIcon name="solar:users-group-rounded-bold" /></span>
        <h2 class="panel-title">USER MANAGEMENT</h2>
      </div>
    </div>

    <div class="section">
      <h3 class="section-title"><AppIcon name="solar:users-group-rounded-bold" :size="13" /> REGISTERED USERS</h3>
      <p class="text-muted" style="margin-bottom:8px;font-size:10px;">PER-FILE USERNAME + PASSWORD AUTH WITH ARGON2 HASHING. ROLE-BASED ACCESS: ADMIN, USER, VIEWER.</p>
      <div v-if="store.users.length === 0" class="text-muted" style="font-size:10px;">NO USERS REGISTERED</div>
      <div class="user-list">
        <div v-for="user in store.users" :key="user.id" class="user-card">
          <div class="user-header">
            <span class="user-name">{{ user.username }}</span>
            <span class="user-role">{{ user.role }}</span>
            <span class="user-active" :class="{ on: user.isActive }">{{ user.isActive ? 'ACTIVE' : 'INACTIVE' }}</span>
            <div class="user-actions">
              <button class="user-action-btn" @click="handleRole(user.id, user.role === 'admin' ? 'user' : 'admin')" title="TOGGLE ROLE" aria-label="TOGGLE ROLE"><AppIcon name="solar:user-check-bold" :size="13" /></button>
              <button class="user-action-btn" @click="handleDelete(user.id)" title="DELETE USER" aria-label="DELETE USER"><AppIcon name="solar:trash-bin-trash-bold" :size="13" /></button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div class="create-section" v-if="showCreate">
      <h3 class="section-title"><AppIcon name="solar:add-bold" :size="13" /> CREATE USER</h3>
      <input v-model="newUsername" class="bw-input" placeholder="USERNAME" @keyup.enter="handleCreate" />
      <input v-model="newPassword" class="bw-input" type="password" placeholder="PASSWORD" @keyup.enter="handleCreate" />
      <select v-model="newRole" class="bw-input" style="appearance:none;">
        <option value="user">USER</option>
        <option value="admin">ADMIN</option>
        <option value="viewer">VIEWER</option>
      </select>
      <button class="bw-btn" @click="handleCreate"><AppIcon name="solar:add-bold" :size="13" /> CREATE</button>
    </div>
    <button v-else class="bw-btn" @click="showCreate = true">[+ ADD USER]</button>
  </div>
</template>

<script setup lang="ts">
import AppIcon from '@/components/AppIcon.vue'
import { ref, onMounted } from 'vue'
import { useAppStore } from '@/stores/app'

const store = useAppStore()
const newUsername = ref('')
const newPassword = ref('')
const newRole = ref('user')
const showCreate = ref(false)

onMounted(() => {
  store.fetchUsers()
})

async function handleCreate() {
  if (!newUsername.value.trim() || !newPassword.value.trim()) return
  await store.createUser(newUsername.value.trim(), newPassword.value.trim(), newRole.value)
  newUsername.value = ''
  newPassword.value = ''
  newRole.value = 'user'
  showCreate.value = false
}

async function handleDelete(userId: string) {
  await store.deleteUser(userId)
}

async function handleRole(userId: string, role: string) {
  await store.updateUserRole(userId, role)
}
</script>

<style scoped>
.user-panel {
  width: 100%;
  height: 100%;
  background: var(--ui-surface);
  overflow-y: auto;
  padding: 16px;
  font-family: var(--ui-font);
  color: var(--ui-text);
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-bottom: 10px;
  border-bottom: 1px solid var(--ui-border);
  margin-bottom: 16px;
}

.header-left { display: flex; align-items: center; gap: 8px; }
.icon-user { font-size: 16px; }
.panel-title { font-size: 14px; font-weight: 800; letter-spacing: 1px; margin: 0; }

.section { margin-bottom: 16px; }

.section-title {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 1px;
  color: color-mix(in srgb, var(--ui-text) 60%, transparent);
  margin: 0 0 8px;
}

.user-list { display: flex; flex-direction: column; gap: 6px; }

.user-card {
  border: 1px solid var(--ui-border);
  padding: 8px 10px;
}

.user-header {
  display: flex;
  align-items: center;
  gap: 8px;
}

.user-name { font-size: 12px; font-weight: 700; flex: 1; }
.user-role { font-size: 9px; border: 1px solid var(--ui-border-strong); padding: 0 4px; }
.user-active { font-size: 9px; font-weight: 700; }
.user-active.on { color: var(--ui-text); }
.user-active:not(.on) { color: color-mix(in srgb, var(--ui-text) 35%, transparent); }

.user-actions {
  display: flex;
  gap: 4px;
}

.user-action-btn {
  background: transparent;
  border: 1px solid var(--ui-border);
  color: var(--ui-text);
  padding: 1px 4px;
  font-family: var(--ui-font);
  font-size: 8px;
  font-weight: 700;
  cursor: pointer;
}

.user-action-btn:hover {
  background: var(--ui-glass-2);
  color: var(--ui-text);
}

.create-section {
  margin-top: 16px;
  padding-top: 12px;
  border-top: 1px solid var(--ui-border);
}

.bw-input {
  background: var(--ui-surface);
  border: 1px solid var(--ui-border);
  color: var(--ui-text);
  font-family: var(--ui-font);
  font-size: 10px;
  padding: 4px 6px;
  width: 100%;
  margin-bottom: 6px;
}

.bw-input::placeholder {
  color: color-mix(in srgb, var(--ui-text) 35%, transparent);
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
  width: 100%;
}

.bw-btn:hover {
  background: var(--ui-glass-2);
  color: var(--ui-text);
}

.text-muted { color: color-mix(in srgb, var(--ui-text) 50%, transparent) !important; }
</style>
