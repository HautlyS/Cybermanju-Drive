<template>
  <span
    class="ui-chip"
    :class="{ 'ui-chip--active': active, 'ui-chip--square': square }"
    role="button"
    tabindex="0"
    :aria-pressed="active"
    @click="emit('click', $event)"
    @keydown.enter.prevent="emit('click', $event as unknown as MouseEvent)"
    @keydown.space.prevent="emit('click', $event as unknown as MouseEvent)"
  >
    <AppIcon v-if="icon" :name="icon" :size="13" class="ui-chip__icon" />
    <span class="ui-chip__label"><slot>{{ label }}</slot></span>
    <span v-if="count !== undefined" class="ui-chip__count">{{ count }}</span>
    <button
      v-if="removable"
      class="ui-chip__remove"
      type="button"
      :aria-label="`Remove ${label}`"
      @click.stop="emit('remove', $event)"
    >
      <AppIcon name="solar:close-bold" :size="11" />
    </button>
  </span>
</template>

<script setup lang="ts">
import AppIcon from '@/components/AppIcon.vue'

withDefaults(
  defineProps<{
    label?: string
    icon?: string
    count?: number
    active?: boolean
    removable?: boolean
    square?: boolean
  }>(),
  { label: '', icon: '', active: false, removable: false, square: false }
)

const emit = defineEmits<{ click: [event: MouseEvent]; remove: [event: MouseEvent] }>()
</script>

<style scoped>
.ui-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  min-height: calc(var(--ui-control-h) * 0.78);
  padding: 0 11px;
  background: var(--ui-glass);
  border: 1px solid var(--ui-border);
  border-radius: var(--ui-radius-full);
  color: var(--ui-text-2);
  font-family: var(--ui-font);
  font-size: var(--ui-fs-xs);
  font-weight: 600;
  letter-spacing: 0.03em;
  cursor: pointer;
  user-select: none;
  backdrop-filter: blur(var(--ui-blur)) saturate(var(--ui-saturate));
  -webkit-backdrop-filter: blur(var(--ui-blur)) saturate(var(--ui-saturate));
  transition:
    background-color var(--ui-dur-fast) var(--ui-ease-out),
    border-color var(--ui-dur-fast) var(--ui-ease-out),
    color var(--ui-dur-fast) var(--ui-ease-out),
    box-shadow var(--ui-dur) var(--ui-ease-out),
    transform var(--ui-dur-fast) var(--ui-ease-spring);
}

.ui-chip:hover {
  border-color: var(--ui-border-hover);
  color: var(--ui-text);
  transform: translateY(-1px);
}

.ui-chip:active {
  transform: translateY(0) scale(0.97);
}

.ui-chip:focus-visible {
  outline: 2px solid color-mix(in srgb, var(--ui-accent) 75%, transparent);
  outline-offset: 2px;
}

.ui-chip--square {
  border-radius: var(--ui-radius-sm);
}

.ui-chip--active {
  background: var(--ui-accent-softer);
  border-color: color-mix(in srgb, var(--ui-accent) 55%, transparent);
  color: var(--ui-accent);
  box-shadow: var(--ui-glow-soft);
}

.ui-chip__count {
  font-family: var(--ui-font-mono);
  font-size: 9.5px;
  font-weight: 700;
  padding: 1px 5px;
  border-radius: var(--ui-radius-full);
  background: color-mix(in srgb, var(--ui-text) 12%, transparent);
  color: var(--ui-text-3);
}

.ui-chip--active .ui-chip__count {
  background: color-mix(in srgb, var(--ui-accent) 22%, transparent);
  color: var(--ui-accent);
}

.ui-chip__remove {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 15px;
  height: 15px;
  margin-right: -4px;
  border-radius: 50%;
  color: var(--ui-text-3);
  background: transparent;
  transition:
    background-color var(--ui-dur-fast) var(--ui-ease-out),
    color var(--ui-dur-fast) var(--ui-ease-out);
}

.ui-chip__remove:hover {
  background: color-mix(in srgb, var(--ui-danger) 20%, transparent);
  color: var(--ui-danger);
}
</style>
