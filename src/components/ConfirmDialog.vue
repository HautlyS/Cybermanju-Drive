<template>
  <Teleport to="body">
    <div
      v-if="visible"
      class="confirm-overlay"
      @click.self="handleCancel"
      role="dialog"
      aria-modal="true"
      :aria-label="title"
    >
      <div ref="dialogRef" class="confirm-modal">
        <div class="confirm-header">{{ title }}</div>
        <div class="confirm-body">{{ message }}</div>
        <div class="confirm-actions">
          <button ref="cancelBtnRef" class="confirm-btn cancel" @click="handleCancel">{{ cancelText }}</button>
          <button ref="confirmBtnRef" class="confirm-btn ok" @click="handleConfirm">{{ confirmText }}</button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, toRef } from 'vue'
import { useFocusTrap } from '@/composables/useFocusTrap'

const props = withDefaults(defineProps<{
  visible: boolean
  title?: string
  message?: string
  confirmText?: string
  cancelText?: string
}>(), {
  title: 'CONFIRM',
  message: 'ARE YOU SURE?',
  confirmText: '[YES]',
  cancelText: '[CANCEL]',
})

const emit = defineEmits<{
  confirm: []
  cancel: []
  'update:visible': [value: boolean]
}>()

const dialogRef = ref<HTMLElement | null>(null)
const cancelBtnRef = ref<HTMLElement | null>(null)
const confirmBtnRef = ref<HTMLElement | null>(null)

useFocusTrap(dialogRef, toRef(props, 'visible'))

function handleConfirm() { emit('confirm'); emit('update:visible', false) }
function handleCancel() { emit('cancel'); emit('update:visible', false) }
</script>

<style scoped>
.confirm-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.85);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10002;
}

.confirm-modal {
  background: #FFFFFF;
  border: 2px solid #000000;
  box-shadow: 4px 4px 0 #000000;
  padding: 20px;
  max-width: 360px;
  width: 90%;
  font-family: 'Courier New', monospace;
}

.confirm-header {
  font-size: 12px;
  font-weight: 800;
  color: #000000;
  margin-bottom: 10px;
  letter-spacing: 1px;
}

.confirm-body {
  font-size: 11px;
  color: rgba(0, 0, 0, 0.7);
  margin-bottom: 16px;
  line-height: 1.4;
}

.confirm-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.confirm-btn {
  padding: 6px 14px;
  font-family: 'Courier New', monospace;
  font-size: 10px;
  font-weight: 700;
  cursor: pointer;
  border: 2px solid #000000;
}

.confirm-btn.ok {
  background: #000000;
  color: #FFFFFF;
}

.confirm-btn.ok:hover {
  background: #FFFFFF;
  color: #000000;
}

.confirm-btn.cancel {
  background: #FFFFFF;
  color: #000000;
}

.confirm-btn.cancel:hover {
  background: #000000;
  color: #FFFFFF;
}
</style>
