<template>
  <q-tree
    :nodes="treeNodes"
    node-key="id"
    label-key="label"
    children-key="children"
    v-model:expanded="expandedKeys"
    :selected="selected"
    selected-color="brand"
    no-selection-unset
    dense
    class="category-tree"
    @update:selected="$emit('select', $event as number)"
  >
    <template v-slot:default-header="prop">
      <div class="tree-row">
        <span class="tree-label">{{ prop.node.label }}</span>
        <q-tooltip v-if="prop.node.label.length > 18" :delay="500" :offset="[8, 4]">{{ prop.node.label }}</q-tooltip>
        <span class="tree-count">{{ prop.node.count }}</span>
      </div>
    </template>
  </q-tree>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { LocalStorage } from 'quasar'
import type { Category } from '../api'

interface TreeNode {
  id: number
  label: string
  count: number
  children?: TreeNode[]
}

const STORAGE_KEY = 'art-gallery-sidebar-expanded'

const props = defineProps<{
  categories: Category[]
  selected: number | null
}>()

defineEmits<{ select: [id: number] }>()

function toTreeNodes(cats: Category[]): TreeNode[] {
  return [...cats]
    .sort((a, b) => a.name.localeCompare(b.name))
    .map(cat => ({
      id: cat.id,
      label: cat.name,
      count: cat.total_book_count || cat.book_count,
      ...(cat.sub_categories?.length
        ? { children: toTreeNodes(cat.sub_categories) }
        : {}),
    }))
}

const treeNodes = computed(() => toTreeNodes(props.categories))

// 从 localStorage 恢复展开状态，不存在时默认不展开
const savedExpanded = LocalStorage.getItem<number[]>(STORAGE_KEY)
const expandedKeys = ref<number[]>(savedExpanded ?? [])

// 展开状态变化时保存到 localStorage
watch(expandedKeys, (keys) => {
  LocalStorage.set(STORAGE_KEY, keys)
}, { deep: true })
</script>

<style lang="sass" scoped>
.category-tree
  width: 100%
  max-width: 320px

  :deep(.q-tree__node-header)
    display: flex
    align-items: center
    padding: 3px 6px
    border-radius: 6px
    min-height: 30px
    min-width: 0
    width: 100%
    overflow: hidden

  :deep(.q-tree__node)
    overflow: hidden

  :deep(.q-tree__node-header:hover)
    background: color-mix(in srgb, var(--brand) 6%, transparent)

  :deep(.q-tree__node--selected .q-tree__node-header)
    background: color-mix(in srgb, var(--brand) 10%, transparent) !important
    color: var(--brand)
    font-weight: 600

  :deep(.q-tree__arrow)
    font-size: 14px
    color: var(--text-muted)
    flex-shrink: 0

  :deep(.q-tree__node-header-content)
    font-size: 13px
    color: var(--text-secondary)
    flex: 1
    min-width: 0
    overflow: hidden

  :deep(.q-tree__node--selected .q-tree__node-header-content)
    color: var(--brand)

.tree-row
  display: flex
  align-items: center
  gap: 6px
  width: 100%
  min-width: 0

.tree-label
  flex: 1
  min-width: 0
  overflow: hidden
  text-overflow: ellipsis
  white-space: nowrap

.tree-count
  font-size: 10px
  color: var(--text-muted)
  flex-shrink: 0
  font-variant-numeric: tabular-nums
  margin-left: auto

  .q-tree__node--selected &
    color: var(--brand)
    opacity: 0.7
</style>
