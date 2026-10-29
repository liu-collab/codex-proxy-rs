<script setup lang="ts">
import { Minimize2 } from '@lucide/vue'

import { useI18n } from 'vue-i18n'

defineProps<{ record: { route: string | null, compact?: boolean } }>()

const { t } = useI18n({ useScope: 'global' })
</script>

<template>
  <div class="inline-flex max-w-full items-center gap-1.5 whitespace-nowrap">
    <code class="font-mono text-cp-sm font-emphasis">{{ record.route || '—' }}</code>
    <span
      v-if="record.compact"
      class="inline-flex shrink-0 text-cp-orange-text"
      :title="t('usage.route.compressionRequest')"
      :aria-label="t('usage.route.compressionRequest')"
    >
      <Minimize2 class="size-3.5" stroke-width="2.4" />
    </span>
  </div>
</template>
