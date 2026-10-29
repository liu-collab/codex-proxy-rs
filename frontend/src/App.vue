<script setup lang="ts">
import { ZConfigProvider } from '@codex-proxy/ui'
import { watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterView } from 'vue-router'
import { applyLocale } from '@/i18n/locale'
import { uiLocale } from '@/i18n/ui'

const { locale } = useI18n({ useScope: 'global' })
watch(locale, applyLocale, { immediate: true })
</script>

<template>
  <ZConfigProvider :locale="uiLocale">
    <RouterView />
  </ZConfigProvider>
</template>
