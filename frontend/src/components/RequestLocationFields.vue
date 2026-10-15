<script setup lang="ts">
import type { RequestLocation } from '@/api'
import { ZFormItem, ZInput } from '@codex-proxy/ui'

defineProps<{ disabled?: boolean }>()
const location = defineModel<RequestLocation>({ required: true })

function update(field: keyof RequestLocation, value: string) {
  location.value = { ...location.value, [field]: value }
}
</script>

<template>
  <div class="grid gap-4 sm:grid-cols-2">
    <ZFormItem label="国家代码" required>
      <ZInput :model-value="location.country" :disabled="disabled" aria-label="国家代码" placeholder="请输入两位国家代码" maxlength="2" @update:model-value="update('country', $event)" />
    </ZFormItem>
    <ZFormItem label="地区" required>
      <ZInput :model-value="location.region" :disabled="disabled" aria-label="地区" placeholder="请输入地区" maxlength="128" @update:model-value="update('region', $event)" />
    </ZFormItem>
    <ZFormItem label="城市" required>
      <ZInput :model-value="location.city" :disabled="disabled" aria-label="城市" placeholder="请输入城市" maxlength="128" @update:model-value="update('city', $event)" />
    </ZFormItem>
    <ZFormItem label="IANA 时区" required>
      <ZInput :model-value="location.timezone" :disabled="disabled" aria-label="IANA 时区" placeholder="请输入 IANA 时区" @update:model-value="update('timezone', $event)" />
    </ZFormItem>
  </div>
</template>
