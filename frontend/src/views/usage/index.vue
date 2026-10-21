<script setup lang="ts">
import type { TableColumn } from '@codex-proxy/ui'
import type { UsageListRecord, UsageRecordStatus } from '@/api'
import { useTableColumns, ZCard, ZDatePicker, ZIconButton, ZPageHeader, ZPagination, ZTableColumnSettings } from '@codex-proxy/ui'

import { Eye } from '@lucide/vue'
import { computed, shallowRef, watch } from 'vue'
import ProviderFilter from '@/components/ProviderFilter.vue'
import { usageRecordColumns } from '@/components/usage/shared/columns'
import UsageRecordsTable from '@/components/usage/UsageRecordsTable.vue'
import UsageRecordStatusFilter from '@/components/usage/UsageRecordStatusFilter.vue'
import UsageErrorDetailModal from './components/UsageErrorDetailModal.vue'
import UsageFilters from './components/UsageFilters.vue'
import UsageInsightsGrid from './components/UsageInsightsGrid.vue'
import UsageRecordDetailModal from './components/UsageRecordDetailModal.vue'
import UsageSummaryCards from './components/UsageSummaryCards.vue'
import { useUsageRecordDetail } from './composables/useUsageRecordDetail'
import { useUsageRecordsTable } from './composables/useUsageRecordsTable'
import { useUsageTimeRange } from './composables/useUsageTimeRange'
import { isErrorUsageRecord } from './utils/detail'

const recordView = shallowRef<UsageRecordStatus>('success')
const recordColumns = computed<TableColumn<UsageListRecord>[]>(() => recordView.value === 'error'
  ? usageRecordColumns.flatMap(column => column.key === 'route'
      ? [column, { key: 'errorMessage', label: '错误', kind: 'custom', size: '4xl', hideable: false }]
      : [column])
  : usageRecordColumns)
const { visibleColumns, columnOptions, setColumnVisible, setColumnOrder, resetColumns } = useTableColumns(recordColumns, 'usage-records')
const { timeRange, timeRangeParams, disabledDate, refreshTimeRangeEnd, latestTimeRangeParams }
  = useUsageTimeRange()

const {
  searchQuery,
  providerQuery,
  usagePagination,
  loading,
  error,
  analyticsLoading,
  records,
  summary,
  insights,
  refreshingList,
  diagnosticDimension,
  loadUsageRecords,
  refreshUsageRecords,
  handlePageChange,
  handlePageSizeChange,
} = useUsageRecordsTable({
  timeRangeParams,
  latestTimeRangeParams,
  status: recordView,
})

const { showDetailModal, selectedUsageRecord, handleViewDetail } = useUsageRecordDetail()
const detailComponent = computed(() => isErrorUsageRecord(selectedUsageRecord.value)
  ? UsageErrorDetailModal
  : UsageRecordDetailModal)

watch(timeRange, () => {
  refreshTimeRangeEnd()
  void loadUsageRecords()
})
</script>

<template>
  <div class="w-full">
    <ZPageHeader title="使用统计" description="查看请求用量、性能趋势与调用错误记录">
      <template #actions>
        <ZDatePicker
          v-model="timeRange"
          type="daterange"
          :clearable="false"
          :editable="false"
          :disabled-date="disabledDate"
          single-panel
          aria-label="统计日期范围，仅可选择最近 31 天"
          class="w-60 max-w-full"
        />
        <ProviderFilter
          v-model="providerQuery"
          :disabled="refreshingList"
          class="shrink-0"
        />
      </template>
    </ZPageHeader>

    <UsageSummaryCards :summary="summary" />
    <UsageInsightsGrid
      v-model:diagnostic-dimension="diagnosticDimension"
      :overview="insights.overview"
      :diagnostics="insights.diagnostics"
      :loading="analyticsLoading"
    />

    <ZCard
      class="mt-5 flex flex-col"
    >
      <template #header>
        <div>
          <h2 class="m-0 text-xl leading-[1.15] font-heavy text-cp-text">
            请求明细
          </h2>
          <p
            class="mt-1.75 mb-0 text-cp leading-[1.15] font-emphasis text-cp-text-secondary"
          >
            成功请求与失败请求明细
          </p>
        </div>
      </template>

      <template #body>
        <div
          class="grid min-h-130 min-w-0 flex-1 grid-rows-[auto_minmax(0,1fr)] gap-3"
        >
          <UsageFilters
            v-model:search="searchQuery"
            :loading="loading"
            :refreshing="refreshingList"
            @refresh="refreshUsageRecords"
          >
            <template #actions>
              <UsageRecordStatusFilter v-model="recordView" />
              <ZTableColumnSettings
                :options="columnOptions"
                @change="setColumnVisible"
                @reorder="setColumnOrder"
                @reset="resetColumns"
              />
            </template>
          </UsageFilters>

          <div class="flex min-h-0 min-w-0 flex-col">
            <UsageRecordsTable
              class="min-h-0 flex-1"
              :columns="visibleColumns"
              :rows="records"
              :loading="loading"
              :empty-text="error ? `加载失败：${error}` : '所选条件下暂无请求记录'"
            >
              <template #actions="{ row }">
                <div class="flex items-center justify-start">
                  <ZIconButton
                    size="small"
                    :aria-label="isErrorUsageRecord(row) ? '查看错误详情' : '查看使用记录详情'"
                    @click="handleViewDetail(row)"
                  >
                    <Eye class="size-3.5" />
                  </ZIconButton>
                </div>
              </template>
            </UsageRecordsTable>
            <ZPagination
              v-bind="usagePagination"
              :disabled="loading"
              @current-change="handlePageChange"
              @size-change="handlePageSizeChange"
            />
          </div>
        </div>
      </template>
    </ZCard>

    <component :is="detailComponent" v-model="showDetailModal" :record="selectedUsageRecord" />
  </div>
</template>
