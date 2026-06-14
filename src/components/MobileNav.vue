<template>
  <nav class="mobile-nav" role="navigation" aria-label="MOBILE NAVIGATION">
    <button class="mn-btn" :class="{ active: store.currentPanel === 'files' }" @click="store.currentPanel = 'files'" aria-label="FILES">[#]</button>
    <button class="mn-btn" :class="{ active: store.currentPanel === 'search' }" @click="store.currentPanel = 'search'; store.searchQuery ? null : null" aria-label="SEARCH">[S]</button>
    <button class="mn-btn" :class="{ active: store.currentPanel === 'favorites' }" @click="store.currentPanel = 'favorites'" aria-label="FAVORITES">[*]</button>
    <button class="mn-btn" :class="{ active: store.currentPanel === 'recent' }" @click="store.currentPanel = 'recent'" aria-label="RECENT">[T]</button>
    <button class="mn-btn" :class="{ active: store.currentPanel === 'settings' }" @click="store.currentPanel = 'settings'" aria-label="SETTINGS">[@]</button>
  </nav>
</template>

<script setup lang="ts">
import { useAppStore } from '@/stores/app'
const store = useAppStore()
</script>

<style scoped>
.mobile-nav {
  display: none;
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  height: 48px;
  background: #000;
  border-top: 2px solid #FFFFFF;
  z-index: 100;
  font-family: 'Courier New', monospace;
}

.mn-btn {
  flex: 1;
  background: transparent;
  border: none;
  border-right: 1px solid rgba(255,255,255,0.2);
  color: rgba(255,255,255,0.4);
  font-size: 14px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 44px;
}

.mn-btn:last-child {
  border-right: none;
}

.mn-btn.active {
  color: #000;
  background: #FFFFFF;
}

@media (max-width: 768px) {
  .mobile-nav {
    display: flex;
  }
}
</style>
