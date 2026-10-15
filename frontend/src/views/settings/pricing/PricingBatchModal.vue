<script setup lang="ts">
import type { PricingRow } from './model'
import type { PricingChange } from '@/api'
import { ZButton, ZDialog, ZFormItem, ZInput, ZScrollbar } from '@codex-proxy/ui'
import { computed, ref, watch } from 'vue'
import { multiplierText, parseMultiplier } from './model'

defineProps<{ rows: PricingRow[], reset: boolean, saving: boolean }>()
defineEmits<{ confirm: [change: PricingChange] }>()
const open = defineModel<boolean>({ required: true })
const multiplier = ref('1')
const bps = computed(() => parseMultiplier(multiplier.value))
watch(open, (value) => {
  if (value)
    multiplier.value = '1'
})
</script>

<template>
  <ZDialog v-model="open" :title="reset ? '清除人工覆盖' : '批量设置倍率'" :description="`仅修改下列 ${rows.length} 个模型，不重算历史账单`" :show-close="!saving" :close-on-click-modal="!saving" :close-on-press-escape="!saving" type="warning">
    <div class="grid gap-4">
      <p v-if="reset" class="m-0 text-cp leading-relaxed text-cp-text-secondary">
        恢复来源或内置价格，无可用价格时设为未配置
      </p>
      <ZFormItem v-else label="目标倍率" description="设置为此倍率，不与已有倍率相乘，0～100，最多四位小数">
        <ZInput v-model="multiplier" aria-label="目标倍率" inputmode="decimal" :disabled="saving">
          <template #suffix>
            ×
          </template>
        </ZInput>
      </ZFormItem>
      <ZScrollbar max-height="16rem" class="rounded-cp bg-cp-fill-quaternary">
        <ul class="m-0 list-none p-3 text-cp-sm">
          <li v-for="row in rows" :key="row.model" class="flex items-start justify-between gap-3 py-2">
            <span class="min-w-0 break-all font-mono">{{ row.model }}</span>
            <span class="shrink-0 font-mono text-cp-text-secondary">{{ multiplierText(row.effective.multiplierBps) }} → {{ reset ? '1×' : bps === undefined ? '—' : multiplierText(bps) }}</span>
          </li>
        </ul>
      </ZScrollbar>
      <p v-if="!reset && bps === undefined" role="alert" class="m-0 text-cp-sm text-cp-error">
        请输入合法倍率
      </p>
    </div>
    <template #footer>
      <ZButton :disabled="saving" @click="open = false">
        取消
      </ZButton>
      <ZButton type="primary" :loading="saving" :disabled="!rows.length || (!reset && bps === undefined)" @click="$emit('confirm', reset ? { action: 'reset' } : { action: 'multiplier', multiplierBps: bps! })">
        {{ reset ? '确认清除覆盖' : '确认设置倍率' }}
      </ZButton>
    </template>
  </ZDialog>
</template>
