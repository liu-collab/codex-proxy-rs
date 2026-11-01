<script setup lang="ts">
import type { ApiKeyAccountForm } from '../utils/upstreamApiKey'

import { ZFormItem, ZInput, ZSegmented } from '@codex-proxy/ui'

import { useI18n } from 'vue-i18n'

defineProps<{ disabled?: boolean, editing?: boolean }>()

const { t } = useI18n({ useScope: 'global' })

const form = defineModel<ApiKeyAccountForm>({ required: true })
const transportOptions = [
  { label: 'SSE', value: 'http' },
  { label: 'WS', value: 'prefer_websocket' },
]
</script>

<template>
  <ZFormItem prop="apiKey.base_url" :show-message="Boolean(form.base_url.trim())" :label="t('common.fields.baseUrl')" required>
    <template #extra>
      <span class="text-z-xs text-z-text-quaternary" :title="t('accounts.credentials.baseUrlHelp')">{{ t('accounts.credentials.apiPrefix') }}</span>
    </template>
    <ZInput v-model="form.base_url" type="url" placeholder="https://api.example.com/v1" :disabled="disabled" autocomplete="off" />
  </ZFormItem>
  <ZFormItem prop="apiKey.apiKey" :show-message="Boolean(form.apiKey)" :label="t('common.fields.apiKey')" :required="!editing">
    <ZInput v-model="form.apiKey" type="password" :placeholder="editing ? t('accounts.credentials.apiKeyPlaceholder') : t('accounts.credentials.enterUpstreamKey')" :disabled="disabled" autocomplete="new-password" />
  </ZFormItem>
  <ZFormItem prop="apiKey.transport" :show-message="false" :label="t('accounts.credentials.transportLabel')">
    <ZSegmented v-model="form.transport" :aria-label="t('accounts.credentials.transportLabel')" :options="transportOptions" :disabled="disabled" :title="t('accounts.credentials.webSocketHelp')" />
  </ZFormItem>
</template>
