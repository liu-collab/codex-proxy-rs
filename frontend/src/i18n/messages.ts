import type { UiLocale } from '@codex-proxy/ui'
import type { Locale } from './locale'
import type en from './locales/en'
import { shallowReactive } from 'vue'

export type MessageSchema = typeof en

interface LocaleMessages {
  default: MessageSchema
  uiLocale: UiLocale
}

export const localeLoaders = {
  'en': () => import('./locales/en'),
  'zh-Hans': () => import('./locales/zh-Hans'),
  'zh-Hant': () => import('./locales/zh-Hant'),
} satisfies Record<Locale, () => Promise<LocaleMessages>>

export const uiMessages = shallowReactive<Partial<Record<Locale, UiLocale>>>({})
