import type { RequestLocation } from '@/api'
import { t } from '@/i18n'

export function normalizeRequestLocation(location: RequestLocation): RequestLocation {
  return {
    country: location.country.trim().toUpperCase(),
    region: location.region.trim(),
    city: location.city.trim(),
    timezone: location.timezone.trim(),
  }
}

export function requestLocationError(location: RequestLocation): string {
  if (!/^[A-Z]{2}$/.test(location.country) || !location.region || !location.city || !location.timezone)
    return t('common.location.required')
  return ''
}
