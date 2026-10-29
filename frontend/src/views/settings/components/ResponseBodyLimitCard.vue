<script setup lang="ts">
import { ZCard, ZForm, ZFormItem, ZInput } from '@codex-proxy/ui'

import { useI18n } from 'vue-i18n'

const { t } = useI18n({ useScope: 'global' })

const model = defineModel<string>({ required: true })
</script>

<template>
  <ZCard :title="t('settings.responseLimit.requestBodyLimit')" :description="t('settings.responseLimit.description')">
    <ZForm class="max-w-6xl sm:grid-cols-2 sm:items-end">
      <ZFormItem :label="t('settings.responseLimit.maximumSizeAfterDecompression')" :description="t('settings.responseLimit.sizeHelp')">
        <ZInput v-model="model" :aria-label="t('settings.responseLimit.maxSizeLabel')" type="number" min="1" step="1">
          <template #suffix>
            MiB
          </template>
        </ZInput>
      </ZFormItem>
    </ZForm>
  </ZCard>
</template>
