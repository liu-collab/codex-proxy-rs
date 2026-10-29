<script setup lang="ts">
import type { RequestLocation } from '@/api'

import { ZFormItem, ZInput } from '@codex-proxy/ui'
import { useI18n } from 'vue-i18n'

defineProps<{ disabled?: boolean }>()

const { t } = useI18n({ useScope: 'global' })

const location = defineModel<RequestLocation>({ required: true })

function update(field: keyof RequestLocation, value: string) {
  location.value = { ...location.value, [field]: value }
}
</script>

<template>
  <div class="grid gap-4 sm:grid-cols-2">
    <ZFormItem :label="t('common.location.countryCode')" required>
      <ZInput :model-value="location.country" :disabled="disabled" :aria-label="t('common.location.countryCode')" :placeholder="t('common.location.countryCodePlaceholder')" maxlength="2" @update:model-value="update('country', $event)" />
    </ZFormItem>
    <ZFormItem :label="t('common.location.region')" required>
      <ZInput :model-value="location.region" :disabled="disabled" :aria-label="t('common.location.region')" :placeholder="t('common.location.regionPlaceholder')" maxlength="128" @update:model-value="update('region', $event)" />
    </ZFormItem>
    <ZFormItem :label="t('common.location.city')" required>
      <ZInput :model-value="location.city" :disabled="disabled" :aria-label="t('common.location.city')" :placeholder="t('common.location.cityPlaceholder')" maxlength="128" @update:model-value="update('city', $event)" />
    </ZFormItem>
    <ZFormItem :label="t('common.location.timeZone')" required>
      <ZInput :model-value="location.timezone" :disabled="disabled" :aria-label="t('common.location.timeZone')" :placeholder="t('common.location.timeZonePlaceholder')" @update:model-value="update('timezone', $event)" />
    </ZFormItem>
  </div>
</template>
