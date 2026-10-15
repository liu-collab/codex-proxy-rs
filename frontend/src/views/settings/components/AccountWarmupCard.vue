<script setup lang="ts">
import { ZCard, ZForm, ZFormItem, ZInput, ZSwitch } from '@codex-proxy/ui'
import { Clock, Sparkles } from '@lucide/vue'

const enabled = defineModel<boolean>('enabled', { required: true })
const scheduleTime = defineModel<string>('scheduleTime', { required: true })
const model = defineModel<string>('model', { required: true })
</script>

<template>
  <ZCard
    title="账号预激活"
    description="在指定时间向空闲的 Codex OAuth 账号发送一次请求，尝试启动额度窗口"
  >
    <ZForm class="max-w-6xl sm:grid-cols-2">
      <ZSwitch
        v-model="enabled"
        class="col-span-full justify-self-start"
        label="启用每日预激活"
      />

      <ZFormItem
        label="执行时间"
        description="HH:MM；多个时间用英文逗号分隔"
      >
        <ZInput
          v-model="scheduleTime"
          :disabled="!enabled"
          aria-label="每日激活时间"
          placeholder="08:00"
        >
          <template #prefix>
            <Clock class="size-4" />
          </template>
        </ZInput>
      </ZFormItem>

      <ZFormItem
        label="预激活模型"
        description="启用时必填，请填写账号可用的模型"
        :required="enabled"
      >
        <ZInput
          v-model="model"
          :disabled="!enabled"
          aria-label="预激活模型"
          placeholder="请输入模型名称"
        >
          <template #prefix>
            <Sparkles class="size-4" />
          </template>
        </ZInput>
      </ZFormItem>
    </ZForm>
  </ZCard>
</template>
