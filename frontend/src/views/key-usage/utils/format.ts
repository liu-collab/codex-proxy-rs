import { t } from '@/i18n'

export function money(value: string | number | null) {
  if (value === null)
    return t('usage.metrics.notPriced')
  const amount = Number(value)
  if (!Number.isFinite(amount))
    return '—'
  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: 'USD',
    minimumFractionDigits: 2,
    maximumFractionDigits: 6,
  }).format(amount)
}
