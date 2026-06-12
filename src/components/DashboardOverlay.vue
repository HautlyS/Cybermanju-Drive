<template>
  <div class="dash-overlay" @click.self="$emit('close')">
    <div class="dash-content">
      <div class="dash-header">
        <h2>WEB DASHBOARD</h2>
        <button class="close-btn" @click="$emit('close')">X</button>
      </div>
      <div class="dash-body">
        <p class="text-muted">WEB DASHBOARD ON PORT 3456. ACCESS FROM ANY DEVICE.</p>
        <div class="dash-url">
          <span class="mono">HTTP://LOCALHOST:3456</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
defineEmits<{ close: [] }>()
</script>

<style scoped>
.dash-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0,0,0,0.9);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.dash-content {
  background: #000;
  border: 2px solid #FFFFFF;
  padding: 24px;
  max-width: 400px;
  width: 90%;
  font-family: 'Courier New', monospace;
  color: #FFFFFF;
}

.dash-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
  padding-bottom: 8px;
  border-bottom: 2px solid #FFFFFF;
}

.dash-header h2 {
  font-size: 14px;
  font-weight: 800;
  letter-spacing: 1px;
  margin: 0;
}

.close-btn {
  background: none;
  border: 2px solid #FFFFFF;
  color: #FFFFFF;
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  font-family: 'Courier New', monospace;
  font-weight: 700;
  font-size: 10px;
}

.close-btn:hover { background: #FFFFFF; color: #000; }

.dash-body {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.dash-body p { font-size: 11px; margin: 0; }

.dash-url {
  border: 2px solid #FFFFFF;
  padding: 10px;
  text-align: center;
}

.mono { font-family: 'Courier New', monospace; font-size: 12px; font-weight: 700; }
.text-muted { color: rgba(255,255,255,0.5) !important; }
</style>
