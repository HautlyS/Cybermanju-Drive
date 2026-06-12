<template>
  <div class="user-panel">
    <div class="panel-header">
      <div class="header-left">
        <span class="icon-user">[!]</span>
        <h2 class="panel-title">USER MANAGEMENT</h2>
      </div>
    </div>

    <div class="section">
      <h3 class="section-title">[USERS] REGISTERED USERS</h3>
      <p class="text-muted" style="margin-bottom:8px;font-size:10px;">PER-FILE USERNAME + PASSWORD AUTH WITH ARGON2 HASHING. ROLE-BASED ACCESS: ADMIN, USER, VIEWER.</p>
      <div v-if="users.length === 0" class="text-muted" style="font-size:10px;">NO USERS REGISTERED</div>
      <div class="user-list">
        <div v-for="user in users" :key="user.id" class="user-card">
          <div class="user-header">
            <span class="user-name">{{ user.username }}</span>
            <span class="user-role">{{ user.role }}</span>
            <span class="user-active" :class="{ on: user.isActive }">{{ user.isActive ? 'ACTIVE' : 'INACTIVE' }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useAppStore } from '@/stores/app'

const store = useAppStore()
const users = computed(() => [] as any[])

onMounted(() => {})
</script>

<style scoped>
.user-panel {
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
.icon-user { font-size: 16px; }
.panel-title { font-size: 14px; font-weight: 800; letter-spacing: 1px; margin: 0; }

.section { margin-bottom: 16px; }

.section-title {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 1px;
  color: rgba(255,255,255,0.6);
  margin: 0 0 8px;
}

.user-list { display: flex; flex-direction: column; gap: 6px; }

.user-card {
  border: 2px solid #FFFFFF;
  padding: 8px 10px;
}

.user-header {
  display: flex;
  align-items: center;
  gap: 8px;
}

.user-name { font-size: 12px; font-weight: 700; flex: 1; }
.user-role { font-size: 9px; border: 1px solid #FFFFFF; padding: 0 4px; }
.user-active { font-size: 9px; font-weight: 700; }
.user-active.on { color: #FFFFFF; }
.user-active:not(.on) { color: rgba(255,255,255,0.3); }

.text-muted { color: rgba(255,255,255,0.5) !important; }
</style>
