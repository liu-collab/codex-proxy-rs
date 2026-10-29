<script setup lang="ts">
import { ZButton, ZCard, ZCheckbox, ZForm, ZFormItem, ZInput } from '@codex-proxy/ui'

import { CalendarClock, Save } from '@lucide/vue'

import { useI18n } from 'vue-i18n'

defineProps<{
  disabled: boolean
  saving: boolean
  storageReady: boolean
}>()

const emit = defineEmits<{
  save: []
}>()

const { t } = useI18n({ useScope: 'global' })

interface ScheduleForm {
  scheduleEnabled: boolean
  cronExpression: string
  retentionDays: string
  retentionCount: string
}

const schedule = defineModel<ScheduleForm>('schedule', { required: true })

function updateSchedule<Key extends keyof ScheduleForm>(key: Key, value: ScheduleForm[Key]) {
  schedule.value = { ...schedule.value, [key]: value }
}
</script>

<template>
  <ZCard
    :title="t('settings.backup.schedule.backupPlan')"
    :description="t('settings.backup.schedule.description')"
  >
    <template #actions>
      <ZButton type="primary" :loading="saving" :disabled="disabled" @click="emit('save')">
        <template #icon>
          <Save class="size-4" />
        </template>
        {{ saving ? t('settings.backup.schedule.saving') : t('common.actions.save') }}
      </ZButton>
    </template>

    <div class="@container">
      <ZForm class="max-w-6xl @min-[640px]:grid-cols-2 @min-[640px]:items-end">
        <div class="col-span-2 flex items-center gap-4 @max-[640px]:col-span-1">
          <ZCheckbox
            :model-value="schedule.scheduleEnabled" :disabled="disabled || !storageReady"
            :label="t('settings.backup.schedule.enableScheduledBackup')"
            @update:model-value="updateSchedule('scheduleEnabled', $event)"
          />
        </div>

        <ZFormItem
          :label="t('settings.backup.schedule.cronExpression')"
          :description="t('settings.backup.schedule.cronHelp')"
        >
          <ZInput :model-value="schedule.cronExpression" :disabled="disabled" :aria-label="t('settings.backup.schedule.cronExpression')" @update:model-value="updateSchedule('cronExpression', $event)">
            <template #prefix>
              <CalendarClock class="size-4" />
            </template>
          </ZInput>
        </ZFormItem>

        <ZFormItem :label="t('settings.backup.schedule.retentionDaysLabel')" :description="t('settings.backup.schedule.retentionDaysHelp')">
          <ZInput :model-value="schedule.retentionDays" :disabled="disabled" :aria-label="t('settings.backup.schedule.retentionDaysLabel')" type="number" min="0" @update:model-value="updateSchedule('retentionDays', $event)" />
        </ZFormItem>

        <ZFormItem
          :label="t('settings.backup.schedule.maxBackupsLabel')"
          :description="t('settings.backup.schedule.retentionCountHelp')"
        >
          <ZInput :model-value="schedule.retentionCount" :disabled="disabled" :aria-label="t('settings.backup.schedule.maxBackupsLabel')" type="number" min="0" @update:model-value="updateSchedule('retentionCount', $event)" />
        </ZFormItem>
      </ZForm>
    </div>
  </ZCard>
</template>
