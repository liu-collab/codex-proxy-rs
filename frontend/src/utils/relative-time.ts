import type { RelativeTimeView } from '@/api/time'
import { t } from '@/i18n'

export function formatRelativeTime(value: RelativeTimeView | null | undefined) {
  if (value == null)
    return '—'
  if (value.kind === 'absolute')
    return value.value
  if (value.kind === 'just_now')
    return t('common.time.justNow')

  switch (value.unit) {
    case 'minute': return t('common.time.minutesAgo', { count: value.value }, value.value)
    case 'hour': return t('common.time.hoursAgo', { count: value.value }, value.value)
    case 'day': return t('common.time.daysAgo', { count: value.value }, value.value)
  }
}
