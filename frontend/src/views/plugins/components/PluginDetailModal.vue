<script setup lang="ts">
import type { InstalledPlugin } from '../utils/catalog'
import type { PluginArtifact, PluginInstance, PluginManagementView } from '@/api'
import { ArrowInDownSquareHalf } from '@boxicons/vue'
import { ZButton, ZDialog, ZEmpty, ZIconButton, ZSegmented, ZTag } from '@codex-proxy/ui'
import { ArrowUpRight, History, Layers, Play, Power, RefreshCw, Settings2 } from '@lucide/vue'
import { computed, shallowRef, watch } from 'vue'
import { RouterLink } from 'vue-router'
import { pluginPageLocation } from '@/utils/plugin'
import { PLUGIN_STATUS_LABELS } from '../constants'
import { configurationStatus, currentPluginInstance, hasPluginSettings, pluginStatusType } from '../utils/catalog'
import { artifactForInstance } from '../utils/model'
import PluginConfigurationSummary from './PluginConfigurationSummary.vue'
import PluginHelpPopover from './PluginHelpPopover.vue'
import PluginLegacyConfigurations from './PluginLegacyConfigurations.vue'
import PluginStatusNotice from './PluginStatusNotice.vue'
import PluginVersionsPanel from './PluginVersionsPanel.vue'

const props = defineProps<{ plugin: InstalledPlugin | null, initialSection: 'configurations' | 'versions', views: PluginManagementView[], busy: boolean }>()
defineEmits<{
  accept: [artifact: PluginArtifact]
  edit: [instance: PluginInstance]
  enable: [instance: PluginInstance]
  disable: [instance: PluginInstance]
  deleteConfiguration: [instance: PluginInstance]
  deleteVersion: [artifact: PluginArtifact]
  rollback: [instance: PluginInstance]
  installVersion: [plugin: InstalledPlugin]
  checkUpdate: [plugin: InstalledPlugin]
  switchVersion: [artifact: PluginArtifact]
  uninstall: [plugin: InstalledPlugin]
}>()
const open = defineModel<boolean>({ required: true })
const section = shallowRef('configurations')
const sections = [{ label: '概览', value: 'configurations', icon: Settings2 }, { label: '版本', value: 'versions', icon: Layers }]
const current = computed(() => props.plugin && currentPluginInstance(props.plugin))
const legacy = computed(() => props.plugin?.configurations.filter(instance => instance.id !== current.value?.id) ?? [])
const currentArtifact = computed(() => current.value && artifactForInstance(current.value, props.plugin?.artifacts ?? []))
const acceptedArtifact = computed(() => props.plugin?.artifacts.find(artifact => artifact.acceptedAt) ?? null)
const pendingArtifact = computed(() => props.plugin?.artifacts.find(artifact => !artifact.acceptedAt) ?? null)
const viewByInstance = computed(() => new Map(props.views.map(view => [view.target.instanceId, view])))
function capabilities(instance: PluginInstance) {
  return artifactForInstance(instance, props.plugin?.artifacts ?? [])?.metadata.contributes ?? {}
}
function configurationNotes(instance: PluginInstance) {
  const notes: string[] = []
  if (configurationStatus(instance) === 'pending')
    notes.push('配置已保存，等待生效，状态会自动刷新')
  if (configurationStatus(instance) === 'unconfigured')
    notes.push('补充必填配置后即可启用')
  if (instance.runtime.drainingRevisions.length)
    notes.push('旧版本仍有进行中的调用，完成前暂不能删除')
  return notes
}
watch(open, (value) => {
  if (value)
    section.value = props.initialSection
})
watch(() => props.initialSection, value => section.value = value)
</script>

<template>
  <ZDialog v-model="open" :title="plugin?.artifact.metadata.displayName ?? '插件详情'" :description="plugin?.artifact.metadata.description" width="42rem" :show-close="!busy" :close-on-click-modal="!busy" :close-on-press-escape="!busy">
    <div v-if="plugin" class="grid gap-5">
      <div class="flex flex-wrap items-center gap-3">
        <ZSegmented v-model="section" :options="sections" aria-label="插件详情分区" class="w-44" />
        <div v-if="plugin.artifact.source.kind !== 'builtin'" class="ml-auto flex items-center gap-2">
          <ZIconButton v-if="plugin.source?.source.kind === 'github' || plugin.source?.source.kind === 'url'" aria-label="检查更新" variant="solid" :disabled="busy" @click="$emit('checkUpdate', plugin)">
            <RefreshCw class="size-4" />
          </ZIconButton>
          <ZIconButton aria-label="手动安装版本" variant="solid" :disabled="busy" @click="$emit('installVersion', plugin)">
            <ArrowInDownSquareHalf pack="filled" class="size-5" />
          </ZIconButton>
        </div>
      </div>
      <template v-if="section === 'configurations'">
        <ZEmpty v-if="!plugin.configurations.length" :title="acceptedArtifact ? '插件尚未初始化' : '插件待安装'" size="small" surface="inset">
          <template #action>
            <ZButton v-if="acceptedArtifact" type="primary" @click="$emit('accept', acceptedArtifact)">
              初始化插件
            </ZButton>
            <ZButton v-else-if="pendingArtifact" type="primary" @click="$emit('accept', pendingArtifact)">
              安装
            </ZButton>
          </template>
        </ZEmpty>
        <article v-for="instance in current ? [current] : []" :key="instance.id" class="grid min-h-36 content-between gap-3 rounded-cp bg-cp-fill-alter p-4">
          <div class="flex flex-wrap items-center gap-2">
            <strong class="min-w-0 flex-1 wrap-break-word text-cp-sm">当前版本</strong>
            <ZTag>{{ artifactForInstance(instance, plugin.artifacts)?.metadata.version ?? '版本不可用' }}</ZTag>
            <PluginStatusNotice :instance="instance" />
            <ZTag :type="pluginStatusType(configurationStatus(instance))">
              {{ PLUGIN_STATUS_LABELS[configurationStatus(instance)] }}
            </ZTag>
            <PluginHelpPopover v-if="configurationNotes(instance).length" :label="`${instance.name}配置状态说明`">
              <p v-for="note in configurationNotes(instance)" :key="note" class="m-0">
                {{ note }}
              </p>
            </PluginHelpPopover>
          </div>
          <PluginConfigurationSummary
            :instance="instance"
            :metadata="artifactForInstance(instance, plugin.artifacts)?.metadata"
            :show-command="configurationStatus(instance) === 'enabled' && Boolean(capabilities(instance).command_line)"
          />
          <div class="flex flex-wrap items-center gap-2">
            <div v-if="configurationStatus(instance) === 'enabled'" class="mr-auto flex min-w-0 flex-wrap items-center gap-2">
              <RouterLink v-for="page in viewByInstance.get(instance.id)?.pages ?? []" :key="page.id" :to="pluginPageLocation(viewByInstance.get(instance.id)!, page)" class="inline-flex h-cp-control-sm min-w-0 items-center gap-1.5 rounded-cp bg-cp-primary-container px-2.5 text-cp-sm leading-none text-cp-primary-on-container no-underline outline-none transition-colors hover:bg-cp-primary-container-hover focus-visible:ring-2 focus-visible:ring-cp-control-outline motion-reduce:transition-none" @click="open = false">
                <span class="truncate">{{ page.title }}</span>
                <ArrowUpRight class="size-3.5 shrink-0" />
              </RouterLink>
            </div>
            <ZIconButton v-if="instance.configurationRequired || (currentArtifact && hasPluginSettings(currentArtifact))" size="small" variant="solid" aria-label="设置" :disabled="busy" @click="$emit('edit', instance)">
              <Settings2 class="size-4" />
            </ZIconButton>
            <ZIconButton v-if="configurationStatus(instance) === 'failed'" aria-label="重新启动" size="small" variant="solid" :disabled="busy || Boolean(instance.loadError)" @click="$emit('enable', instance)">
              <RefreshCw class="size-4" />
            </ZIconButton>
            <ZIconButton v-if="instance.enabled" aria-label="停用" size="small" variant="solid" :disabled="busy" @click="$emit('disable', instance)">
              <Power class="size-4" />
            </ZIconButton>
            <ZIconButton v-else :aria-label="instance.configurationRequired ? '完成设置并启用' : '启用'" size="small" variant="solid" :disabled="busy || Boolean(instance.loadError)" @click="$emit('enable', instance)">
              <Play class="size-4" />
            </ZIconButton>
            <ZIconButton v-if="plugin.artifacts.length > 1" aria-label="回退版本" size="small" type="default" variant="solid" :disabled="busy" @click="$emit('rollback', instance)">
              <History class="size-4" />
            </ZIconButton>
          </div>
        </article>
        <PluginLegacyConfigurations
          :instances="legacy"
          :artifacts="plugin.artifacts"
          :busy="busy"
          @edit="$emit('edit', $event)"
          @enable="$emit('enable', $event)"
          @disable="$emit('disable', $event)"
          @delete="$emit('deleteConfiguration', $event)"
        />
      </template>
      <PluginVersionsPanel v-else :plugin="plugin" :busy="busy" @accept="$emit('accept', $event)" @switch-version="$emit('switchVersion', $event)" @delete-version="$emit('deleteVersion', $event)" />
    </div>
    <template #footer>
      <ZButton v-if="plugin && plugin.artifacts.every(artifact => artifact.source.kind !== 'builtin')" type="danger" variant="plain" :disabled="busy" @click="$emit('uninstall', plugin)">
        卸载插件
      </ZButton>
      <ZButton :disabled="busy" @click="open = false">
        关闭
      </ZButton>
    </template>
  </ZDialog>
</template>
