<script setup lang="ts">
import type { PrivacyRule } from '@/api/modules/settings/privacy'
import { defineTableColumns, ZIconButton, ZSwitch, ZTable } from '@codex-proxy/ui'
import { GripVertical, Pencil, Trash2 } from '@lucide/vue'
import { usePreferredReducedMotion } from '@vueuse/core'
import { moveArrayElement, useSortable } from '@vueuse/integrations/useSortable'
import { computed, nextTick, shallowRef, useId, useTemplateRef, watch } from 'vue'
import { actions, scopes } from './presets'

const props = defineProps<{ rules: PrivacyRule[], disabled: boolean }>()
const emit = defineEmits<{
  edit: [rule: PrivacyRule]
  toggle: [rule: PrivacyRule, enabled: boolean]
  reorder: [rules: PrivacyRule[]]
  remove: [rule: PrivacyRule]
}>()
const container = useTemplateRef<HTMLDivElement>('container')
const tableBody = shallowRef<HTMLTableSectionElement | null>(null)
const reorderHelpId = useId()
const announcement = shallowRef('')
const preferredMotion = usePreferredReducedMotion()
const animation = computed(() => preferredMotion.value === 'reduce' ? 0 : 200)
const sortingDisabled = computed(() => props.disabled || props.rules.length < 2)
const sortableRules = computed({
  get: () => props.rules,
  set: (rules) => {
    if (!props.disabled)
      emit('reorder', rules)
  },
})
const { option: setSortableOption } = useSortable(tableBody, sortableRules, {
  watchElement: true,
  handle: '[data-rule-handle]',
  draggable: 'tr',
  animation: animation.value,
  disabled: sortingDisabled.value,
  easing: 'cubic-bezier(0.2, 0, 0, 1)',
  ghostClass: 'opacity-40',
  onEnd: (event) => {
    const id = event.item.querySelector<HTMLElement>('[data-rule-handle]')?.dataset.ruleHandle
    if (id && event.oldIndex !== event.newIndex)
      void nextTick(() => focusMovedRule(id))
  },
})

// 表格空态没有 tbody，增删规则后重新绑定排序容器
watch([container, () => props.rules.length], () => {
  tableBody.value = container.value?.querySelector('tbody') ?? null
}, { flush: 'post' })
watch([tableBody, sortingDisabled, animation], async () => {
  // 等待排序实例随 tbody 重建后再同步选项
  await nextTick()
  setSortableOption('disabled', sortingDisabled.value)
  setSortableOption('animation', animation.value)
}, { flush: 'post' })

async function focusMovedRule(id: string) {
  await nextTick()
  const handles = container.value?.querySelectorAll<HTMLButtonElement>('[data-rule-handle]')
  const handle = Array.from(handles ?? []).find(handle => handle.dataset.ruleHandle === id)
  handle?.focus({ preventScroll: true })
  handle?.scrollIntoView({ block: 'nearest' })
  const index = props.rules.findIndex(rule => rule.id === id)
  const rule = props.rules[index]
  if (rule)
    announcement.value = `${rule.name}已移到第 ${index + 1} 条`
}

function moveWithKeyboard(id: string, direction: -1 | 1) {
  const index = props.rules.findIndex(rule => rule.id === id)
  if (sortingDisabled.value || index < 0 || !props.rules[index + direction])
    return
  moveArrayElement(sortableRules, index, index + direction)
  void nextTick(() => focusMovedRule(id))
}

const columns = defineTableColumns<PrivacyRule>([
  { key: 'enabled', label: '启用', kind: 'status', size: 'sm', fixedWidth: true },
  { key: 'name', label: '规则 / 字段', kind: 'identity', size: '3xl' },
  { key: 'scope', label: '作用范围', kind: 'meta', size: 'lg' },
  { key: 'action', label: '处理方式', kind: 'meta', size: 'md' },
  { key: 'actions', label: '', kind: 'actions', size: 'lg', fixedWidth: true },
])
</script>

<template>
  <div ref="container">
    <p :id="reorderHelpId" class="sr-only">
      拖动手柄调整规则顺序，也可聚焦手柄后使用上下方向键移动
    </p>
    <p class="sr-only" role="status" aria-live="polite">
      {{ announcement }}
    </p>
    <ZTable :columns="columns" :data="rules" size="small" class="[&_tbody>tr>td:last-child]:pr-2!" empty-text="暂无规则，添加一条或从预设开始">
      <template #enabled="{ row }">
        <div class="flex items-center justify-center">
          <ZSwitch :disabled="disabled" :model-value="row.enabled" :aria-label="`启用规则：${row.name}`" @update:model-value="emit('toggle', row, $event)" />
        </div>
      </template>
      <template #name="{ row }">
        <div class="grid min-w-0 gap-0.5 py-1.5">
          <span class="truncate font-emphasis text-cp-text">{{ row.name }}</span>
          <span class="truncate font-mono text-cp-xs text-cp-text-quaternary" :title="row.steps.map(step => step.selector).join('、')">{{ row.steps.map(step => step.selector).join('、') }}</span>
        </div>
      </template>
      <template #scope="{ row }">
        {{ [...new Set(row.steps.map(step => scopes.find(item => item.value === step.scope)?.label))].join('、') }}
      </template>
      <template #action="{ row }">
        {{ [...new Set(row.steps.map(step => actions.find(item => item.value === step.action)?.label))].join('、') }}
      </template>
      <template #actions="{ row }">
        <div class="flex items-center justify-end gap-1">
          <ZIconButton size="small" :disabled="disabled" :aria-label="`编辑${row.name}`" @click="emit('edit', row)">
            <Pencil class="size-3.5" />
          </ZIconButton>
          <ZIconButton size="small" :disabled="disabled" :aria-label="`删除${row.name}`" @click="emit('remove', row)">
            <Trash2 class="size-3.5" />
          </ZIconButton>
          <ZIconButton
            size="small"
            :disabled="sortingDisabled"
            :aria-label="`拖动调整${row.name}顺序`"
            :aria-describedby="reorderHelpId"
            :data-rule-handle="row.id"
            class="touch-none cursor-grab active:cursor-grabbing disabled:cursor-not-allowed"
            @keydown.up.stop.prevent="moveWithKeyboard(row.id, -1)"
            @keydown.down.stop.prevent="moveWithKeyboard(row.id, 1)"
          >
            <GripVertical class="size-3.5" aria-hidden="true" />
          </ZIconButton>
        </div>
      </template>
    </ZTable>
  </div>
</template>
