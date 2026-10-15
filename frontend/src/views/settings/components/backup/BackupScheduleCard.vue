<script setup lang="ts">
import { ZButton, ZCard, ZCheckbox, ZForm, ZFormItem, ZInput } from '@codex-proxy/ui'

import { CalendarClock, Save } from '@lucide/vue'

interface ScheduleForm {
  scheduleEnabled: boolean
  cronExpression: string
  retentionDays: string
  retentionCount: string
}

defineProps<{
  disabled: boolean
  saving: boolean
  storageReady: boolean
}>()

const emit = defineEmits<{
  save: []
}>()

const schedule = defineModel<ScheduleForm>('schedule', { required: true })

function updateSchedule<Key extends keyof ScheduleForm>(key: Key, value: ScheduleForm[Key]) {
  schedule.value = { ...schedule.value, [key]: value }
}
</script>

<template>
  <ZCard
    title="备份计划"
    description="配置自动备份的执行时间与保留策略"
  >
    <template #actions>
      <ZButton type="primary" :loading="saving" :disabled="disabled" @click="emit('save')">
        <template #icon>
          <Save class="size-4" />
        </template>
        {{ saving ? '保存中...' : '保存' }}
      </ZButton>
    </template>

    <div class="@container">
      <ZForm class="max-w-6xl @min-[640px]:grid-cols-2">
        <div class="col-span-2 flex items-center gap-4 @max-[640px]:col-span-1">
          <ZCheckbox
            :model-value="schedule.scheduleEnabled" :disabled="disabled || !storageReady"
            label="启用定时备份"
            @update:model-value="updateSchedule('scheduleEnabled', $event)"
          />
        </div>

        <ZFormItem
          label="Cron 表达式"
          description="5 段格式，例如 0 2 * * * 表示每天凌晨 2 点"
        >
          <ZInput :model-value="schedule.cronExpression" :disabled="disabled" aria-label="Cron 表达式" @update:model-value="updateSchedule('cronExpression', $event)">
            <template #prefix>
              <CalendarClock class="size-4" />
            </template>
          </ZInput>
        </ZFormItem>

        <ZFormItem label="保留天数" description="超过此天数自动删除，0 表示不按天数清理，仍受最大保留份数限制">
          <ZInput :model-value="schedule.retentionDays" :disabled="disabled" aria-label="备份保留天数" type="number" min="0" @update:model-value="updateSchedule('retentionDays', $event)" />
        </ZFormItem>

        <ZFormItem
          label="最大保留份数"
          description="最多保留的备份数量，0 表示不按份数清理，仍受保留天数限制"
        >
          <ZInput :model-value="schedule.retentionCount" :disabled="disabled" aria-label="最大保留份数" type="number" min="0" @update:model-value="updateSchedule('retentionCount', $event)" />
        </ZFormItem>
      </ZForm>
    </div>
  </ZCard>
</template>
