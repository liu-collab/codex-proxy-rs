<script setup lang="ts">
import type { PluginArtifact, PluginInstance } from '@/api'
import { ZIconButton, ZTag } from '@codex-proxy/ui'
import { Play, Power, Settings2, Trash2 } from '@lucide/vue'
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
</script>

<template>
  <p v-if="instances.some(instance => instance.enabled)" role="status" class="m-0 text-cp-sm text-cp-warning-text">
    检测到多套启用配置，请在历史配置中停用不再使用的一套
  </p>
  <details v-if="instances.length" :open="instances.some(instance => instance.enabled)">
    <summary class="cursor-pointer text-cp-sm text-cp-text-secondary">
      历史配置 · {{ instances.length }}
    </summary>
    <p class="text-cp-xs text-cp-text-secondary">
      保留已有设置和数据，使用历史配置会替换当前启用配置
    </p>
    <div v-for="instance in instances" :key="instance.id" class="mt-2 flex flex-wrap items-center gap-2 rounded-cp bg-cp-fill-alter p-3">
      <span class="min-w-0 flex-1 wrap-anywhere text-cp-sm">{{ instance.name }}</span>
      <ZTag size="small">
        {{ artifactForInstance(instance, artifacts)?.metadata.version }}
      </ZTag>
      <PluginStatusNotice :instance="instance" />
      <ZTag size="small" :type="pluginStatusType(configurationStatus(instance))">
        {{ PLUGIN_STATUS_LABELS[configurationStatus(instance)] }}
      </ZTag>
      <div class="flex shrink-0 items-center gap-1">
        <ZIconButton aria-label="设置" variant="solid" size="small" :disabled="busy" @click="$emit('edit', instance)">
          <Settings2 class="size-4" />
        </ZIconButton>
        <ZIconButton v-if="instance.enabled" aria-label="停用历史配置" variant="solid" size="small" :disabled="busy" @click="$emit('disable', instance)">
          <Power class="size-4" />
        </ZIconButton>
        <ZIconButton v-else aria-label="启动" size="small" variant="solid" :disabled="busy || Boolean(instance.loadError)" @click="$emit('enable', instance)">
          <Play class="size-4" />
        </ZIconButton>
        <ZIconButton v-if="!instance.enabled" aria-label="删除历史配置" variant="solid" size="small" class="group" :disabled="busy" @click="$emit('delete', instance)">
          <Trash2 class="size-4 text-cp-error-text group-disabled:text-cp-text-disabled" />
        </ZIconButton>
      </div>
    </div>
  </details>
</template>
