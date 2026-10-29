<script setup lang="ts">
import type { AccountGroupRef } from '@/api'

import { ZCheckbox } from '@codex-proxy/ui'

import { useI18n } from 'vue-i18n'

withDefaults(
  defineProps<{
    groups: AccountGroupRef[]
    loading?: boolean
    disabled?: boolean
  }>(),
  {
    loading: false,
    disabled: false,
  },
)

const { t } = useI18n({ useScope: 'global' })

const selectedGroupIds = defineModel<string[]>({ required: true })

function updateGroup(groupId: string, selected: boolean) {
  const next = new Set(selectedGroupIds.value)
  if (selected)
    next.add(groupId)
  else next.delete(groupId)
  selectedGroupIds.value = [...next]
}
</script>

<template>
  <div v-if="groups.length > 0" class="grid gap-2 sm:grid-cols-2">
    <div
      v-for="group in groups"
      :key="group.id"
      class="flex min-h-11 items-center justify-between gap-3 rounded-cp bg-cp-fill-quaternary px-3.5 py-2.5"
    >
      <span class="flex min-w-0 items-center gap-2.5">
        <span class="size-3.5 shrink-0 rounded-sm" :style="{ backgroundColor: group.color }" />
        <ZCheckbox
          :model-value="selectedGroupIds.includes(group.id)"
          :label="group.name"
          :disabled="disabled || loading"
          @update:model-value="updateGroup(group.id, $event)"
        />
      </span>
      <span v-if="!group.enabled" class="shrink-0 text-cp-xs font-emphasis text-cp-text-quaternary"> {{ t('common.status.disabled') }} </span>
    </div>
  </div>
  <p v-else class="m-0 rounded-cp bg-cp-fill-quaternary px-3.5 py-3 text-cp-sm font-emphasis text-cp-text-secondary">
    {{ loading ? t('groups.selector.loading') : t('groups.selector.empty') }}
  </p>
</template>
