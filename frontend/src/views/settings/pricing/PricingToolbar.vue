<script setup lang="ts">
import { ZButton, ZIconButton, ZInput, ZPopover, ZSegmented, ZSelect } from '@codex-proxy/ui'
import { CircleAlert, Download, Percent, Plus, RotateCcw, Search, X } from '@lucide/vue'
import { computed, shallowRef, useId } from 'vue'
import { formatProviderLabel, providerIcon } from '@/utils/providers'
import { sourceLabels } from './model'

const props = defineProps<{
  selectedCount: number
  disabled: boolean
  saving: boolean
  syncing: boolean
  syncedAtDisplay?: string | null
  providers: string[]
}>()
defineEmits<{ add: [], sync: [], setMultiplier: [], reset: [], clear: [] }>()
const search = defineModel<string>('search', { required: true })
const provider = defineModel<string>('provider', { required: true })
const source = defineModel<string>('source', { required: true })
const syncInfoOpen = shallowRef(false)
const syncInfoId = useId()
const syncTime = computed(() => props.syncedAtDisplay ?? '尚未同步')
const providerOptions = computed(() => props.providers.map(value => ({
  label: formatProviderLabel(value),
  value,
  icon: providerIcon(value),
})))
</script>

<template>
  <div class="mb-3 shrink-0">
    <div class="flex flex-wrap items-center gap-2">
      <ZInput v-model="search" class="w-full sm:w-72" placeholder="搜索模型" aria-label="搜索模型 ID">
        <template #prefix>
          <Search class="size-4" />
        </template>
      </ZInput>
      <div v-if="providerOptions.length" class="min-w-0 max-w-full overflow-x-auto">
        <ZSegmented
          v-model="provider"
          aria-label="模型提供商"
          display="icon"
          :style="{ width: `${providerOptions.length * 40 + 4}px` }"
          :options="providerOptions"
          :disabled="saving"
        />
      </div>
      <ZSelect v-model="source" class="w-32" aria-label="价格来源" :options="[{ label: '全部来源', value: 'all' }, { label: sourceLabels.custom, value: 'custom' }, { label: sourceLabels.synced, value: 'synced' }, { label: sourceLabels.builtin, value: 'builtin' }]" />
      <div v-if="selectedCount" class="ml-auto flex w-full flex-wrap items-center justify-end gap-2 sm:w-auto" role="group" aria-label="批量操作">
        <span class="inline-flex h-cp-control w-full items-center gap-1 whitespace-nowrap text-cp text-cp-text-secondary sm:mr-1 sm:w-auto" aria-live="polite">
          已选 <span class="font-mono tabular-nums">{{ selectedCount }}</span> 项
        </span>
        <ZButton :disabled="disabled || selectedCount > 500" @click="$emit('setMultiplier')">
          <template #icon>
            <Percent class="size-4" />
          </template>
          设置倍率
        </ZButton>
        <ZButton :disabled="disabled || selectedCount > 500" @click="$emit('reset')">
          <template #icon>
            <RotateCcw class="size-4" />
          </template>
          清除覆盖
        </ZButton>
        <ZIconButton aria-label="取消选择" variant="plain" :disabled="disabled" @click="$emit('clear')">
          <X class="size-4" />
        </ZIconButton>
      </div>
      <div v-else class="ml-auto flex items-center gap-2">
        <div class="flex items-center gap-1">
          <ZPopover v-model="syncInfoOpen" placement="bottom-end" trigger="hover-click">
            <template #reference>
              <ZIconButton aria-label="同步说明" size="small" :aria-expanded="syncInfoOpen" :aria-controls="syncInfoOpen ? syncInfoId : undefined">
                <CircleAlert class="size-3.5" />
              </ZIconButton>
            </template>
            <div :id="syncInfoId" class="grid w-64 gap-2 p-3 text-cp-xs leading-relaxed text-cp-text-secondary">
              <p class="m-0">
                从 models.dev 获取社区价目，保留人工单价与倍率
              </p>
              <p class="m-0 text-cp-text-tertiary">
                最近同步：{{ syncTime }}
              </p>
            </div>
          </ZPopover>
          <ZButton :loading="syncing" :disabled="disabled" aria-label="同步价目" @click="$emit('sync')">
            <template #icon>
              <Download class="size-4" />
            </template>
            同步
          </ZButton>
        </div>
        <ZButton type="primary" :disabled="disabled" @click="$emit('add')">
          <template #icon>
            <Plus class="size-4" />
          </template>
          添加模型
        </ZButton>
      </div>
    </div>
  </div>
</template>
