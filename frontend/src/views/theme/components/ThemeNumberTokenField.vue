<script setup lang="ts">
import { ZIconButton, ZInputNumber, ZSlider } from '@codex-proxy/ui'

import { RotateCcw } from '@lucide/vue'

defineProps<{
  label: string
  token: string
  value: number
  min: number
  max: number
  step?: number
  unit?: string
  overridden?: boolean
}>()

const emit = defineEmits<{
  change: [value: number]
  reset: []
}>()
</script>

<template>
  <div class="grid gap-3 rounded-cp-lg bg-cp-fill-quaternary px-3 py-3">
    <div class="flex items-start justify-between gap-3">
      <div class="min-w-0">
        <div class="flex items-center gap-2">
          <strong class="text-cp-sm font-bold text-cp-text">{{ label }}</strong>
          <span v-if="overridden" class="rounded-full bg-cp-primary-container px-1.5 py-0.5 text-[9px] font-heavy text-cp-primary-on-container">已修改</span>
        </div>
        <code class="mt-1 block truncate font-mono text-[10px] text-cp-text-quaternary">{{ token }}</code>
      </div>
      <div class="flex items-center gap-1">
        <ZInputNumber
          size="small"
          :model-value="value"
          :aria-label="label"
          :min="min"
          :max="max"
          :step="step ?? 1"
          :unit="unit ?? 'px'"
          @update:model-value="emit('change', $event)"
        />
        <ZIconButton
          v-if="overridden"
          aria-label="恢复派生值"
          size="small"
          @click="emit('reset')"
        >
          <RotateCcw class="size-3.5" />
        </ZIconButton>
      </div>
    </div>
    <ZSlider
      :model-value="value"
      :min="min"
      :max="max"
      :step="step ?? 1"
      :aria-label="label"
      :unit="unit ?? 'px'"
      @update:model-value="emit('change', $event)"
    />
  </div>
</template>
