<script setup lang="ts">
import type { CodexPrivacyPolicy, PrivacyRule } from '@/api/modules/settings/privacy'
import { ZButton, ZCard, ZIconButton, ZMenuItem, ZPopover, ZSelect, ZSwitch } from '@codex-proxy/ui'
import { ChevronDown, CircleHelp, Play, Plus } from '@lucide/vue'
import { computed, shallowRef } from 'vue'
import PolicyPreview from './PolicyPreview.vue'
import { createPreset, newRule, presets } from './presets'
import RuleEditor from './RuleEditor.vue'
import RulesTable from './RulesTable.vue'

defineProps<{ disabled: boolean }>()
const model = defineModel<CodexPrivacyPolicy>({ required: true })
const editorOpen = shallowRef(false)
const selected = shallowRef<PrivacyRule | null>(null)
const isNew = shallowRef(false)
const presetOpen = shallowRef(false)
const testOpen = shallowRef(false)
const stepCount = computed(() => model.value.rules.reduce((count, rule) => count + rule.steps.length, 0))
const availableSteps = computed(() => 32 - stepCount.value + (isNew.value ? 0 : selected.value?.steps.length ?? 0))
const failureOptions: { value: CodexPrivacyPolicy['onError'], label: string }[] = [{ value: 'skip_rule', label: '跳过当前规则' }, { value: 'reject_request', label: '阻止请求发送' }]

function edit(rule: PrivacyRule, create = false) {
  selected.value = rule
  isNew.value = create
  editorOpen.value = true
  presetOpen.value = false
}
function apply(rule: PrivacyRule) {
  model.value = { ...model.value, rules: isNew.value ? [...model.value.rules, rule] : model.value.rules.map(item => item.id === rule.id ? rule : item) }
}
function add(key?: string) {
  edit(key ? createPreset(key) : newRule(), true)
}
function toggle(rule: PrivacyRule, enabled: boolean) {
  model.value = { ...model.value, rules: model.value.rules.map(item => item.id === rule.id ? { ...item, enabled } : item) }
}
function remove(rule: PrivacyRule) {
  model.value = { ...model.value, rules: model.value.rules.filter(item => item.id !== rule.id) }
}
</script>

<template>
  <ZCard id="privacy-policy" title="隐私策略" description="按规则统一或移除 Codex 请求中的隐私信息">
    <template #actions>
      <ZSwitch v-model="model.enabled" aria-label="启用请求规则" active-text="启用" inactive-text="关闭" inline-prompt :width="56" :disabled="disabled" />
    </template>
    <template #body>
      <div class="grid gap-3">
        <div class="flex flex-wrap items-center justify-between gap-2">
          <div class="flex flex-wrap items-center gap-2">
            <ZButton size="small" :disabled="disabled || stepCount >= 32" @click="add()">
              <template #icon>
                <Plus class="size-3.5" />
              </template>添加规则
            </ZButton>
            <ZPopover v-model="presetOpen" placement="bottom-start">
              <template #reference>
                <ZButton size="small" variant="text" :disabled="disabled || stepCount >= 32">
                  从预设添加<ChevronDown class="ml-1 size-3" />
                </ZButton>
              </template>
              <div class="w-48 p-1.5">
                <ZMenuItem v-for="preset in presets" :key="preset.key" :disabled="stepCount + preset.steps.length > 32" @click="add(preset.key)">
                  {{ preset.name }}
                </ZMenuItem>
              </div>
            </ZPopover>
          </div>
          <ZButton size="small" variant="text" :disabled="disabled" @click="testOpen = true">
            <template #icon>
              <Play class="size-3.5" />
            </template>测试规则
          </ZButton>
        </div>
        <RulesTable :rules="model.rules" :disabled="disabled" @edit="edit($event)" @toggle="toggle" @remove="remove" @reorder="model = { ...model, rules: $event }" />
        <div v-if="model.rules.length" class="flex flex-wrap items-center justify-between gap-x-4 gap-y-2">
          <span class="text-cp-xs text-cp-text-quaternary">按列表顺序执行 · {{ model.rules.length }} 条规则 · {{ stepCount }} / 32 个步骤</span>
          <div class="flex items-center gap-2">
            <span class="text-cp-xs text-cp-text-secondary">执行失败</span>
            <ZSelect v-model="model.onError" size="small" class="w-34" :options="failureOptions" aria-label="规则执行失败" :disabled="disabled" />
            <ZPopover trigger="hover-click" placement="top-end">
              <template #reference="{ open: helpOpen }">
                <ZIconButton size="small" aria-label="规则执行说明" :title="undefined" class="hover:bg-transparent! active:bg-transparent!" :aria-expanded="helpOpen">
                  <CircleHelp class="size-3.5" />
                </ZIconButton>
              </template>
              <div class="max-w-72 space-y-2 p-3 text-cp-sm text-cp-text-secondary">
                <p class="m-0">
                  跳过时撤销本条规则所有步骤的修改并继续执行，未完成本条脱敏
                </p><p class="m-0">
                  允许改写选定字段，认证、会话与工具行为的影响由配置者负责
                </p>
              </div>
            </ZPopover>
          </div>
        </div>
      </div>
    </template>
  </ZCard>
  <RuleEditor v-model="editorOpen" :rule="selected" :max-steps="availableSteps" :is-new="isNew" :disabled="disabled" @apply="apply" />
  <PolicyPreview v-model="testOpen" :policy="model" />
</template>
