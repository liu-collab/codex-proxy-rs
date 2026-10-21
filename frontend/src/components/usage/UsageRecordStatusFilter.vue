<script setup lang="ts">
import type { UsageRecordStatus } from '@/api/modules/usage'
import { ZSegmented } from '@codex-proxy/ui'
import { CloudRain, List, Sun } from '@lucide/vue'

const model = defineModel<UsageRecordStatus>({ required: true })
const options = [
  { label: '成功', value: 'success', icon: Sun },
  { label: '错误', value: 'error', icon: CloudRain },
  { label: '全部', value: 'all', icon: List },
]
</script>

<template>
  <ZSegmented v-model="model" aria-label="请求明细类型" :options="options" display="icon" class="w-30 shrink-0" />
</template>
