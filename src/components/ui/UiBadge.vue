<template>
  <span
    class="ui-badge"
    :class="[
      `ui-badge--${tone}`,
      `ui-badge--${size}`,
      { 'ui-badge--pill': pill, 'ui-badge--outline': outline, 'ui-badge--pulse': pulse },
    ]"
  >
    <span v-if="dot || pulse" class="ui-badge__dot" aria-hidden="true" />
    <AppIcon v-if="icon" :name="icon" :size="size === 'sm' ? 10 : 12" />
    <span class="ui-badge__label"><slot>{{ label }}</slot></span>
  </span>
</template>

<script setup lang="ts">
import AppIcon from '@/components/AppIcon.vue'

withDefaults(
  defineProps<{
    label?: string
    icon?: string
    tone?: 'neutral' | 'accent' | 'success' | 'warning' | 'danger' | 'info'
    size?: 'sm' | 'md'
    pill?: boolean
    outline?: boolean
    pulse?: boolean
    dot?: boolean
  }>(),
  {
    label: '',
    icon: '',
    tone: 'neutral',
    size: 'sm',
    pill: true,
    outline: false,
    pulse: false,
    dot: false,
  }
)
</script>

<style scoped>
.ui-badge {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-family: var(--ui-font-mono);
  font-weight: 700;
  letter-spacing: 0.07em;
  text-transform: uppercase;
  white-space: nowrap;
  border: 1px solid transparent;
  border-radius: var(--ui-radius-full);
  background: var(--ui-surface-2);
  color: var(--ui-text-2);
  transition:
    background-color var(--ui-dur) var(--ui-ease-out),
    border-color var(--ui-dur) var(--ui-ease-out),
    color var(--ui-dur) var(--ui-ease-out);
}

.ui-badge--sm { font-size: 9.5px; padding: 2px 8px; }
.ui-badge--md { font-size: var(--ui-fs-xs); padding: 3px 10px; }

.ui-badge:not(.ui-badge--pill) { border-radius: var(--ui-radius-xs); }

.ui-badge__dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: currentColor;
  flex-shrink: 0;
}

.ui-badge--pulse .ui-badge__dot {
  box-shadow: 0 0 0 0 currentColor;
  animation: ui-badge-pulse 1.8s var(--ui-ease-out) infinite;
}

@keyframes ui-badge-pulse {
  0% { box-shadow: 0 0 0 0 color-mix(in srgb, currentColor 55%, transparent); }
  70% { box-shadow: 0 0 0 6px transparent; }
  100% { box-shadow: 0 0 0 0 transparent; }
}

.ui-badge--accent {
  background: var(--ui-accent-softer);
  color: var(--ui-accent);
  border-color: color-mix(in srgb, var(--ui-accent) 35%, transparent);
}

.ui-badge--success {
  background: color-mix(in srgb, var(--ui-success) 14%, transparent);
  color: var(--ui-success);
  border-color: color-mix(in srgb, var(--ui-success) 35%, transparent);
}

.ui-badge--warning {
  background: color-mix(in srgb, var(--ui-warning) 14%, transparent);
  color: var(--ui-warning);
  border-color: color-mix(in srgb, var(--ui-warning) 35%, transparent);
}

.ui-badge--danger {
  background: color-mix(in srgb, var(--ui-danger) 14%, transparent);
  color: var(--ui-danger);
  border-color: color-mix(in srgb, var(--ui-danger) 38%, transparent);
}

.ui-badge--info {
  background: color-mix(in srgb, var(--ui-info) 14%, transparent);
  color: var(--ui-info);
  border-color: color-mix(in srgb, var(--ui-info) 35%, transparent);
}

.ui-badge--outline {
  background: transparent;
}
</style>
