type PresetVisualTone = 'blue' | 'cyan' | 'green' | 'orange'

const presetVisualTones: readonly PresetVisualTone[] = ['blue', 'green', 'orange', 'cyan']
const presetVisualToneClasses = {
  blue: 'bg-z-blue-container-strong text-z-blue-on-container',
  cyan: 'bg-z-cyan-container-strong text-z-cyan-on-container',
  green: 'bg-z-green-container-strong text-z-green-on-container',
  orange: 'bg-z-orange-container-strong text-z-orange-on-container',
} as const satisfies Record<PresetVisualTone, string>

export function stableVisualIndex(value: unknown, length: number): number {
  if (length <= 0)
    return 0

  const text = String(value ?? '')
  let hash = 0
  for (const character of text)
    hash += character.codePointAt(0) ?? 0

  return hash % length
}

/** 为无状态含义的身份与分类生成稳定色调，不把颜色误解为成功或警告。 */
export function stablePresetVisualToneClass(value: unknown): string {
  const tone = presetVisualTones[stableVisualIndex(value, presetVisualTones.length)]!
  return presetVisualToneClasses[tone]
}
