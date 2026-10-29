<script setup lang="ts">
import type { InstalledPlugin } from '../utils/catalog'

import type { PluginArtifact } from '@/api'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { currentPluginInstance } from '../utils/catalog'
import { artifactForInstance } from '../utils/model'
import PluginVersionCard from './PluginVersionCard.vue'

const props = defineProps<{ plugin: InstalledPlugin, busy: boolean }>()

defineEmits<{
  switchVersion: [artifact: PluginArtifact]
  deleteVersion: [artifact: PluginArtifact]
  accept: [artifact: PluginArtifact]
}>()

const { t } = useI18n({ useScope: 'global' })

const current = computed(() => currentPluginInstance(props.plugin))
const currentArtifact = computed(() => current.value && artifactForInstance(current.value, props.plugin.artifacts))
const history = computed(() => props.plugin.artifacts.filter(artifact => artifact.metadata.sha256 !== currentArtifact.value?.metadata.sha256))
</script>

<template>
  <div class="grid gap-5">
    <PluginVersionCard
      v-if="currentArtifact"
      :artifact="currentArtifact"
      :current="current"
      :configurations="plugin.configurations"
      :busy="busy"
      @accept="$emit('accept', $event)"
      @switch-version="$emit('switchVersion', $event)"
      @delete-version="$emit('deleteVersion', $event)"
    />
    <details v-if="history.length" :open="!currentArtifact">
      <summary class="cursor-pointer text-cp-sm text-cp-text-secondary">
        {{ currentArtifact ? t('plugins.versions.historicalVersion') : t('plugins.catalog.installedVersion') }} · {{ history.length }}
      </summary>
      <ul class="mt-4 mb-0 grid list-none gap-2 p-0" :aria-label="currentArtifact ? t('plugins.versions.historicalVersion') : t('plugins.catalog.installedVersion')">
        <li v-for="artifact in history" :key="artifact.metadata.sha256" class="min-w-0">
          <PluginVersionCard
            :artifact="artifact"
            :current="current"
            :configurations="plugin.configurations"
            :busy="busy"
            @accept="$emit('accept', $event)"
            @switch-version="$emit('switchVersion', $event)"
            @delete-version="$emit('deleteVersion', $event)"
          />
        </li>
      </ul>
    </details>
  </div>
</template>
