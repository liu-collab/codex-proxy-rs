<script setup lang="ts">
import { ZPopover } from '@codex-proxy/ui'

import { Info } from '@lucide/vue'

// 表格单元格的“信息点”悬浮明细：统一触发按钮与面板壳。
defineProps<{
  title?: string
  triggerLabel: string
  tone?: 'primary' | 'warning'
}>()
</script>

<template>
  <ZPopover
    trigger="hover"
    placement="right"
  >
    <template #reference>
      <button
        type="button"
        class="inline-flex size-4 items-center justify-center rounded-full bg-transparent outline-none focus-visible:ring-2 focus-visible:ring-z-control-outline"
        :class="tone === 'warning'
          ? 'text-z-warning-text'
          : 'text-z-primary-text'"
        :aria-label="triggerLabel"
      >
        <Info class="size-3" />
      </button>
    </template>

    <div class="grid w-max min-w-60 max-w-[calc(100vw-2rem)] gap-2 p-3 text-z-sm leading-none">
      <p v-if="title" class="m-0 font-heavy text-z-text">
        {{ title }}
      </p>
      <slot />
    </div>
  </ZPopover>
</template>
