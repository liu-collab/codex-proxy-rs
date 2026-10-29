<script setup lang="ts">
import type { RequestLocation } from '@/api'

import { ZCard, ZForm, ZSwitch } from '@codex-proxy/ui'
import { useI18n } from 'vue-i18n'
import RequestLocationDetectButton from '@/components/RequestLocationDetectButton.vue'
import RequestLocationFields from '@/components/RequestLocationFields.vue'

defineProps<{ disabled: boolean, detectingLocation: boolean }>()

const emit = defineEmits<{ detectLocation: [] }>()

const { t } = useI18n({ useScope: 'global' })

const location = defineModel<RequestLocation>({ required: true })
const enabled = defineModel<boolean>('enabled', { required: true })
</script>

<template>
  <ZCard :title="t('settings.location.requestLocationOverride')" :description="t('settings.location.description')">
    <template #body>
      <div class="flex max-w-6xl items-center justify-between gap-3">
        <ZSwitch v-model="enabled" :label="t('settings.location.enableLocationOverlay')" :disabled="disabled" />
        <RequestLocationDetectButton :loading="detectingLocation" :disabled="disabled" @detect="emit('detectLocation')" />
      </div>
      <ZForm class="mt-4 max-w-6xl">
        <RequestLocationFields v-model="location" :disabled="disabled || !enabled" />
      </ZForm>
    </template>
  </ZCard>
</template>
