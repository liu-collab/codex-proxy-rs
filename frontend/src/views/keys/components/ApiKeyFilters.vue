<script setup lang="ts">
import { ZButton, ZInput } from '@codex-proxy/ui'

import { Plus, Search, Trash2 } from '@lucide/vue'

defineProps<{
  batchDeleting: boolean
  selectedCount: number
}>()

const emit = defineEmits<{
  create: []
  deleteSelected: []
}>()

const search = defineModel<string>('search', { required: true })
</script>

<template>
  <div
    class="flex w-full flex-wrap items-center gap-3"
    role="group"
    aria-label="API Key 筛选与操作"
  >
    <div class="min-w-0 flex-1 md:w-96 md:flex-none">
      <ZInput v-model="search" placeholder="搜索名称或标签" aria-label="搜索 API Key 名称或标签" class="w-full">
        <template #prefix>
          <Search class="size-4.5 text-cp-text-tertiary" />
        </template>
      </ZInput>
    </div>

    <div class="flex shrink-0 items-center justify-end gap-2 md:ml-auto">
      <ZButton
        v-if="selectedCount > 0"
        type="danger" variant="plain"
        :disabled="batchDeleting"
        @click="emit('deleteSelected')"
      >
        <template #icon>
          <Trash2 class="size-4" />
        </template>
        删除选中 ({{ selectedCount }})
      </ZButton>
      <ZButton type="primary" @click="emit('create')">
        <template #icon>
          <Plus class="size-4" />
        </template>
        创建 API Key
      </ZButton>
    </div>
  </div>
</template>
