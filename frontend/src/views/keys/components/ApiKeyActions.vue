<script setup lang="ts">
import type { ApiKey } from '@/api'

import { ZIconButton, ZMenuItem, ZPopover } from '@codex-proxy/ui'

import { MoreHorizontal, Pencil, Power, RotateCcw, Terminal, Trash2, Upload } from '@lucide/vue'

defineProps<{
  apiKey: ApiKey
  deleting: boolean
  updatingStatus: boolean
  revealing: boolean
}>()

const emit = defineEmits<{
  use: [apiKey: ApiKey]
  importCcs: [apiKey: ApiKey]
  toggle: [apiKey: ApiKey]
  delete: [apiKey: ApiKey]
  edit: [apiKey: ApiKey]
  resetBudget: [apiKey: ApiKey]
}>()
</script>

<template>
  <div class="flex items-center justify-start gap-0.5">
    <ZIconButton
      size="small"
      aria-label="编辑密钥"
      @click.stop="emit('edit', apiKey)"
    >
      <Pencil class="size-3.5 text-cp-link" />
    </ZIconButton>
    <ZIconButton
      size="small"
      aria-label="使用密钥"
      :loading="revealing"
      :disabled="revealing"
      @click.stop="emit('use', apiKey)"
    >
      <Terminal class="size-3.5 text-cp-primary-text" />
    </ZIconButton>

    <ZPopover placement="bottom-end">
      <template #reference="{ open }">
        <ZIconButton size="small" aria-label="更多操作" :pressed="open">
          <MoreHorizontal class="size-4" />
        </ZIconButton>
      </template>
      <template #default="{ close }">
        <div class="w-44 p-1.5">
          <ZMenuItem @click.stop="(close(), emit('resetBudget', apiKey))">
            <template #icon>
              <RotateCcw class="size-3.5 text-cp-text-quaternary" />
            </template>
            重置已用额度
          </ZMenuItem>
          <ZMenuItem :disabled="revealing" @click.stop="(close(), emit('importCcs', apiKey))">
            <template #icon>
              <Upload class="size-3.5 text-cp-text-quaternary" />
            </template>
            导入 CCSwitch
          </ZMenuItem>
          <ZMenuItem :loading="updatingStatus" @click.stop="(close(), emit('toggle', apiKey))">
            <template #icon>
              <Power class="size-3.5" :class="apiKey.enabled ? 'text-cp-warning' : 'text-cp-success'" />
            </template>
            {{ apiKey.enabled ? '禁用密钥' : '启用密钥' }}
          </ZMenuItem>
          <ZMenuItem type="danger" :disabled="deleting" @click.stop="(close(), emit('delete', apiKey))">
            <template #icon>
              <Trash2 class="size-3.5" />
            </template>
            删除密钥
          </ZMenuItem>
        </div>
      </template>
    </ZPopover>
  </div>
</template>
