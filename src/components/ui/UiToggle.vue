<template>
  <button
    class="ui-toggle"
    :class="{ 'ui-toggle--on': modelValue, 'ui-toggle--sm': size === 'sm' }"
    type="button"
    role="switch"
    :aria-checked="modelValue"
    :disabled="disabled"
    :aria-label="label || undefined"
    @click="toggle"
  >
    <span class="ui-toggle__track">
      <span class="ui-toggle__thumb">
        <AppIcon v-if="modelValue" name="solar:check-bold" :size="9" />
      </span>
    </span>
    <span v-if="label || $slots.default" class="ui-toggle__label">
      <slot>{{ label }}</slot>
    </span>
  </button>
</template>

<script setup lang="ts">
import AppIcon from '@/components/AppIcon.vue'

const props = withDefaults(
  defineProps<{
    modelValue?: boolean
    label?: string
    disabled?: boolean
    size?: 'sm' | 'md'
  }>(),
  { modelValue: false, label: '', disabled: false, size: 'md' }
)

const emit = defineEmits<{ 'update:modelValue': [value: boolean] }>()

function toggle() {
  emit('update:modelValue', !props.modelValue)
}
</script>

<style scoped>
.ui-toggle {
  display: inline-flex;
  align-items: center;
  gap: 9px;
  background: none;
  border: none;
  padding: 0;
  cursor: pointer;
  user-select: none;
  color: var(--ui-text-2);
  font-family: var(--ui-font);
  font-size: var(--ui-fs-sm);
}

.ui-toggle:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.ui-toggle:focus-visible .ui-toggle__track {
  outline: 2px solid color-mix(in srgb, var(--ui-accent) 75%, transparent);
  outline-offset: 2px;
}

.ui-toggle__track {
  position: relative;
  display: flex;
  align-items: center;
  width: 40px;
  height: 22px;
  padding: 2px;
  border-radius: var(--ui-radius-full);
  background: var(--ui-surface-3);
  border: 1px solid var(--ui-border-strong);
  transition:
    background-color var(--ui-dur) var(--ui-ease-out),
    border-color var(--ui-dur) var(--ui-ease-out),
    box-shadow var(--ui-dur) var(--ui-ease-out);
}

.ui-toggle--sm .ui-toggle__track {
  width: 32px;
  height: 18px;
}

.ui-toggle__thumb {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: linear-gradient(180deg, #ffffff, #d8dee2);
  color: var(--ui-on-accent);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.35);
  transform: translateX(0);
  transition:
    transform var(--ui-dur) var(--ui-ease-spring),
    background-color var(--ui-dur) var(--ui-ease-out);
}

.ui-toggle--sm .ui-toggle__thumb {
  width: 13px;
  height: 13px;
}

.ui-toggle--on .ui-toggle__track {
  background: linear-gradient(
    180deg,
    color-mix(in srgb, var(--ui-accent) 85%, white),
    var(--ui-accent)
  );
  border-color: color-mix(in srgb, var(--ui-accent) 70%, transparent);
  box-shadow: var(--ui-glow-soft);
}

.ui-toggle--on .ui-toggle__thumb {
  transform: translateX(18px);
}

.ui-toggle--sm.ui-toggle--on .ui-toggle__thumb {
  transform: translateX(14px);
}

.ui-toggle--on .ui-toggle__label {
  color: var(--ui-text);
}

.ui-toggle__label {
  transition: color var(--ui-dur) var(--ui-ease-out);
}
</style>
