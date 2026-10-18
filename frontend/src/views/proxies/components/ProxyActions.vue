<script setup lang="ts">
import type { OutboundProxyRecord } from '@/api'
import { ZIconButton, ZMenuItem, ZPopover } from '@codex-proxy/ui'
import { MoreHorizontal, Pencil, Power, Trash2, Wifi } from '@lucide/vue'

defineProps<{
  proxy: OutboundProxyRecord
  testing: boolean
  batchTesting: boolean
  updating: boolean
}>()

const emit = defineEmits<{
  test: [proxy: OutboundProxyRecord]
  edit: [proxy: OutboundProxyRecord]
  toggle: [proxy: OutboundProxyRecord]
  delete: [proxy: OutboundProxyRecord]
}>()
</script>

<template>
  <div class="flex items-center gap-0.5">
    <ZIconButton size="small" aria-label="测试代理" :loading="testing && !batchTesting" :disabled="batchTesting || testing || updating" @click="emit('test', proxy)">
      <Wifi class="size-3.5 text-cp-link" />
    </ZIconButton>
    <ZIconButton size="small" aria-label="编辑代理" :disabled="testing || updating" @click="emit('edit', proxy)">
      <Pencil class="size-3.5 text-cp-link" />
    </ZIconButton>
    <ZPopover placement="bottom-end">
      <template #reference="{ open }">
        <ZIconButton size="small" :aria-label="`${proxy.name} 的更多操作`" :pressed="open" :disabled="testing || updating" :loading="updating">
          <MoreHorizontal class="size-4" />
        </ZIconButton>
      </template>
      <template #default="{ close }">
        <div class="w-36 p-1.5">
          <ZMenuItem :disabled="testing || updating" @click="(close(), emit('toggle', proxy))">
            <template #icon>
              <Power class="size-3.5" :class="proxy.enabled ? 'text-cp-warning' : 'text-cp-success'" />
            </template>
            {{ proxy.enabled ? '禁用代理' : '启用代理' }}
          </ZMenuItem>
          <ZMenuItem type="danger" :disabled="testing || updating" @click="(close(), emit('delete', proxy))">
            <template #icon>
              <Trash2 class="size-3.5" />
            </template>
            删除代理
          </ZMenuItem>
        </div>
      </template>
    </ZPopover>
  </div>
</template>
