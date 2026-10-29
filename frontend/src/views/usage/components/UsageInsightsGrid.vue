<script setup lang="ts">
import type {
  getUsageRecordInsightsDiagnostics,
  getUsageRecordInsightsOverview,
} from '@/api'

import { useI18n } from 'vue-i18n'

import UsageCostCard from './UsageCostCard.vue'
import UsageDiagnosticCard from './UsageDiagnosticCard.vue'
import UsageHealthCard from './UsageHealthCard.vue'
import UsagePerformanceCard from './UsagePerformanceCard.vue'

withDefaults(
  defineProps<{
    overview: Omit<Awaited<ReturnType<typeof getUsageRecordInsightsOverview>>, 'analytics'>
    diagnostics: Omit<Awaited<ReturnType<typeof getUsageRecordInsightsDiagnostics>>, 'analytics'>
    loading?: boolean
  }>(),
  {
    loading: false,
  },
)

const { t } = useI18n({ useScope: 'global' })

const diagnosticDimension = defineModel('diagnosticDimension', {
  type: String,
  default: 'model',
})
</script>

<template>
  <section
    class="mt-5 grid grid-cols-1 gap-3 xl:auto-rows-[minmax(24.25rem,auto)] xl:grid-cols-2"
    :aria-label="t('usage.insights.useStatisticalObservations')"
  >
    <UsageHealthCard
      :health="overview.health"
      :granularity="overview.granularity"
      :loading="loading"
    />

    <UsageDiagnosticCard
      v-model:dimension="diagnosticDimension"
      :diagnostics="diagnostics"
      :loading="loading"
    />

    <UsagePerformanceCard
      :performance="overview.performance"
      :activity="overview.health.points"
      :loading="loading"
    />

    <UsageCostCard
      :cost="overview.cost"
      :activity="overview.health.points"
      :loading="loading"
    />
  </section>
</template>
