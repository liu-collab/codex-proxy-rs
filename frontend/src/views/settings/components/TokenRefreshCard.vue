<script setup lang="ts">
import { ZCard, ZForm, ZFormItem, ZInput } from '@codex-proxy/ui'
import { Timer, Zap } from '@lucide/vue'

const refreshMarginSeconds = defineModel<string>('refreshMarginSeconds', { required: true })
const refreshConcurrency = defineModel<string>('refreshConcurrency', { required: true })
</script>

<template>
  <ZCard title="凭据刷新">
    <ZForm class="max-w-6xl sm:grid-cols-2">
      <ZFormItem label="提前刷新时间（秒）" description="访问令牌到期前提前触发自动刷新的时间">
        <ZInput v-model="refreshMarginSeconds" aria-label="提前刷新时间（秒）" type="number">
          <template #prefix>
            <Timer class="size-4" />
          </template>
        </ZInput>
      </ZFormItem>
      <ZFormItem label="刷新并发上限" description="同时执行凭据刷新的最大请求数，降低上限可减少限流风险">
        <ZInput v-model="refreshConcurrency" aria-label="刷新并发上限" type="number">
          <template #prefix>
            <Zap class="size-4" />
          </template>
        </ZInput>
      </ZFormItem>
    </ZForm>
  </ZCard>
</template>
