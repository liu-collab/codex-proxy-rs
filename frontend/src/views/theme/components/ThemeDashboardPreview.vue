<script setup lang="ts">
import type { DashboardTrendKind } from '@/api/modules/dashboard'

import { computed, shallowRef } from 'vue'
import { useI18n } from 'vue-i18n'

import DashboardContent from '@/views/dashboard/components/DashboardContent.vue'
import {
  dashboardSnapshotView,
  dashboardTrendView,
} from '@/views/dashboard/presenter'

import { themeDashboardSummary } from '../fixtures/dashboard'

const { t } = useI18n({ useScope: 'global' })

const snapshot = computed(() => dashboardSnapshotView(themeDashboardSummary.value))
const trend = computed(() => dashboardTrendView(themeDashboardSummary.value.trend))
const trendKind = shallowRef<DashboardTrendKind>('usage')
</script>

<template>
  <DashboardContent
    v-model:trend-kind="trendKind"
    :last-refreshed-at="t('appearance.preview.dashboard.justUpdated')"
    :metrics="snapshot.metrics"
    :analytics="snapshot.analytics"
    :trend-points="trend.points"
    :trend-summary="trend.summary"
    :health-timeline="snapshot.healthTimeline"
    :account-usage="snapshot.accountUsage"
    :wire-profiles="snapshot.wireProfiles"
    :usage-records="snapshot.usageRecords"
    :pool-summary="snapshot.poolSummary"
    :capacity-info="snapshot.capacityInfo"
    :rotation-strategy="snapshot.rotationStrategy"
  />
</template>
