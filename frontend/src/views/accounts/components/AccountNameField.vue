<script setup lang="ts">
import { ZFormItem, ZInput } from '@codex-proxy/ui'
import { useI18n } from 'vue-i18n'

defineProps<{ disabled?: boolean, required?: boolean, placeholder?: string }>()
const name = defineModel<string>({ required: true })
const { t } = useI18n({ useScope: 'global' })
</script>

<template>
  <ZFormItem prop="name" :label="t('accounts.credentials.accountName')" :required="required">
    <ZInput
      v-model="name"
      :aria-label="t('accounts.credentials.accountName')"
      :placeholder="placeholder ?? t('accounts.editor.namePlaceholder')"
      :disabled="disabled"
    />
  </ZFormItem>
</template>
