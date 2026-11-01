<script setup lang="ts">
import { useI18n } from 'vue-i18n'

defineProps<{
  apiKey: {
    enabled: boolean
  }
}>()

const { t } = useI18n({ useScope: 'global' })
</script>

<template>
  <span
    class="inline-flex h-6 min-w-14 items-center justify-center rounded-full px-2 text-z-sm leading-none font-bold"
    :class="{
      'bg-z-success-container text-z-success-on-container': apiKey.enabled,
      'bg-z-fill-quaternary text-z-text-secondary': !apiKey.enabled,
    }"
  >
    {{ apiKey.enabled ? t('common.status.enabled') : t('common.status.disabled') }}
  </span>
</template>
