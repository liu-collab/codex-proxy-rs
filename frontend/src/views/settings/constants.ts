import { computed } from 'vue'
import { t } from '@/i18n'

export const rotationOptions = computed(() => ([
  {
    label: t('settings.rotation.smartLabel'),
    value: 'smart',
    description: t('settings.rotation.smartDescription'),
  },

  {
    label: t('settings.rotation.quotaResetLabel'),
    value: 'quota_reset_priority',
    description: t('settings.rotation.quotaResetDescription'),
  },
  {
    label: t('settings.rotation.roundRobinLabel'),
    value: 'round_robin',
    description: t('settings.rotation.roundRobinDescription'),
  },
  {
    label: t('settings.rotation.stickyLabel'),
    value: 'sticky',
    description: t('settings.rotation.stickyDescription'),
  },
] as const))
