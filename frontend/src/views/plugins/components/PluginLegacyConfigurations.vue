<script setup lang="ts">
import type { PluginArtifact, PluginInstance } from '@/api'

import { ZIconButton, ZTag } from '@codex-proxy/ui'
import { Play, Power, Settings2, Trash2 } from '@lucide/vue'
import { useI18n } from 'vue-i18n'
import { PLUGIN_STATUS_LABELS } from '../constants'
import { configurationStatus, pluginStatusType } from '../utils/catalog'
import { artifactForInstance } from '../utils/model'
import PluginStatusNotice from './PluginStatusNotice.vue'

defineProps<{ instances: PluginInstance[], artifacts: PluginArtifact[], busy: boolean }>()

defineEmits<{
  edit: [instance: PluginInstance]
  enable: [instance: PluginInstance]
  disable: [instance: PluginInstance]
  delete: [instance: PluginInstance]
}>()

const { t } = useI18n({ useScope: 'global' })
</script>

<template>
  <p v-if="instances.some(instance => instance.enabled)" role="status" class="m-0 text-cp-sm text-cp-warning-text">
    {{ t('plugins.configuration.legacy.multipleEnabledNotice') }}
  </p>
  <details v-if="instances.length" :open="instances.some(instance => instance.enabled)">
    <summary class="cursor-pointer text-cp-sm text-cp-text-secondary">
      {{ t('plugins.configuration.legacy.previousConfigurations', { count: instances.length }) }}
    </summary>
    <p class="text-cp-xs text-cp-text-secondary">
      {{ t('plugins.configuration.legacy.description') }}
    </p>
    <div v-for="instance in instances" :key="instance.id" class="mt-2 flex flex-wrap items-center gap-2 rounded-cp bg-cp-fill-alter p-3">
      <span class="min-w-0 flex-1 wrap-anywhere text-cp-sm">{{ instance.name }}</span>
      <ZTag size="small">
        {{ artifactForInstance(instance, artifacts)?.metadata.version }}
      </ZTag>
      <PluginStatusNotice :instance="instance" />
      <ZTag v-if="!['failed', 'incompatible'].includes(configurationStatus(instance))" size="small" :type="pluginStatusType(configurationStatus(instance))">
        {{ PLUGIN_STATUS_LABELS[configurationStatus(instance)] }}
      </ZTag>
      <div class="flex shrink-0 items-center gap-1">
        <ZIconButton :aria-label="t('plugins.details.settings')" variant="solid" size="small" :disabled="busy" @click="$emit('edit', instance)">
          <Settings2 class="size-4" />
        </ZIconButton>
        <ZIconButton v-if="instance.enabled" :aria-label="t('plugins.configuration.legacy.disableAction')" variant="solid" size="small" :disabled="busy" @click="$emit('disable', instance)">
          <Power class="size-4" />
        </ZIconButton>
        <ZIconButton v-else :aria-label="t('plugins.configuration.legacy.start')" size="small" variant="solid" :disabled="busy || Boolean(instance.loadError)" @click="$emit('enable', instance)">
          <Play class="size-4" />
        </ZIconButton>
        <ZIconButton v-if="!instance.enabled" :aria-label="t('plugins.configuration.legacy.deleteHistoricalConfiguration')" variant="solid" size="small" class="group" :disabled="busy" @click="$emit('delete', instance)">
          <Trash2 class="size-4 text-cp-error-text group-disabled:text-cp-text-disabled" />
        </ZIconButton>
      </div>
    </div>
  </details>
</template>
