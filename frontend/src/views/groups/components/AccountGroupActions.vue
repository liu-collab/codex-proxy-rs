<script setup lang="ts">
import type { AccountGroup } from '@/api'

import { ZIconButton } from '@codex-proxy/ui'
import { Pencil, Power, Trash2 } from '@lucide/vue'

import { useI18n } from 'vue-i18n'

defineProps<{
  group: AccountGroup
  updatingStatus: boolean
  deleting: boolean
}>()

const emit = defineEmits<{
  edit: [group: AccountGroup]
  toggle: [group: AccountGroup]
  delete: [group: AccountGroup]
}>()

const { t } = useI18n({ useScope: 'global' })
</script>

<template>
  <div class="flex items-center gap-1">
    <ZIconButton
      size="small"
      :aria-label="t('groups.actions.editGroup')"
      @click.stop="emit('edit', group)"
    >
      <Pencil class="size-3.5 text-cp-link" />
    </ZIconButton>
    <ZIconButton
      size="small"
      :aria-label="group.enabled ? t('groups.actions.disableGroup') : t('groups.actions.enableGroup')"
      :loading="updatingStatus"
      @click.stop="emit('toggle', group)"
    >
      <Power
        class="size-3.5"
        :class="group.enabled ? 'text-cp-warning' : 'text-cp-success'"
      />
    </ZIconButton>
    <ZIconButton
      size="small"
      :aria-label="t('groups.actions.deleteGroup')"
      :disabled="deleting"
      @click.stop="emit('delete', group)"
    >
      <Trash2 class="size-3.5 text-cp-error" />
    </ZIconButton>
  </div>
</template>
