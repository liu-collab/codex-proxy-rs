<script setup lang="ts">
import { ZCard, ZForm, ZFormItem, ZInput } from '@codex-proxy/ui'

import { useI18n } from 'vue-i18n'

const { t } = useI18n({ useScope: 'global' })

const leadMinutes = defineModel<string>({ required: true })
const maxWeeklyRemainingPercent = defineModel<string>('maxWeeklyRemainingPercent', { required: true })
</script>

<template>
  <ZCard id="reset-credits" :title="t('settings.quotaReset.title')" :description="t('settings.quotaReset.enableHelp')">
    <ZForm class="max-w-6xl sm:grid-cols-2">
      <ZFormItem :label="t('settings.advanceUsageTimeMinutes')" :description="t('settings.quotaReset.description')">
        <ZInput v-model="leadMinutes" type="number" min="1" max="1440" step="1" :aria-label="t('settings.resetCardAdvanceTime')" />
      </ZFormItem>
      <ZFormItem :label="t('settings.remainingWeeklyQuotaLimit')" :description="t('settings.quotaReset.thresholdHelp')">
        <ZInput v-model="maxWeeklyRemainingPercent" type="number" min="0" max="100" step="1" :placeholder="t('common.values.unlimited')" :aria-label="t('settings.quotaReset.weeklyQuotaLimit')" />
      </ZFormItem>
    </ZForm>
  </ZCard>
</template>
