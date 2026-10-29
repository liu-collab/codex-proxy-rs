<script setup lang="ts">
import { ZButton, ZCard, ZIconButton, ZInput } from '@codex-proxy/ui'

import { Plus, Trash2 } from '@lucide/vue'

import { useI18n } from 'vue-i18n'

withDefaults(defineProps<{
  mappings: Array<{ requestedModel: string, upstreamModel: string }>
  loading?: boolean
}>(), {
  loading: false,
})

const emit = defineEmits<{
  addMapping: []
  updateMapping: [index: number, key: 'requestedModel' | 'upstreamModel', value: string]
  removeMapping: [index: number]
}>()

const { t } = useI18n({ useScope: 'global' })
</script>

<template>
  <ZCard
    :title="t('settings.modelMapping')"
    :description="t('settings.routing.modelMapHelp')"
  >
    <div class="grid gap-4">
      <div class="flex flex-wrap items-center gap-3">
        <ZButton :disabled="loading" @click="emit('addMapping')">
          <template #icon>
            <Plus class="size-4" />
          </template>
          {{ t('settings.addMapping') }}
        </ZButton>
      </div>

      <div
        v-if="loading"
        class="rounded-cp bg-cp-fill-quaternary px-4 py-4 text-cp font-emphasis text-cp-text-quaternary"
      >
        {{ t('settings.loadingModelMapping') }}
      </div>
      <div v-else-if="mappings.length > 0" class="grid max-w-6xl gap-3">
        <div
          v-for="(row, index) in mappings"
          :key="index"
          class="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-3 rounded-cp-card bg-cp-fill-quaternary p-3 sm:grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)_auto]"
        >
          <ZInput
            :model-value="row.requestedModel"
            :placeholder="t('settings.requestModel')"
            :aria-label="t('settings.requestModel')"
            @update:model-value="emit('updateMapping', index, 'requestedModel', $event)"
          />
          <span class="hidden text-cp-text-quaternary sm:block" aria-hidden="true">→</span>
          <ZInput
            class="col-start-1 row-start-2 sm:col-start-auto sm:row-start-auto"
            :model-value="row.upstreamModel"
            :placeholder="t('settings.upstreamModel')"
            :aria-label="t('settings.upstreamModelName')"
            @update:model-value="emit('updateMapping', index, 'upstreamModel', $event)"
          />
          <ZIconButton
            class="col-start-2 row-span-2 row-start-1 sm:col-start-auto sm:row-span-1 sm:row-start-auto"
            :aria-label="t('settings.deleteMapping')"
            @click="emit('removeMapping', index)"
          >
            <Trash2 class="size-4 text-cp-error" />
          </ZIconButton>
        </div>
      </div>
    </div>
  </ZCard>
</template>
