<script setup lang="ts">
import type { BackupStatus } from '@/api'

import { computed } from 'vue'

import { useI18n } from 'vue-i18n'

const props = withDefaults(
  defineProps<{
    status: BackupStatus
    variant?: 'inline' | 'pill'
  }>(),
  {
    variant: 'inline',
  },
)

const { t } = useI18n({ useScope: 'global' })

const STATUS_LABELS = computed<Record<BackupStatus, string>>(() => ({
  queued: t('settings.backup.status.queuing'),
  dumping: t('settings.backup.status.exporting'),
  uploading: t('settings.backup.status.uploading'),
  completed: t('common.status.completed'),
  failed: t('common.status.failed'),
  deleting: t('settings.backup.status.deleting'),
}))

const STATUS_TONES: Record<BackupStatus, 'success' | 'danger' | 'warning' | 'info' | 'neutral'> = {
  queued: 'info',
  dumping: 'info',
  uploading: 'info',
  completed: 'success',
  failed: 'danger',
  deleting: 'warning',
}

const tone = computed(() => STATUS_TONES[props.status])
const label = computed(() => STATUS_LABELS.value[props.status] || props.status)

const textClass = computed(() => {
  switch (tone.value) {
    case 'success':
      return 'text-z-success-text'
    case 'danger':
      return 'text-z-error-text'
    case 'warning':
      return 'text-z-warning-text'
    case 'info':
      return 'text-z-info-text'
    default:
      return 'text-z-text-secondary'
  }
})

const dotClass = computed(() => {
  switch (tone.value) {
    case 'success':
      return 'bg-z-success'
    case 'danger':
      return 'bg-z-error'
    case 'warning':
      return 'bg-z-warning'
    case 'info':
      return 'bg-z-info'
    default:
      return 'bg-z-text-quaternary'
  }
})
</script>

<template>
  <span
    v-if="variant === 'pill'"
    class="inline-flex h-7 shrink-0 items-center rounded-full px-2.5 text-z-sm font-heavy"
    :class="textClass"
  >
    {{ label }}
  </span>
  <span
    v-else
    class="inline-flex min-w-16 items-center gap-1.5 text-z-sm leading-none font-emphasis"
    :class="textClass"
  >
    <span class="size-1.5 rounded-full" :class="dotClass" />
    <span>{{ label }}</span>
  </span>
</template>
