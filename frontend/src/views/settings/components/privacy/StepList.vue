<script setup lang="ts">
import type { PrivacyStep } from '@/api/modules/settings/privacy'
import { ZIconButton, ZPopover, ZScrollbar } from '@codex-proxy/ui'
import { CircleHelp, GripVertical, Plus, Trash2 } from '@lucide/vue'
import { usePreferredReducedMotion } from '@vueuse/core'
import { moveArrayElement, useSortable } from '@vueuse/integrations/useSortable'
import { computed, nextTick, shallowRef, useId, useTemplateRef, watch } from 'vue'
import { actions, fieldsByScope, scopes } from './presets'

const props = defineProps<{ steps: PrivacyStep[], fixedValues: string[], canAdd: boolean }>()
const emit = defineEmits<{ add: [], remove: [index: number], reorder: [steps: PrivacyStep[]] }>()
const selected = defineModel<number>({ required: true })
const list = useTemplateRef<HTMLOListElement>('list')
const scrollbar = useTemplateRef('scrollbar')
const items = computed(() => props.steps.map((step, index) => {
  const field = fieldsByScope[step.scope]?.find(field => field.value && field.value === step.selector)?.label
    || (step.selector === '$' && step.scope !== 'request_header' ? (step.scope === 'environment_text' ? '整段环境文本' : '完整内容') : step.selector.trim() || '未选择字段')
  const scope = scopes.find(scope => scope.value === step.scope)?.label
  const action = actions.find(action => action.value === step.action)?.label
  const effect = step.action === 'set_value'
    ? `→ ${props.fixedValues[index] ?? '""'}`
    : step.action === 'remove_field'
      ? (step.pattern === null ? '删除匹配的字段或数组项' : `匹配后删除：${step.pattern || '未填写表达式'}`)
      : `→ ${step.replacement || '清空匹配内容'}`
  return { step, field, scope, action, effect }
}))

const stepKeys = new WeakMap<PrivacyStep, number>()
let nextKey = 0
function stepKey(step: PrivacyStep) {
  // 步骤没有持久化 ID，按草稿对象提供稳定的 DOM 身份，不改动接口合同
  if (!stepKeys.has(step))
    stepKeys.set(step, nextKey++)
  return stepKeys.get(step)!
}

const reorderHelpId = useId()
const announcement = shallowRef('')
const preferredMotion = usePreferredReducedMotion()
const animation = computed(() => preferredMotion.value === 'reduce' ? 0 : 200)
const sortingDisabled = computed(() => props.steps.length < 2)
const sortableSteps = computed({
  get: () => props.steps,
  set: steps => emit('reorder', steps),
})
const { option: setSortableOption } = useSortable(list, sortableSteps, {
  watchElement: true,
  handle: '[data-step-handle]',
  draggable: 'li',
  animation: animation.value,
  disabled: sortingDisabled.value,
  easing: 'cubic-bezier(0.2, 0, 0, 1)',
  ghostClass: 'opacity-40',
  onEnd: (event) => {
    if (event.newIndex !== undefined && event.oldIndex !== event.newIndex)
      void nextTick(() => focusMovedStep(event.newIndex!))
  },
})
watch([list, sortingDisabled, animation], async () => {
  await nextTick()
  setSortableOption('disabled', sortingDisabled.value)
  setSortableOption('animation', animation.value)
}, { flush: 'post' })

function revealStep(index: number) {
  const container = scrollbar.value?.wrapRef
  const item = list.value?.children[index] as HTMLElement | undefined
  if (!container || !item)
    return
  // 仅滚动步骤列表，保持编辑表单和弹窗的位置稳定
  const top = item.getBoundingClientRect().top - container.getBoundingClientRect().top + container.scrollTop
  if (top < container.scrollTop)
    container.scrollTop = top
  else if (top + item.offsetHeight > container.scrollTop + container.clientHeight)
    container.scrollTop = top + item.offsetHeight - container.clientHeight
}
watch(selected, () => revealStep(selected.value), { flush: 'post' })

async function focusMovedStep(index: number) {
  await nextTick()
  list.value?.children[index]?.querySelector<HTMLButtonElement>('[data-step-handle]')?.focus({ preventScroll: true })
  revealStep(index)
  announcement.value = `步骤已移到第 ${index + 1} 位`
}
function moveWithKeyboard(index: number, offset: -1 | 1) {
  const target = index + offset
  if (!props.steps[target])
    return
  moveArrayElement(sortableSteps, index, target)
  void nextTick(() => focusMovedStep(target))
}
</script>

<template>
  <section class="grid min-h-0 min-w-0 grid-rows-[auto_minmax(0,1fr)]" aria-label="处理步骤">
    <div class="mb-1 flex min-h-8 items-center justify-between gap-2 pr-5 pl-1">
      <div class="flex items-center gap-1">
        <h3 class="m-0 text-cp leading-none font-medium text-cp-text-secondary">
          处理步骤
        </h3>
        <ZPopover trigger="hover-click">
          <template #reference="{ open: helpOpen }">
            <ZIconButton size="small" aria-label="处理步骤说明" :title="undefined" class="size-5! hover:bg-transparent! active:bg-transparent!" :aria-expanded="helpOpen">
              <CircleHelp class="size-3.5" />
            </ZIconButton>
          </template>
          <div class="max-w-72 space-y-2 p-3 text-cp-sm text-cp-text-secondary">
            <p class="m-0">
              拖动手柄调整执行顺序，也可聚焦手柄后使用上下方向键移动
            </p>
            <p class="m-0">
              每一步可分别设置作用范围、字段和处理方式，后一步基于前一步的结果处理
            </p>
            <p class="m-0">
              所有步骤共用规则开关，任一步骤失败会撤销本条规则的全部修改，再按失败策略跳过或阻止请求
            </p>
          </div>
        </ZPopover>
      </div>
      <ZIconButton size="small" aria-label="添加步骤" class="bg-cp-fill-quaternary! hover:bg-cp-fill-tertiary!" :disabled="!canAdd" @click="emit('add')">
        <Plus class="size-3.5" />
      </ZIconButton>
    </div>
    <p :id="reorderHelpId" class="sr-only">
      拖动手柄调整步骤顺序，也可聚焦手柄后使用上下方向键移动
    </p>
    <p class="sr-only" role="status" aria-live="polite">
      {{ announcement }}
    </p>
    <div class="relative h-52 min-h-0 md:h-auto">
      <div class="absolute inset-y-0 right-1 left-0">
        <ZScrollbar ref="scrollbar">
          <ol ref="list" class="m-0 grid list-none gap-1 p-1 pr-4">
            <li
              v-for="(item, index) in items" :key="stepKey(item.step)"
              class="relative min-w-0 rounded-cp transition-colors motion-reduce:transition-none"
              :class="selected === index ? 'bg-cp-primary-container text-cp-primary-on-container hover:bg-cp-primary-container-hover' : 'bg-cp-fill-quaternary text-cp-text hover:bg-cp-bg-text-hover'"
            >
              <ZIconButton
                size="small" :disabled="sortingDisabled"
                :aria-label="`拖动步骤 ${index + 1}：${item.field}`" :aria-describedby="reorderHelpId"
                data-step-handle class="absolute top-1.5 left-1.5 size-6! touch-none cursor-grab p-0 active:cursor-grabbing disabled:cursor-not-allowed"
                @keydown.up.stop.prevent="moveWithKeyboard(index, -1)"
                @keydown.down.stop.prevent="moveWithKeyboard(index, 1)"
              >
                <GripVertical class="size-3.5" />
              </ZIconButton>
              <button
                type="button"
                class="grid w-full min-w-0 cursor-pointer gap-0.5 rounded-cp border-0 bg-transparent py-2 pr-2 pl-9 text-left outline-none focus-visible:ring-2 focus-visible:ring-cp-control-outline"
                :aria-current="selected === index ? 'step' : undefined"
                :aria-label="`步骤 ${index + 1}：${item.field}，${item.scope}，${item.action}，${item.effect}`"
                @click="selected = index"
              >
                <span class="truncate pr-7 text-cp-sm leading-5 font-emphasis" :title="item.field">{{ item.field }}</span>
                <span class="truncate text-cp-xs leading-4" :class="selected === index ? 'text-cp-primary-on-container' : 'text-cp-text-secondary'" :title="`${item.scope} · ${item.action}`">{{ item.scope }} · {{ item.action }}</span>
                <span class="truncate font-mono text-cp-xs leading-4" :class="selected === index ? 'text-cp-primary-on-container' : 'text-cp-text-tertiary'" :title="item.effect">{{ item.effect }}</span>
              </button>
              <ZIconButton size="small" class="absolute top-1.5 right-1.5 size-6! p-0" :disabled="steps.length === 1" :aria-label="`删除步骤 ${index + 1}：${item.field}`" @click="emit('remove', index)">
                <Trash2 class="size-3.5" />
              </ZIconButton>
            </li>
          </ol>
        </ZScrollbar>
      </div>
    </div>
  </section>
</template>
