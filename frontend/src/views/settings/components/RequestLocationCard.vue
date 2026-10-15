<script setup lang="ts">
import type { RequestLocation } from '@/api'
import { ZCard, ZForm, ZSwitch } from '@codex-proxy/ui'
import RequestLocationFields from '@/components/RequestLocationFields.vue'

defineProps<{ disabled: boolean }>()
const location = defineModel<RequestLocation>({ required: true })
const enabled = defineModel<boolean>('enabled', { required: true })
</script>

<template>
  <ZCard title="请求位置覆盖" description="覆盖 Codex 请求中的时区与位置字段，账号出站代理的自定义位置优先">
    <template #body>
      <div>
        <ZSwitch v-model="enabled" label="启用位置覆盖" :disabled="disabled" />
      </div>
      <ZForm class="mt-4 max-w-6xl">
        <RequestLocationFields v-model="location" :disabled="disabled || !enabled" />
      </ZForm>
    </template>
  </ZCard>
</template>
