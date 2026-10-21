<script setup lang="ts">
import type { UsageRecordDetail } from '@/api'
import { ZButton, ZDialog } from '@codex-proxy/ui'
import { computed } from 'vue'
import { usageAccountText, usageModelDisplay, usageTransportType } from '@/components/usage/shared/presenter'
import { formatDuration } from '@/utils/format'
import { attemptOutcomeClass, attemptOutcomeText, visibleResponseText } from '../utils/detail'
import DiagnosticsDownloadButton from './DiagnosticsDownloadButton.vue'
import UsageDetailCodePanel from './UsageDetailCodePanel.vue'
import UsageDetailFieldGrid from './UsageDetailFieldGrid.vue'
import UsageStatusCodeBadge from './UsageStatusCodeBadge.vue'

const props = defineProps<{ record: UsageRecordDetail | null }>()
const open = defineModel<boolean>({ default: false })

const panelClass = 'min-w-0 rounded-cp-card bg-cp-fill-quaternary px-4 py-3.5'
const panelTitleClass = 'm-0 text-cp-sm leading-none font-heavy text-cp-text-secondary'

const errorMessage = computed(() => {
  const message = props.record?.message.trim()
  return message && message !== props.record?.logicalOutcome ? message : '未记录具体错误消息'
})
const responseText = computed(() => props.record ? visibleResponseText(props.record) : '')
const metadataText = computed(() => props.record && Object.keys(props.record.metadata).length
  ? JSON.stringify(props.record.metadata, null, 2)
  : '')
const contextItems = computed(() => [
  { label: '账号', value: props.record ? usageAccountText(props.record) : null, mono: true, fullWidth: true },
  { label: '时间', value: props.record?.createdAtDisplay, mono: true },
  { label: '总耗时', value: props.record?.latencyMsDisplay, mono: true },
  { label: '平台', value: props.record?.provider, mono: true },
  { label: '端点', value: props.record?.route, mono: true },
  { label: '请求模型', value: props.record ? usageModelDisplay(props.record).primary : null, mono: true },
  { label: '上游模型', value: props.record?.upstreamModel, mono: true },
  { label: '客户端传输', value: usageTransportType(props.record?.clientTransport), mono: true },
  { label: '上游传输', value: usageTransportType(props.record?.upstreamTransport), mono: true },
])
const identifierItems = computed(() => [
  { label: '客户端状态码', value: props.record?.clientStatusCode, mono: true },
  { label: '上游状态码', value: props.record?.upstreamStatusCode, mono: true },
  { label: '请求 ID', value: props.record?.requestId, mono: true, fullWidth: true },
  { label: '上游请求 ID', value: props.record?.upstreamRequestId, mono: true, fullWidth: true },
  { label: '响应 ID', value: props.record?.responseId, mono: true, fullWidth: true },
  { label: '客户端 Key ID', value: props.record?.clientApiKeyId, mono: true, fullWidth: true },
])
const attempts = computed(() => (props.record?.attempts ?? []).map(attempt => ({
  id: attempt.id,
  index: attempt.attemptIndex,
  outcome: attempt.outcome,
  statusCode: attempt.statusCode,
  items: [
    { label: '错误码', value: attempt.providerErrorCode, mono: true },
    { label: '失败分类', value: attempt.failureClass, mono: true },
    { label: '账号', value: usageAccountText(attempt), mono: true },
    { label: '平台', value: attempt.provider, mono: true },
    { label: '上游传输', value: usageTransportType(attempt.transport), mono: true },
    { label: '耗时', value: formatDuration(attempt.latencyMs), mono: true },
    { label: '触发方式', value: attempt.trigger, mono: true },
    { label: '发送状态', value: attempt.sendState, mono: true },
  ],
})))
</script>

<template>
  <ZDialog v-model="open" title="错误详情" description="失败原因与请求上下文" type="danger" width="64rem">
    <div v-if="record" class="grid min-w-0 gap-3">
      <section class="min-w-0 rounded-cp-card bg-cp-error-container px-4 py-3.5 text-cp-error-on-container">
        <div class="flex items-center gap-3">
          <h3 class="m-0 text-cp-sm leading-none font-heavy">
            {{ record.logicalOutcome === 'incomplete' ? '响应未完成' : '请求失败' }}
          </h3>
          <UsageStatusCodeBadge v-if="record.statusCode !== null" :status-code="record.statusCode" />
        </div>
        <p class="mt-3 mb-0 wrap-anywhere whitespace-pre-wrap font-mono text-cp-sm leading-relaxed">
          {{ errorMessage }}
        </p>
      </section>

      <section class="grid min-w-0 gap-3 lg:grid-cols-2">
        <section :class="panelClass">
          <h3 :class="panelTitleClass">
            请求上下文
          </h3>
          <UsageDetailFieldGrid :items="contextItems" />
        </section>
        <section :class="panelClass">
          <h3 :class="panelTitleClass">
            错误定位
          </h3>
          <UsageDetailFieldGrid :items="identifierItems" />
        </section>
      </section>

      <section v-if="attempts.length || !record.attemptsComplete" :class="panelClass">
        <h3 :class="panelTitleClass">
          尝试链路
        </h3>
        <p v-if="!record.attemptsComplete" class="mt-3 mb-0 text-cp-sm text-cp-text-tertiary">
          尝试记录可能不完整
        </p>
        <ol v-if="attempts.length" class="mt-3 mb-0 grid list-none gap-4 p-0 lg:grid-cols-2">
          <li v-for="attempt in attempts" :key="attempt.id" class="min-w-0">
            <div class="flex items-center gap-3 text-cp-sm font-bold">
              <span>尝试 {{ attempt.index }}</span>
              <span :class="attemptOutcomeClass(attempt.outcome)">{{ attemptOutcomeText(attempt.outcome) }}</span>
              <UsageStatusCodeBadge :status-code="attempt.statusCode" />
            </div>
            <UsageDetailFieldGrid :items="attempt.items" />
          </li>
        </ol>
      </section>

      <details v-if="responseText || metadataText" :class="panelClass">
        <summary class="cursor-pointer text-cp-sm font-heavy text-cp-text-secondary">
          诊断数据
        </summary>
        <div class="mt-3 grid min-w-0 gap-3">
          <UsageDetailCodePanel v-if="responseText" title="响应内容" max-height="180px" :content="responseText" />
          <UsageDetailCodePanel v-if="metadataText" title="元数据" max-height="240px" :content="metadataText" />
        </div>
      </details>
    </div>

    <template #footer>
      <DiagnosticsDownloadButton v-if="record" :request-id="record.requestId" :active="open" />
      <ZButton type="primary" @click="open = false">
        关闭
      </ZButton>
    </template>
  </ZDialog>
</template>
