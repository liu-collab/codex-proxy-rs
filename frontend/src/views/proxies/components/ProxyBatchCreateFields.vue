<script setup lang="ts">
import type { ProxyBatchInput } from '../utils/batch'
import { ZForm, ZFormItem, ZIconButton, ZInput, ZPopover, ZSelect, ZTextarea } from '@codex-proxy/ui'
import { CircleHelp } from '@lucide/vue'
import { computed } from 'vue'
import { MAX_BATCH_CREATE, proxyProtocolOptions } from '../utils/batch'

const props = defineProps<{ rows: ProxyBatchInput[], error: string, disabled: boolean }>()
const text = defineModel<string>({ required: true })
const prefix = defineModel<string>('prefix', { required: true })
const protocol = defineModel<string>('protocol', { required: true })
const invalid = computed(() => props.rows.filter(row => row.error))
</script>

<template>
  <ZForm class="grid gap-5">
    <ZFormItem label="代理列表" required :error="error || undefined">
      <template #label-extra>
        <ZPopover class="-my-1" trigger="hover-click" placement="bottom-start">
          <template #reference="{ open }">
            <ZIconButton aria-label="代理列表格式说明" :title="undefined" :aria-expanded="open" class="size-6! hover:bg-transparent! active:bg-transparent!">
              <CircleHelp class="size-3.5" aria-hidden="true" />
            </ZIconButton>
          </template>
          <div class="max-w-80 space-y-3 p-3 text-cp-sm text-cp-text-secondary">
            <p class="m-0">
              支持以下格式
            </p>
            <div class="space-y-1 font-mono text-cp-xs break-all text-cp-text">
              <p class="m-0">
                host:port
              </p>
              <p class="m-0">
                host:port:用户名:密码
              </p>
              <p class="m-0">
                http://用户名:密码@host:port
              </p>
            </div>
            <p class="m-0">
              未写协议时使用下方默认协议，完整 URL 保留各自协议
            </p>
            <p class="m-0">
              IPv6 地址需加方括号，例如 <code>[::1]:8080</code>
            </p>
          </div>
        </ZPopover>
      </template>
      <template #extra>
        <span class="text-cp-xs text-cp-text-tertiary">每行一条 · 最多 {{ MAX_BATCH_CREATE }} 条</span>
      </template>
      <ZTextarea v-model="text" :rows="8" :disabled="disabled" class="font-mono" aria-label="批量代理列表" autocomplete="off" autocapitalize="off" spellcheck="false" placeholder="proxy.example.com:8080&#10;proxy.example.net:1080:user:password&#10;socks5://user:password@proxy.example.org:1080" />
      <div v-if="rows.length && !error" class="mt-2 space-y-1.5 text-cp-xs">
        <p class="m-0 text-cp-text-secondary" role="status">
          共 {{ rows.length }} 条，{{ rows.length - invalid.length }} 条可添加<span v-if="invalid.length" class="text-cp-error-text">，{{ invalid.length }} 条需修改</span>
        </p>
        <ul v-if="invalid.length" class="m-0 list-none space-y-1 p-0 text-cp-error-text" role="alert">
          <li v-for="row in invalid.slice(0, 5)" :key="row.line">
            第 {{ row.line }} 行：{{ row.error }}
          </li>
          <li v-if="invalid.length > 5">
            另有 {{ invalid.length - 5 }} 条需处理
          </li>
        </ul>
      </div>
    </ZFormItem>
    <div class="grid items-start gap-4 sm:grid-cols-[10rem_minmax(0,1fr)]">
      <ZFormItem label="默认协议">
        <ZSelect v-model="protocol" :options="proxyProtocolOptions" :disabled="disabled" aria-label="无协议代理的默认协议" class="w-full" />
      </ZFormItem>
      <ZFormItem label="名称前缀">
        <ZInput v-model="prefix" maxlength="96" :disabled="disabled" aria-label="批量代理名称前缀" placeholder="代理" />
      </ZFormItem>
    </div>
  </ZForm>
</template>
