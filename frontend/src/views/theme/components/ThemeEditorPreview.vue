<script setup lang="ts">
import type { ThemeScheme } from '@codex-proxy/ui/theme'

import type { CSSProperties } from 'vue'

import type { ThemeEditorPreview } from '../composables/useThemeEditor'
import { ZSegmented } from '@codex-proxy/ui'
import { computed } from 'vue'

import { useI18n } from 'vue-i18n'

import ThemeComponentPreview from './ThemeComponentPreview.vue'
import ThemeDashboardPreview from './ThemeDashboardPreview.vue'
import ThemePreviewCanvas from './ThemePreviewCanvas.vue'

defineProps<{
  theme: ThemeScheme
  style: CSSProperties
}>()

const { t } = useI18n({ useScope: 'global' })

const preview = defineModel<ThemeEditorPreview>({ required: true })

const previewOptions = computed(() => ([
  {
    label: t('appearance.preview.homeArtboard'),
    value: 'page',
  },
  {
    label: t('appearance.preview.components.componentOverview'),
    value: 'components',
  },
]))
</script>

<template>
  <section class="grid h-full min-h-0 grid-rows-[auto_minmax(0,1fr)] gap-3 overflow-hidden" :aria-label="t('appearance.preview.themeRealTimePreview')">
    <header class="shrink-0 flex flex-wrap items-center justify-between gap-3 rounded-z-lg bg-z-bg-container px-3 py-2.5 shadow-z-tertiary">
      <div>
        <strong class="block text-z-sm font-heavy text-z-text">{{ t('appearance.preview.livePreview') }}</strong>
        <span class="mt-0.5 block text-[9px] font-emphasis text-z-text-quaternary">{{ t('appearance.preview.description') }}</span>
      </div>
      <ZSegmented v-model="preview" :aria-label="t('appearance.preview.previewType')" size="small" :options="previewOptions" />
    </header>

    <ThemePreviewCanvas
      class="h-full min-h-0"
      :theme="theme"
      :style="style"
    >
      <ThemeDashboardPreview v-if="preview === 'page'" />
      <ThemeComponentPreview v-else />
    </ThemePreviewCanvas>
  </section>
</template>
