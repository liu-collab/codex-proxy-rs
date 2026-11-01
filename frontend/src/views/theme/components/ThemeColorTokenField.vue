<script setup lang="ts">
import { ZColorPicker, ZIconButton } from '@codex-proxy/ui'

import { supportsThemeTokenAlpha } from '@codex-proxy/ui/theme'

import { RotateCcw } from '@lucide/vue'
import { useI18n } from 'vue-i18n'

withDefaults(defineProps<{
  label: string
  token: string
  value: string
  description?: string
  presets?: readonly string[]
  overridden?: boolean
}>(), {
  description: undefined,
  presets: () => [],
  overridden: false,
})

const emit = defineEmits<{
  change: [value: string]
  reset: []
}>()

const { t } = useI18n({ useScope: 'global' })
</script>

<template>
  <div class="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-3 rounded-z-lg bg-z-fill-quaternary px-3 py-3">
    <div class="min-w-0">
      <div class="flex flex-wrap items-center gap-2">
        <strong class="text-z-sm font-bold text-z-text">{{ label }}</strong>
        <span v-if="overridden" class="rounded-full bg-z-primary-container px-1.5 py-0.5 text-[9px] font-heavy text-z-primary-on-container">
          {{ t('appearance.color.modified') }}
        </span>
      </div>
      <code class="mt-1 block truncate font-mono text-[10px] text-z-text-quaternary">{{ token }}</code>
      <p
        v-if="description"
        class="mt-1.5 mb-0 truncate text-[10px] leading-[1.45] font-emphasis text-z-text-secondary"
        :title="description"
      >
        {{ description }}
      </p>
    </div>

    <div class="flex items-center gap-1.5 rounded-z bg-z-bg-container p-1 shadow-z-tertiary">
      <ZColorPicker
        :model-value="value"
        :presets="presets"
        :allow-alpha="supportsThemeTokenAlpha(token)"
        :aria-label="t('appearance.color.edit', { label })"
        @update:model-value="emit('change', $event)"
      />
      <span class="min-w-17 font-mono text-[10px] tabular-nums text-z-text-secondary">{{ value }}</span>
      <ZIconButton
        v-if="overridden"
        :aria-label="t('appearance.color.restoreDerivedValue')"
        size="small"
        @click="emit('reset')"
      >
        <RotateCcw class="size-3.5" />
      </ZIconButton>
    </div>
  </div>
</template>
