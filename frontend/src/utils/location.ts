import type { RequestLocation } from '@/api'

export function normalizeRequestLocation(location: RequestLocation): RequestLocation {
  return {
    country: location.country.trim().toUpperCase(),
    region: location.region.trim(),
    city: location.city.trim(),
    timezone: location.timezone.trim(),
  }
}
