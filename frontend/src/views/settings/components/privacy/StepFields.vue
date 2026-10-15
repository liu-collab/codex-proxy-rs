<script setup lang="ts">
import type { PrivacyStep } from '@/api/modules/settings/privacy'
import { ZCheckbox, ZForm, ZFormItem, ZIconButton, ZInput, ZPopover, ZSelect, ZTextarea } from '@codex-proxy/ui'
import { CircleHelp } from '@lucide/vue'
import { computed, useId, watch } from 'vue'
import { actions, fieldsByScope, scopes } from './presets'

const step = defineModel<PrivacyStep>({ required: true })
const fixedValue = defineModel<string>('fixedValue', { required: true })
const fieldPresetId = useId()
const usesPattern = computed(() => ['regex_replace', 'rename_key'].includes(step.value.action) || (step.value.action === 'remove_field' && step.value.pattern !== null))
const pattern = computed({ get: () => step.value.pattern ?? '', set: (value: string) => {
  step.value.pattern = value
} })
const conditional = computed({ get: () => step.value.pattern !== null, set: (value: boolean) => {
  step.value.pattern = value ? '' : null
} })
const fields = computed(() => fieldsByScope[step.value.scope] ?? [])
const fieldChoice = computed({ get: () => fields.value.some(field => field.value === step.value.selector) ? step.value.selector : '', set: (value: string) => {
  step.value.selector = value
} })
const selectorPlaceholder = computed(() => ({
  turn_metadata: '例如：$.workspaces.*.associated_remote_urls.*',
  desktop_git_context: '例如：$.remotes[*].fetchUrl',
  environment_text: '填写 $，匹配整段环境文本',
  request_body: '例如：$.metadata.workspace',
  request_header: '例如：x-client-name',
})[step.value.scope])

watch(() => step.value.action, (action) => {
  if (action === 'set_value')
    step.value.pattern = null
  else if (action !== 'remove_field' && step.value.pattern === null)
    step.value.pattern = ''
})
</script>

<template>
  <ZForm class="sm:grid-cols-2">
    <ZFormItem class="md:[&>div:first-child]:min-h-8" label="作用范围">
      <ZSelect v-model="step.scope" class="w-full" :options="scopes" aria-label="作用范围" />
    </ZFormItem>
    <ZFormItem class="md:[&>div:first-child]:min-h-8" label="处理方式">
      <ZSelect v-model="step.action" class="w-full" :options="actions" aria-label="处理方式" />
    </ZFormItem>
    <ZFormItem class="sm:col-span-2" :label="step.scope === 'request_header' ? '请求头名称' : '字段路径'" required>
      <template #label-extra>
        <ZPopover trigger="hover-click">
          <template #reference="{ open: helpOpen }">
            <ZIconButton size="small" aria-label="字段路径语法" :title="undefined" class="-my-2 hover:bg-transparent! active:bg-transparent!" :aria-expanded="helpOpen">
              <CircleHelp class="size-3.5" />
            </ZIconButton>
          </template>
          <div class="max-w-72 p-3 text-cp-sm text-cp-text-secondary">
            支持对象成员、双引号键名、数组索引和 * 通配，例如 $["workspaces"]["/home/alex/demo"]<br>环境文本使用 $，请求头直接填写名称
          </div>
        </ZPopover>
      </template>
      <div class="flex flex-wrap items-center gap-2 sm:flex-nowrap">
        <ZInput v-model="step.selector" :placeholder="selectorPlaceholder" aria-label="字段路径" maxlength="1024" class="min-w-0 flex-1 font-mono" :class="fields.length ? 'basis-full sm:basis-auto' : undefined" spellcheck="false" />
        <ZSelect v-if="fields.length" :id="fieldPresetId" v-model="fieldChoice" class="min-w-0 flex-1 sm:w-40 sm:flex-none" :options="fields" aria-label="常用字段" />
      </div>
    </ZFormItem>
    <ZCheckbox v-if="step.action === 'remove_field'" v-model="conditional" class="sm:col-span-2" label="仅删除值匹配正则的字段或数组项" />
    <ZFormItem v-if="usesPattern" :class="step.action === 'remove_field' ? 'sm:col-span-2' : undefined" label="匹配表达式">
      <ZInput v-model="pattern" :placeholder="step.action === 'rename_key' ? '例如：^/home/[^/]+/' : '例如：private-org'" aria-label="匹配表达式" maxlength="4096" spellcheck="false" class="font-mono" />
    </ZFormItem>
    <ZFormItem v-if="step.action === 'regex_replace' || step.action === 'rename_key'" label="替换内容">
      <template #label-extra>
        <ZPopover trigger="hover-click">
          <template #reference="{ open: helpOpen }">
            <ZIconButton size="small" aria-label="正则语法说明" :title="undefined" class="-my-2 hover:bg-transparent! active:bg-transparent!" :aria-expanded="helpOpen">
              <CircleHelp class="size-3.5" />
            </ZIconButton>
          </template>
          <div class="max-w-72 p-3 text-cp-sm text-cp-text-secondary">
            ${1} 或 ${name} 引用捕获组，$$ 表示字面 $<br>使用 Rust regex 语法，不支持前后顾和模式中的反向引用
          </div>
        </ZPopover>
      </template>
      <ZInput v-model="step.replacement" :placeholder="step.action === 'rename_key' ? '例如：/home/user/' : '例如：team，留空则清除匹配内容'" aria-label="替换内容" maxlength="4096" spellcheck="false" class="font-mono" />
    </ZFormItem>
    <ZFormItem v-if="step.action === 'set_value'" class="sm:col-span-2" label="固定值（JSON）">
      <ZTextarea v-model="fixedValue" placeholder="例如：&quot;匿名&quot;、null 或 {}" :rows="2" resize="none" maxlength="4096" aria-label="固定值 JSON" class="font-mono" />
    </ZFormItem>
    <div v-if="usesPattern" class="flex flex-wrap gap-x-4 gap-y-2 sm:col-span-2">
      <ZCheckbox v-if="step.action !== 'remove_field'" v-model="step.replaceAll" label="替换全部匹配" />
      <ZCheckbox v-model="step.caseInsensitive" label="忽略大小写" />
      <ZCheckbox v-model="step.multiLine" label="多行匹配" />
    </div>
  </ZForm>
</template>
