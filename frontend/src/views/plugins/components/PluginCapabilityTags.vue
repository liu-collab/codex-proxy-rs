<script setup lang="ts">
import type { PluginArtifactMetadata } from '@/api'

import { ZPopover, ZTag } from '@codex-proxy/ui'
import { useResizeObserver } from '@vueuse/core'
import { computed, shallowRef, useId, useTemplateRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { pluginCapabilityLabel } from '../utils/model'

const props = withDefaults(defineProps<{
  metadata: PluginArtifactMetadata
  primary?: boolean
}>(), { primary: false })

const { t } = useI18n({ useScope: 'global' })

const open = shallowRef(false)
const contentId = useId()
const container = useTemplateRef<HTMLDivElement>('container')
const measurement = useTemplateRef<HTMLDivElement>('measurement')
const visibleCount = shallowRef(0)
const entries = computed(() => Object.entries(props.metadata.contributes).map(([capability, contribution]) => ({
  capability,
  id: contribution.id,
  label: pluginCapabilityLabel(capability),
})))
const visible = computed(() => entries.value.slice(0, visibleCount.value))
const hidden = computed(() => entries.value.slice(visibleCount.value))

function updateVisibleCount() {
  if (!container.value || !measurement.value)
    return

  // 用完整标签测量，避免折叠结果反过来改变可用宽度的计算
  const widths = Array.from(measurement.value.children, child => child.getBoundingClientRect().width)
  const overflowWidth = widths.pop() ?? 0
  const gap = Number.parseFloat(getComputedStyle(measurement.value).columnGap) || 0
  const available = container.value.getBoundingClientRect().width
  const total = widths.reduce((sum, width) => sum + width, 0) + Math.max(0, widths.length - 1) * gap
  if (total <= available) {
    visibleCount.value = widths.length
    return
  }

  let used = overflowWidth
  let count = 0
  for (const width of widths) {
    if (used + gap + width > available)
      break
    used += gap + width
    count++
  }
  visibleCount.value = count
}

useResizeObserver([container, measurement], updateVisibleCount)
watch(entries, updateVisibleCount, { flush: 'post' })
watch(() => hidden.value.length, (count) => {
  if (!count)
    open.value = false
})
</script>

<template>
  <div ref="container" class="relative flex min-w-0 items-center gap-1 whitespace-nowrap">
    <div aria-hidden="true" inert class="pointer-events-none invisible absolute inset-0 overflow-hidden">
      <div ref="measurement" class="flex w-max items-center gap-1 whitespace-nowrap">
        <ZTag v-for="entry in entries" :key="entry.capability" size="small" :type="primary ? 'primary' : 'default'" class="shrink-0">
          {{ entry.label }}
        </ZTag>
        <ZTag size="small" :type="primary ? 'primary' : 'default'" class="shrink-0 tabular-nums">
          +{{ entries.length }}
        </ZTag>
      </div>
    </div>
    <ZTag v-for="entry in visible" :key="entry.capability" size="small" :type="primary ? 'primary' : 'default'" class="shrink-0">
      <span :title="`${entry.label} · ${entry.id}`">{{ entry.label }}</span>
    </ZTag>
    <ZPopover v-if="hidden.length" v-model="open" trigger="hover-click" placement="top" class="shrink-0">
      <template #reference>
        <button
          type="button"
          class="cursor-pointer rounded-cp border-0 bg-transparent p-0 outline-none focus-visible:ring-2 focus-visible:ring-cp-control-outline"
          :aria-label="t('plugins.capabilities.moreCapabilities', { count: hidden.length })"
          :aria-expanded="open"
          :aria-describedby="open ? contentId : undefined"
        >
          <ZTag size="small" :type="primary ? 'primary' : 'default'" class="tabular-nums">
            +{{ hidden.length }}
          </ZTag>
        </button>
      </template>
      <div :id="contentId" role="tooltip" class="flex max-w-72 flex-wrap gap-1.5 p-3">
        <ZTag v-for="entry in hidden" :key="entry.capability" size="small" :type="primary ? 'primary' : 'default'">
          <span :title="entry.id">{{ entry.label }}</span>
        </ZTag>
      </div>
    </ZPopover>
    <span v-if="!entries.length" class="text-cp-text-quaternary">{{ t('common.values.none') }}</span>
  </div>
</template>
