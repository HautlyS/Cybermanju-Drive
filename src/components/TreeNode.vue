<template>
  <div class="tree-node">
    <div
      class="tree-node-row"
      :style="{ paddingLeft: (depth * 16 + 4) + 'px' }"
      :class="{ selected: isSelected }"
      @click="$emit('select', node.id)"
      @dblclick="toggle"
    >
      <span v-if="hasChildren" class="tree-arrow" @click.stop="toggle">
        {{ expanded ? 'v' : '>' }}
      </span>
      <span v-else class="tree-arrow-placeholder" />
      <span class="tree-name truncate" :title="node.name">{{ node.name }}</span>
    </div>
    <div v-if="hasChildren && expanded" class="tree-children">
      <TreeNode
        v-for="child in node.children || []"
        :key="child.id"
        :node="child"
        :depth="depth + 1"
        @select="(id: string) => $emit('select', id)"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useAppStore } from '@/stores/app'
import type { FileNode } from '@/types'

const props = defineProps<{
  node: FileNode
  depth: number
}>()

defineEmits<{ select: [id: string] }>()

const store = useAppStore()

const expanded = ref(props.depth < 1)

const hasChildren = computed(() => props.node.children && props.node.children.length > 0)

const isSelected = computed(() => store.selectedFileId === props.node.id)

function toggle() {
  expanded.value = !expanded.value
}
</script>

<style scoped>
.tree-node-row {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 3px 8px;
  cursor: pointer;
  font-size: 11px;
  color: #FFFFFF;
}

.tree-node-row:hover {
  background: rgba(255,255,255,0.08);
}

.tree-node-row.selected {
  background: rgba(255,255,255,0.15);
  font-weight: 700;
}

.tree-arrow {
  font-family: 'Courier New', monospace;
  font-size: 8px;
  width: 12px;
  flex-shrink: 0;
  text-align: center;
  color: rgba(255,255,255,0.5);
}

.tree-arrow-placeholder {
  width: 12px;
  flex-shrink: 0;
}

.tree-name {
  flex: 1;
  font-size: 10px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.truncate { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
