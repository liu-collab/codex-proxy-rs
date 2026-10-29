import type { UiLocale } from '@codex-proxy/ui'
import type { Locale } from './locale'
import { computed } from 'vue'
import { locale } from './index'
import { uiMessages } from './messages'

// 挂载和切换语言均等待资源就绪，组件只读取已加载的语言
export const uiLocale = computed(() => uiMessages[locale.value as Locale] as UiLocale)
