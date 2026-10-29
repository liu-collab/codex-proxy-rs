import { useI18n } from 'vue-i18n'
import { setLocale } from '@/i18n'
import { localeOptions } from '@/i18n/locale'

export function useLocale() {
  const { locale } = useI18n({ useScope: 'global' })

  return { locale, localeOptions, setLocale }
}
