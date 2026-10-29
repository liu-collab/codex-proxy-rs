<script setup lang="ts">
import type { UsageListRecord } from '@/api'

import { computed } from 'vue'

import { useI18n } from 'vue-i18n'
import UsageDetailPopover from '@/components/usage/UsageDetailPopover.vue'
import { usageLatencyDetails } from './shared/presenter'

const props = defineProps<{
  record: Pick<UsageListRecord, 'latencyDetails' | 'firstTokenLatencyMs' | 'latencyMs'>
}>()

const { t } = useI18n({ useScope: 'global' })

const latencyDetails = computed(() => usageLatencyDetails(props.record))

const latencyTones = {
  good: 'text-cp-success-text',
  warn: 'text-cp-warning-text',
  // 主题的 Orange 与 Warning 共用色种，混入 Error 让相邻两档可区分
  slow: 'latency-slow',
  critical: 'text-cp-error-text',
}
</script>

<template>
  <div class="flex items-center justify-end gap-1.5">
    <div
      class="grid grid-cols-[auto_auto] items-center justify-end gap-x-2 gap-y-1.5 whitespace-nowrap font-mono text-cp-sm leading-none font-heavy tabular-nums"
    >
      <span class="text-cp-xs text-cp-text-quaternary">{{ t('usage.ttft') }}</span>
      <span :class="latencyDetails.firstOutputSeverity === null ? 'text-cp-text-quaternary' : latencyTones[latencyDetails.firstOutputSeverity]">
        {{ latencyDetails.firstOutputDisplay }}
      </span>
      <span class="text-cp-xs text-cp-text-quaternary">{{ t('usage.latency.total') }}</span>
      <span :class="latencyDetails.totalSeverity === null ? 'text-cp-text-quaternary' : latencyTones[latencyDetails.totalSeverity]">
        {{ latencyDetails.totalDisplay }}
      </span>
    </div>

    <UsageDetailPopover :trigger-label="t('usage.latency.viewLatencyDetails')">
      <section
        v-for="(section, index) in latencyDetails.sections"
        :key="section.title"
        class="grid gap-1.5 text-cp-text-secondary"
        :class="{ 'border-t border-cp-split pt-2': index > 0 }"
      >
        <p class="m-0 flex items-center justify-between gap-3 text-cp-sm font-heavy text-cp-text">
          <span>{{ section.title }}</span>
          <span
            class="rounded px-1 py-0.5 text-[10px] leading-none font-normal"
            :class="section.source === 'official'
              ? 'bg-cp-primary-container text-cp-primary-on-container'
              : 'bg-cp-fill-tertiary text-cp-text-tertiary'"
          >
            {{ section.source === 'official' ? t('usage.latency.official') : t('usage.latency.local') }}
          </span>
        </p>
        <div
          v-for="item in section.items"
          :key="item.label"
          class="flex justify-between gap-4 text-cp-xs"
        >
          <span class="whitespace-nowrap">{{ item.label }}</span>
          <span
            class="whitespace-nowrap font-mono font-heavy"
            :class="item.emphasized ? 'text-cp-info-text' : 'text-cp-text'"
          >
            {{ item.value }}
          </span>
        </div>
      </section>
    </UsageDetailPopover>
  </div>
</template>

<style scoped>
.latency-slow {
  color: color-mix(in oklab, var(--cp-color-warning-text) 65%, var(--cp-color-error-text));
}
</style>
