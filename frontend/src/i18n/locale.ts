export type Locale = 'en' | 'zh-Hans' | 'zh-Hant'

export const localeOptions = [
  { label: '简体中文', value: 'zh-Hans' },
  { label: '繁體中文', value: 'zh-Hant' },
  { label: 'English', value: 'en' },
] satisfies { label: string, value: Locale }[]

const storageKey = 'codex-proxy-rs-locale'

export function isLocale(value: unknown): value is Locale {
  return localeOptions.some(option => option.value === value)
}

export function readInitialLocale(): Locale {
  try {
    const saved = localStorage.getItem(storageKey)
    if (isLocale(saved))
      return saved
  }
  catch {
    // 存储不可用时使用默认语言
  }
  return 'zh-Hans'
}

export function applyLocale(value: string): void {
  document.documentElement.lang = value
  try {
    localStorage.setItem(storageKey, value)
  }
  catch {
    // 存储不可用不影响当前会话的语言切换
  }
}
