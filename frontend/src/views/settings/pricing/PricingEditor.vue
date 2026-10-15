<script setup lang="ts">
import type { PricingRow } from './model'
import type { ModelPricing, PriceBand, TokenPrices } from '@/api'
import { ZButton, ZDialog, ZFormItem, ZInput, ZPopover, ZSegmented, ZSelect } from '@codex-proxy/ui'
import { Info } from '@lucide/vue'
import { computed, ref, shallowRef, useId, watch } from 'vue'
import ProviderIconGroup from '@/components/ProviderIconGroup.vue'
import { bands, parseMultiplier, validPrice } from './model'
import PricingRateFields from './PricingRateFields.vue'

const props = defineProps<{ row?: PricingRow, provider: string, saving: boolean }>()
const emit = defineEmits<{ save: [model: string, pricing: ModelPricing] }>()
const open = defineModel<boolean>({ required: true })
const model = ref('')
const multiplier = ref('1')
const draft = ref<Partial<Record<PriceBand, TokenPrices>>>({})
const selectedBand = ref<PriceBand>('standard')
const validation = ref('')
const helpOpen = shallowRef(false)
const helpId = useId()
const base = computed(() => props.row?.base.bands[selectedBand.value])
const current = computed(() => draft.value[selectedBand.value])
const bps = computed(() => parseMultiplier(multiplier.value))
const sourceOptions = computed(() => [
  { label: '来源价', value: 'inherit', disabled: selectedBand.value === 'standard' && !props.row?.base.bands.standard },
  { label: '自定义', value: 'custom' },
])
const priceMode = computed({
  get: () => current.value ? 'custom' : 'inherit',
  set: value => toggleBand(value === 'custom'),
})
const bandOptions = computed(() => bands
  .filter(band => props.row?.base.bands.image
    ? ['standard', 'image'].includes(band.value)
    : props.provider !== 'xai' || !['flex', 'long_flex', 'ultrafast', 'long_ultrafast', 'image'].includes(band.value))
  .map(band => ({ ...band, label: band.value === 'standard' && props.row?.base.bands.image ? '文本 Token' : band.label })))

watch(open, (value) => {
  if (!value)
    return
  model.value = props.row?.model ?? ''
  multiplier.value = String((props.row?.custom?.multiplierBps ?? 10_000) / 10_000)
  draft.value = JSON.parse(JSON.stringify(props.row?.custom?.bands ?? {}))
  selectedBand.value = props.row?.effective.bands.image ? 'image' : 'standard'
  if (!props.row)
    toggleBand(true)
  validation.value = ''
  helpOpen.value = false
})
watch([model, multiplier, draft], () => {
  validation.value = ''
}, { deep: true })
function toggleBand(enabled: boolean) {
  if (enabled)
    draft.value[selectedBand.value] = { ...(base.value ?? { input: '', output: '', cacheRead: '', cacheWrite: '' }) }
  else delete draft.value[selectedBand.value]
}
function updatePrice(field: keyof TokenPrices, value: string) {
  if (current.value && !props.saving)
    current.value[field] = value
}
function submit() {
  if (!model.value || new TextEncoder().encode(model.value).length > 128 || /\s/.test(model.value) || [...model.value].some(char => char.charCodeAt(0) < 32 || char.charCodeAt(0) === 127)) {
    validation.value = '模型 ID 必须为 1～128 字节，不含空白或控制字符'
    return
  }
  if (bps.value === undefined) {
    validation.value = '倍率范围为 0～100，最多四位小数'
    return
  }
  for (const [band, rates] of Object.entries(draft.value)) {
    if (Object.values(rates).some(price => !validPrice(price))) {
      validation.value = '请完整填写该档位的四项单价：0～1000000，最多四位小数'
      selectedBand.value = band as PriceBand
      return
    }
  }
  if (!props.row?.base.bands.standard && !draft.value.standard) {
    validation.value = '新模型需要配置标准档单价'
    selectedBand.value = 'standard'
    return
  }
  validation.value = ''
  emit('save', model.value, { multiplierBps: bps.value, bands: draft.value })
}
</script>

<template>
  <ZDialog v-model="open" :title="row ? '编辑模型价格' : '添加模型价格'" width="42rem" :show-close="!saving" :close-on-click-modal="!saving" :close-on-press-escape="!saving">
    <template #description>
      <span class="inline-flex items-center gap-2 align-middle text-cp-sm font-normal">
        <ProviderIconGroup :provider="provider" size="sm" />
        <span>USD / 1M Tokens</span>
      </span>
    </template>
    <div class="grid gap-5">
      <div class="grid grid-cols-[minmax(0,1fr)_6.5rem] gap-3 sm:grid-cols-[minmax(0,1fr)_8rem] sm:gap-4">
        <ZFormItem label="上游模型 ID">
          <ZInput v-model="model" :disabled="saving || !!row" aria-label="上游模型 ID" placeholder="例如 gpt-5.4" class="font-mono" />
        </ZFormItem>
        <ZFormItem label="倍率">
          <ZInput v-model="multiplier" aria-label="自定义倍率" class="font-mono" inputmode="decimal" :disabled="saving">
            <template #suffix>
              ×
            </template>
          </ZInput>
        </ZFormItem>
      </div>
      <section aria-label="档位价格">
        <div class="mb-5 flex flex-wrap items-center justify-between gap-3">
          <ZSelect :model-value="selectedBand" :options="bandOptions" aria-label="价格档位" class="min-w-0 flex-1 sm:max-w-56" :disabled="saving" @update:model-value="selectedBand = $event as PriceBand" />
          <ZSegmented v-model="priceMode" aria-label="单价来源" :options="sourceOptions" :disabled="saving" />
        </div>
        <PricingRateFields v-if="current || base" class="rounded-cp-lg bg-cp-fill-quaternary p-4" :base="base" :custom="current" :multiplier-bps="bps" :disabled="saving" @change="updatePrice" />
        <div v-else class="grid justify-items-center gap-3 py-6 text-cp-sm text-cp-text-tertiary">
          <p class="m-0">
            此档位暂无来源价格
          </p>
          <ZButton size="small" :disabled="saving" @click="toggleBand(true)">
            填写单价
          </ZButton>
        </div>
      </section>
      <p v-if="validation" role="alert" class="m-0 rounded-cp bg-cp-error-container p-3 text-cp-sm text-cp-error-on-container">
        {{ validation }}
      </p>
    </div>
    <template #footer>
      <ZPopover v-model="helpOpen" class="mr-auto self-center" placement="top-start" trigger="hover-click">
        <template #reference>
          <ZButton variant="text" :aria-expanded="helpOpen" :aria-controls="helpOpen ? helpId : undefined">
            <template #icon>
              <Info class="size-3.5" aria-hidden="true" />
            </template>
            计价说明
          </ZButton>
        </template>
        <div :id="helpId" class="grid w-72 gap-2 p-3 text-cp-xs leading-relaxed text-cp-text-secondary">
          <p class="m-0">
            模型 ID 使用实际发送给上游的名称，不是客户端别名
          </p>
          <p class="m-0">
            自定义档位需填写四项单价，0 表示免费，单价上限 1000000，倍率范围 0～100，最多四位小数
          </p>
          <p class="m-0">
            仅调整本地估算与金额限额，不代表订阅实际扣费，保存后仅新请求生效，历史账单不变
          </p>
        </div>
      </ZPopover>
      <ZButton :disabled="saving" @click="open = false">
        取消
      </ZButton><ZButton type="primary" :loading="saving" @click="submit">
        保存
      </ZButton>
    </template>
  </ZDialog>
</template>
