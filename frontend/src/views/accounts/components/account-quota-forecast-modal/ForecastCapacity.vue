<script setup lang="ts">
import type { AccountQuotaForecast } from '@/api'

import { ArrowRight } from '@lucide/vue'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

const props = defineProps<{ forecast: AccountQuotaForecast }>()

const { t } = useI18n({ useScope: 'global' })

const source = computed(() => props.forecast.source)
const metrics = computed(() => [
  {
    label: t('accounts.quotaForecast.tokenCapacity'),
    observed: source.value?.tokensDisplay ?? '—',
    estimated: props.forecast.estimatedTokensDisplay,
    tone: 'text-cp-cyan-text',
  },
  {
    label: t('accounts.quotaForecast.equivalentFeeUsd'),
    observed: source.value?.usdDisplay ?? '—',
    estimated: props.forecast.estimatedUsdDisplay,
    tone: 'text-cp-green-text',
  },
])
</script>

<template>
  <section v-if="source" :aria-label="t('accounts.quotaForecast.capacityForecast')" class="flex flex-col gap-5 rounded-cp-card bg-cp-fill-tertiary/70 p-4 [html[data-theme=light]_&]:bg-cp-fill-quaternary/70">
    <div>
      <div class="flex flex-wrap items-center justify-between gap-2">
        <span class="text-cp-xs text-cp-text-secondary">{{ t('accounts.quotaForecast.usedThisCycle') }}</span>
        <span class="inline-flex items-center rounded-cp-sm bg-cp-info-container px-2 py-1 text-cp-xs font-emphasis text-cp-info-on-container">
          {{ forecast.lowSample ? t('accounts.quotaForecast.preliminary') : t('accounts.quotaForecast.estimateLabel') }}
        </span>
      </div>
      <div class="mt-2 flex items-end justify-between gap-4">
        <strong class="font-mono text-3xl font-heavy tabular-nums text-cp-text [html[data-theme=light]_&]:font-emphasis">{{ source.usedPercentDisplay }}</strong>
        <span class="pb-1 text-cp-xs text-cp-text-secondary">{{ t('accounts.quotaForecast.thisCycle') }} <span class="font-mono">100%</span></span>
      </div>

      <!-- 实心为已观测比例，虚线为待推算容量，不伪造历史趋势或置信区间。 -->
      <div
        class="relative mt-3 h-3 overflow-hidden rounded-full bg-cp-bg-container"
        role="img"
        :aria-label="t('accounts.quotaForecast.usedDescription', { rate: source.usedPercentDisplay })"
      >
        <span class="absolute inset-x-0 top-1/2 border-t-2 border-dashed border-cp-primary/50" />
        <span class="absolute inset-y-0 left-0 rounded-full bg-cp-primary" :style="{ width: `${source.usedPercent ?? 0}%` }" />
      </div>
      <div class="mt-2 flex justify-between font-mono text-[10px] text-cp-text-tertiary" aria-hidden="true">
        <span>0</span><span>25</span><span>50</span><span>75</span><span>100%</span>
      </div>
    </div>

    <div class="grid gap-2">
      <div class="flex flex-wrap justify-between gap-x-4 gap-y-1 text-cp-xs text-cp-text-secondary">
        <span>{{ t('accounts.quotaForecast.alreadyRecordedThisCycle') }}</span>
        <span>{{ forecast.extrapolated ? t('accounts.quotaForecast.equivalentCapacityOverDays', { targetDays: forecast.targetDays }) : t('accounts.quotaForecast.projectedTotal') }}</span>
      </div>
      <div class="grid gap-2">
        <div v-for="metric in metrics" :key="metric.label" class="rounded-cp bg-cp-bg-container px-3 py-3">
          <p class="m-0 text-cp-xs text-cp-text-secondary">
            {{ metric.label }}
          </p>
          <div class="mt-2 grid grid-cols-[1fr_20px_1.2fr] items-center gap-2">
            <span class="font-mono text-cp-sm font-emphasis tabular-nums text-cp-text [html[data-theme=light]_&]:font-medium">{{ metric.observed }}</span>
            <ArrowRight class="size-3.5 text-cp-text-tertiary" />
            <strong class="text-right font-mono text-xl font-heavy tabular-nums" :class="metric.tone">
              <span v-if="metric.estimated !== '—'" class="mr-1 text-cp-sm font-normal text-cp-text-tertiary">≈</span>{{ metric.estimated }}
            </strong>
          </div>
        </div>
      </div>
    </div>
  </section>
</template>
