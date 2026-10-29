<script setup lang="ts">
import type { PluginInstance } from '@/api'

import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { PLUGIN_STATUS_LABELS } from '../constants'
import { configurationStatus } from '../utils/catalog'
import PluginHelpPopover from './PluginHelpPopover.vue'

const props = defineProps<{ instance: PluginInstance, failureReason?: string }>()

const { t } = useI18n({ useScope: 'global' })

const status = computed(() => configurationStatus(props.instance))
const failure = computed(() => props.failureReason ?? props.instance.loadError ?? props.instance.runtime.failure?.message)
const incompatible = computed(() => !props.failureReason && status.value === 'incompatible')
const tone = computed(() => (failure.value || status.value === 'failed') && !incompatible.value
  ? 'error'
  : 'warning')
const title = computed(() => {
  if (props.failureReason)
    return PLUGIN_STATUS_LABELS.value.failed
  if (incompatible.value)
    return t('plugins.status.incompatibleNotStarted')
  if (failure.value || status.value === 'failed') {
    return props.instance.loadError
      ? t('plugins.status.unableToStart')
      : t('plugins.status.runFailed')
  }
  return props.instance.compatibilityWarning
    ? t('plugins.status.compatibilityReminder')
    : t('plugins.status.deprecatedApi')
})
</script>

<template>
  <PluginHelpPopover
    v-if="failure || status === 'failed' || instance.compatibilityWarning || instance.apiDeprecations.length"
    :label="`${instance.name}：${title}`"
    :tone="tone"
  >
    <p class="m-0 font-emphasis text-cp-text">
      {{ title }}
    </p>
    <p v-if="failure" class="m-0" :class="tone === 'error' ? 'text-cp-error-text' : 'text-cp-warning'">
      {{ failure }}
    </p>
    <p v-if="instance.runtime.failure && !incompatible" class="m-0 font-mono">
      {{ instance.runtime.failure.code }}
    </p>
    <template v-if="instance.compatibilityWarning && !incompatible">
      <p class="m-0 text-cp-warning">
        {{ instance.compatibilityWarning }}
      </p>
      <p class="m-0">
        {{ t('plugins.status.startupUnverified') }}
      </p>
    </template>
    <div v-for="entry in instance.apiDeprecations" :key="`${entry.capability}:${entry.version}`" class="grid gap-1">
      <p class="m-0 font-mono text-cp-text">
        {{ entry.capability }} v{{ entry.version }} → v{{ entry.replacementVersion }}
      </p>
      <p class="m-0 text-cp-warning">
        {{ entry.remainingReleases > 0 ? t('plugins.status.compatibilityWindow', { remainingReleases: entry.remainingReleases }) : t('plugins.status.compatibilityEnding') }}
      </p>
      <p class="m-0">
        {{ entry.migration }}
      </p>
    </div>
    <p v-if="instance.loadError" class="m-0">
      {{ t('plugins.status.unsupportedVersionHelp') }}
    </p>
    <p v-else-if="(failure || status === 'failed') && !instance.compatibilityWarning" class="m-0">
      {{ t('plugins.status.restartHelp') }}
    </p>
    <p v-else-if="instance.apiDeprecations.length" class="m-0">
      {{ t('plugins.status.upgradeHelp') }}
    </p>
  </PluginHelpPopover>
</template>
