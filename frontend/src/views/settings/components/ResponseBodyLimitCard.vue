<script setup lang="ts">
import { ZCard, ZForm, ZFormItem, ZInput } from '@codex-proxy/ui'

const model = defineModel<string>({ required: true })
</script>

<template>
  <ZCard title="请求体限制" description="限制 Responses 请求解压后的请求体大小">
    <ZForm class="max-w-6xl sm:grid-cols-2 sm:items-end">
      <ZFormItem label="解压后大小上限" description="调高可接收更大的图片和历史上下文，也会增加内存占用">
        <ZInput v-model="model" aria-label="解压后请求体大小上限" type="number" min="1" step="1">
          <template #suffix>
            MiB
          </template>
        </ZInput>
      </ZFormItem>
    </ZForm>
  </ZCard>
</template>
