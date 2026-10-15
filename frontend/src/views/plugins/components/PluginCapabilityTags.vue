<script setup lang="ts">
import type { PluginArtifactMetadata } from '@/api'
import { ZPopover, ZTag } from '@codex-proxy/ui'
import { computed, shallowRef, useId } from 'vue'
import { pluginCapabilityLabel } from '../utils/model'

const props = withDefaults(defineProps<{
  metadata: PluginArtifactMetadata
  primary?: boolean
}>(), { primary: false })

const open = shallowRef(false)
const contentId = useId()
const entries = computed(() => Object.entries(props.metadata.contributes))
const visible = computed(() => entries.value.slice(0, 2))
const hidden = computed(() => entries.value.slice(2))
</script>

<template>
  <div class="flex min-w-0 items-center gap-1 whitespace-nowrap">
    <ZTag v-for="[capability, contribution] in visible" :key="capability" size="small" :type="primary ? 'primary' : 'default'" class="min-w-0">
      <span class="block truncate" :title="`${pluginCapabilityLabel(capability)} · ${contribution.id}`">{{ pluginCapabilityLabel(capability) }}</span>
    </ZTag>
    <ZPopover v-if="hidden.length" v-model="open" trigger="hover-click" placement="top" class="shrink-0">
      <template #reference>
        <button
          type="button"
          class="cursor-pointer rounded-cp border-0 bg-transparent p-0 outline-none focus-visible:ring-2 focus-visible:ring-cp-control-outline"
          :aria-label="`另有 ${hidden.length} 项声明能力`"
          :aria-expanded="open"
          :aria-describedby="open ? contentId : undefined"
        >
          <ZTag size="small" :type="primary ? 'primary' : 'default'">
            +{{ hidden.length }}
          </ZTag>
        </button>
      </template>
      <div :id="contentId" role="tooltip" class="flex max-w-72 flex-wrap gap-1.5 p-3">
        <ZTag v-for="[capability, contribution] in hidden" :key="capability" size="small" :type="primary ? 'primary' : 'default'">
          <span :title="contribution.id">{{ pluginCapabilityLabel(capability) }}</span>
        </ZTag>
      </div>
    </ZPopover>
    <span v-if="!entries.length" class="text-cp-text-quaternary">无</span>
  </div>
</template>
