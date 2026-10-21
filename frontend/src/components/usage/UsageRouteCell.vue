<script setup lang="ts">
import { Minimize2 } from '@lucide/vue'

defineProps<{ record: { route: string | null, compact?: boolean } }>()
</script>

<template>
  <div class="inline-flex max-w-full items-center gap-1.5 whitespace-nowrap">
    <code class="font-mono text-cp-sm font-emphasis">{{ record.route || '—' }}</code>
    <span
      v-if="record.compact"
      class="inline-flex shrink-0 text-cp-orange-text"
      title="压缩请求"
      aria-label="压缩请求"
    >
      <Minimize2 class="size-3.5" stroke-width="2.4" />
    </span>
  </div>
</template>
