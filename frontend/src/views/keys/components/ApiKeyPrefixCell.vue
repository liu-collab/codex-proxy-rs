<script setup lang="ts">
import { ZIconButton } from '@codex-proxy/ui'

import { Copy } from '@lucide/vue'

import { useI18n } from 'vue-i18n'

defineProps<{
  prefix: string
  revealing: boolean
  copying: boolean
}>()

const emit = defineEmits<{
  copy: []
}>()

const { t } = useI18n({ useScope: 'global' })
</script>

<template>
  <div class="flex items-center gap-2">
    <code class="block min-w-0 overflow-hidden whitespace-nowrap font-mono text-cp-sm font-emphasis text-cp-text">
      {{ prefix }}••••••••••••••••
    </code>
    <ZIconButton
      size="small"
      :aria-label="t('keys.fields.copyFullKey')"
      :loading="copying"
      :disabled="revealing"
      @click="emit('copy')"
    >
      <Copy class="size-3.5" />
    </ZIconButton>
  </div>
</template>
