import { computed } from 'vue'

import { useThemeColor } from '@/composables/useThemeColor'

export function useChartPalette() {
  const color = useThemeColor()
  const palette = computed(() => ({
    textPrimary: color('color-text', '#0E1726'),
    textSecondary: color('color-text-secondary', '#64748B'),
    textMuted: color('color-text-quaternary', '#94A3B8'),
    surface: color('color-bg-container', '#FFFFFF'),
    surfaceMuted: color('color-fill-tertiary', '#F1F5F9'),
    grid: color('color-fill-tertiary', '#F1F5F9'),
    divider: color('color-split', '#E2E8F0'),
    border: color('color-border-secondary', '#E2E8F0'),
    pointer: color('color-border', '#CBD5E1'),
    info: color('color-blue-solid', '#5983F4'),
    success: color('color-green-solid', '#12B981'),
    warning: color('color-orange-solid', '#F59E0B'),
    danger: color('color-red-solid', '#EF4444'),
    normal: color('color-text-tertiary', '#94A3B8'),
    reasoning: color('color-purple-solid', '#722ED1'),
  }))

  return { color, palette }
}
