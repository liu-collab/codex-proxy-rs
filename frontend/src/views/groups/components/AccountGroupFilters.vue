<script setup lang="ts">
import { ZButton, ZInput } from '@codex-proxy/ui'

import { Plus, Search, Trash2 } from '@lucide/vue'

import { useI18n } from 'vue-i18n'

defineProps<{
  batchDeleting: boolean
  selectedCount: number
}>()

const emit = defineEmits<{
  create: []
  deleteSelected: []
}>()

const { t } = useI18n({ useScope: 'global' })

const search = defineModel<string>('search', { required: true })
</script>

<template>
  <div class="flex w-full flex-col gap-3 sm:flex-row sm:flex-wrap sm:items-center">
    <ZInput
      v-model="search"
      class="min-w-0 sm:w-80 sm:flex-none"
      :aria-label="t('groups.filters.searchAccountGroup')"
      :placeholder="t('groups.filters.searchGroupName')"
    >
      <template #prefix>
        <Search class="size-4.5 text-cp-text-tertiary" />
      </template>
    </ZInput>
    <div class="flex max-w-full shrink-0 flex-wrap items-center justify-end gap-2 sm:ml-auto">
      <ZButton
        v-if="selectedCount > 0"
        type="danger" variant="plain"
        :disabled="batchDeleting"
        @click="emit('deleteSelected')"
      >
        <template #icon>
          <Trash2 class="size-4" />
        </template>
        {{ t('groups.filters.deleteSelected', { count: selectedCount }) }}
      </ZButton>
      <ZButton type="primary" @click="emit('create')">
        <template #icon>
          <Plus class="size-4" />
        </template>
        {{ t('common.actions.create') }}
      </ZButton>
    </div>
  </div>
</template>
