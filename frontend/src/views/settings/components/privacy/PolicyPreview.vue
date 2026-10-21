<script setup lang="ts">
import type { CodexPrivacyPolicy, PrivacySample } from '@/api/modules/settings/privacy'
import { ZButton, ZDialog, ZFormItem, ZTextarea } from '@codex-proxy/ui'
import { computed, shallowRef, watch } from 'vue'
import { usePrivacyPreview } from '../../composables/usePrivacyPreview'
import { exampleRequest } from './presets'

const props = defineProps<{ policy: CodexPrivacyPolicy }>()
const open = defineModel<boolean>({ required: true })
const sample = shallowRef('')
const localError = shallowRef('')
const preview = usePrivacyPreview()
const { loading, result } = preview
const output = computed(() => result.value ? JSON.stringify({ body: result.value.body, headers: result.value.headers, turnMetadata: result.value.turnMetadata }, null, 2) : '')
watch(open, (value) => {
  preview.invalidate()
  sample.value = value ? JSON.stringify(exampleRequest(), null, 2) : ''
  localError.value = ''
})
watch([sample, () => props.policy], () => preview.invalidate(), { deep: true, flush: 'sync' })
async function test() {
  localError.value = ''
  try {
    const parsed = JSON.parse(sample.value) as PrivacySample
    await preview.run({ ...props.policy, enabled: true }, parsed)
  }
  catch { localError.value = '请输入有效的请求样本 JSON' }
}
</script>

<template>
  <ZDialog v-model="open" title="测试规则" description="按草稿顺序执行已启用的规则，样本仅用于本次测试" width="48rem">
    <div class="grid gap-3">
      <div class="grid min-w-0 gap-3 sm:grid-cols-2">
        <ZFormItem label="请求样本">
          <ZTextarea v-model="sample" aria-label="请求样本" placeholder="粘贴包含 body、headers、turnMetadata 的请求 JSON" resize="none" maxlength="262144" size="small" class="font-mono [&_textarea]:block [&_textarea]:h-64" spellcheck="false" />
        </ZFormItem>
        <div class="grid min-w-0 content-start gap-2">
          <span class="flex min-h-4 items-center text-cp leading-none font-medium text-cp-text-secondary">最终结果</span><pre class="cp-scrollbar m-0 h-64 overflow-auto rounded-cp bg-cp-fill-quaternary p-3 font-mono text-cp-xs break-all whitespace-pre-wrap">{{ output || '点击测试查看结果' }}</pre>
        </div>
      </div>
      <ol v-if="result" class="m-0 grid list-none gap-1.5 p-0" aria-label="规则执行结果">
        <li v-for="outcome in result.outcomes" :key="outcome.ruleId" class="grid min-w-0 grid-cols-[minmax(0,1fr)_auto] items-center gap-x-4 gap-y-1 rounded-cp bg-cp-fill-quaternary px-3 py-2.5 text-cp-sm">
          <span class="truncate font-medium" :title="policy.rules.find(rule => rule.id === outcome.ruleId)?.name">{{ policy.rules.find(rule => rule.id === outcome.ruleId)?.name }}</span>
          <span v-if="outcome.status === 'disabled'" class="text-cp-xs text-cp-text-tertiary">未启用</span>
          <span v-else-if="outcome.status === 'skipped'" class="text-cp-xs text-cp-warning-text">已跳过</span>
          <span v-else class="inline-flex items-baseline gap-1 whitespace-nowrap text-cp-xs text-cp-text-secondary">
            命中 <strong class="text-cp-sm font-emphasis text-cp-text tabular-nums">{{ outcome.matches }}</strong> 处
          </span>
          <p v-if="outcome.reason" class="col-span-2 m-0 text-cp-xs break-words text-cp-warning-text">
            {{ outcome.reason }}
          </p>
        </li>
      </ol>
      <p v-if="localError" role="alert" class="m-0 text-cp-sm text-cp-error-text">
        {{ localError }}
      </p>
    </div>
    <template #footer>
      <ZButton @click="open = false">
        关闭
      </ZButton><ZButton type="primary" :loading="loading" @click="test">
        测试规则
      </ZButton>
    </template>
  </ZDialog>
</template>
