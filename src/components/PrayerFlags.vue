<template>
  <div class="prayer-flags-container">
    <div class="prayer-flags-string">
      <div
        v-for="(color, index) in flagColors"
        :key="index"
        class="prayer-flag"
        :style="{
          background: color,
          animationDelay: `${index * 0.2}s`,
          transformOrigin: 'top center',
        }"
      />
    </div>
    <div class="prayer-flags-shadow" />
  </div>
</template>

<script setup lang="ts">
import { PRAYER_FLAGS } from '@/types'

const flagColors = [...PRAYER_FLAGS, ...PRAYER_FLAGS]
</script>

<style scoped>
.prayer-flags-container {
  position: relative;
  width: 100%;
  height: 28px;
  overflow: hidden;
  user-select: none;
  pointer-events: none;
  z-index: 1;
}

.prayer-flags-string {
  display: flex;
  align-items: flex-start;
  gap: 3px;
  padding: 0 12px;
  height: 100%;
}

.prayer-flag {
  flex: 1;
  height: 20px;
  border-radius: 0 0 2px 2px;
  border: 2px solid rgba(0, 0, 0, 0.6);
  border-top: none;
  animation: flag-wave 3s ease-in-out infinite;
  opacity: 0.85;
  min-width: 0;
  position: relative;
}

.prayer-flag::after {
  content: '';
  position: absolute;
  bottom: 3px;
  left: 50%;
  transform: translateX(-50%);
  width: 8px;
  height: 8px;
  border-radius: 50%;
  border: 1px solid rgba(0, 0, 0, 0.3);
  background: rgba(0, 0, 0, 0.08);
}

.prayer-flags-shadow {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  height: 8px;
  background: linear-gradient(to bottom, transparent, #0a0a0f);
  z-index: 2;
}

@keyframes flag-wave {
  0%, 100% {
    transform: rotate(-2deg) scaleY(1);
  }
  25% {
    transform: rotate(1.5deg) scaleY(1.04);
  }
  50% {
    transform: rotate(2deg) scaleY(0.96);
  }
  75% {
    transform: rotate(-1deg) scaleY(1.02);
  }
}
</style>