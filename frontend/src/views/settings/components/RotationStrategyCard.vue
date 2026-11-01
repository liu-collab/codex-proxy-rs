<script setup lang="ts">
import type { rotationOptions } from '../constants'

import type { SmartSchedulingConfig } from '@/api'
import { ZCard, ZIconButton } from '@codex-proxy/ui'
import { Settings2 } from '@lucide/vue'
import { isEqual } from 'es-toolkit'
import { computed, shallowRef } from 'vue'
import { useI18n } from 'vue-i18n'
import SmartSchedulingModal from './SmartSchedulingModal.vue'

const props = defineProps<{
  options: readonly RotationOption[]
  smartDefaults?: SmartSchedulingConfig
  disabled: boolean
}>()

const { t } = useI18n({ useScope: 'global' })

type RotationOption = (typeof rotationOptions.value)[number]
type RotationStrategy = RotationOption['value']

const model = defineModel<RotationStrategy | ''>({ required: true })
const smartScheduling = defineModel<SmartSchedulingConfig | undefined>('smartScheduling', { required: true })
const settingsOpen = shallowRef(false)
const customized = computed(() => smartScheduling.value && props.smartDefaults && !isEqual(smartScheduling.value, props.smartDefaults))
</script>

<template>
  <ZCard :title="t('settings.rotation.title')" class="@container">
    <div class="grid max-w-6xl gap-3 @md:grid-cols-2 @5xl:grid-cols-4">
      <div
        v-for="option in options"
        :key="option.value"
        class="relative flex"
      >
        <button
          type="button"
          class="flex min-h-25 w-full cursor-pointer flex-col rounded-z border-0 px-4 py-3.5 text-left shadow-z-input outline-none transition-[background-color,box-shadow,color] duration-160 focus-visible:ring-2 focus-visible:ring-z-control-outline"
          :class="
            model === option.value
              ? 'bg-z-control-item-bg-active text-z-primary-text shadow-z-tertiary'
              : 'bg-(--z-input-bg,var(--z-input-bg)) text-z-text hover:bg-(--z-input-hover-bg,var(--z-input-hover-bg)) hover:shadow-z-input-hover'
          "
          :aria-pressed="model === option.value"
          :disabled="disabled"
          @click="model = option.value"
        >
          <span class="flex items-center gap-2" :class="option.value === 'smart' ? 'pr-7' : undefined">
            <span
              class="inline-flex size-4 shrink-0 items-center justify-center rounded-full bg-z-bg-container shadow-[inset_0_0_0_1px_var(--z-color-border)]"
            >
              <span
                class="size-2 rounded-full transition-opacity duration-150"
                :class="model === option.value ? 'bg-z-primary opacity-100' : 'opacity-0'"
              />
            </span>
            <span class="inline-flex items-baseline gap-1 text-z leading-snug font-emphasis">
              {{ option.label }}
              <span v-if="option.value === 'smart' && customized" class="shrink-0 text-z-xs font-normal text-z-text-secondary">{{ t('settings.rotation.customized') }}</span>
            </span>
          </span>
          <span class="mt-2 block text-z leading-normal font-emphasis text-z-text-secondary">
            {{ option.description }}
          </span>
        </button>
        <ZIconButton
          v-if="option.value === 'smart'"
          class="absolute top-3 right-3 size-6! rounded-z-sm p-0"
          :aria-label="t('settings.rotation.smartSchedulingSettings')"
          size="small"
          :disabled="disabled || !smartScheduling || !smartDefaults"
          @click="settingsOpen = true"
        >
          <Settings2 class="size-3.5" />
        </ZIconButton>
      </div>
    </div>
  </ZCard>
  <SmartSchedulingModal
    v-if="smartScheduling && smartDefaults"
    v-model="settingsOpen"
    :config="smartScheduling"
    :defaults="smartDefaults"
    :active="model === 'smart'"
    @confirm="smartScheduling = $event"
  />
</template>
