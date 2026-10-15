<script setup lang="ts">
import type { AccountAffinity } from '@/api'
import { ZCard, ZForm, ZFormItem, ZIconButton, ZInput, ZPopover, ZSegmented } from '@codex-proxy/ui'
import { CircleAlert } from '@lucide/vue'

const props = defineProps<{ disabled: boolean }>()
const accountAffinity = defineModel<AccountAffinity | ''>({ required: true })
const affinityTtlHours = defineModel<string>('ttlHours', { required: true })
const maxAccountRotations = defineModel<string>('maxAccountRotations', { required: true })
const affinityOptions = [{ label: '宽松', value: 'relaxed' }, { label: '优先', value: 'preferred' }, { label: '严格', value: 'strict' }]
</script>

<template>
  <ZCard title="亲和策略">
    <ZForm class="max-w-6xl sm:grid-cols-2 lg:grid-cols-3">
      <ZFormItem label="账号亲和" description="控制同一会话内的账号绑定方式">
        <template #label-extra>
          <ZPopover class="-my-1" trigger="hover-click" placement="top-start">
            <template #reference="{ open }">
              <ZIconButton aria-label="账号亲和说明" :title="undefined" :aria-expanded="open" class="size-6! hover:bg-transparent! active:bg-transparent!">
                <CircleAlert class="size-3.5" aria-hidden="true" />
              </ZIconButton>
            </template>
            <div class="max-w-72 space-y-2 px-3 py-2 text-cp-sm leading-relaxed text-cp-text-secondary">
              <p>宽松：会话内请求直接按调度策略选号，不优先主账号</p>
              <p>优先：会话内请求优先沿用主账号，繁忙或不可用时临时分流，不改变会话绑定</p>
              <p>严格：同一会话共用账号，子线程等待当前账号，并跟随主线程换号</p>
              <p>重新选号仍遵循当前调度策略</p>
            </div>
          </ZPopover>
        </template>
        <ZSegmented v-model="accountAffinity" aria-label="账号亲和" :options="affinityOptions" :disabled="props.disabled" class="w-full" />
      </ZFormItem>
      <ZFormItem label="亲和时长" description="账号绑定的保留时间，成功调度后续期">
        <ZInput v-model="affinityTtlHours" aria-label="亲和时长（小时）" type="number" min="1" max="720" step="1" :disabled="props.disabled">
          <template #suffix>
            小时
          </template>
        </ZInput>
      </ZFormItem>
      <ZFormItem label="最大换号次数" description="单请求最多换号次数，0 表示不换号">
        <ZInput v-model="maxAccountRotations" aria-label="最大换号次数" type="number" min="0" max="31" step="1" :disabled="props.disabled" />
      </ZFormItem>
    </ZForm>
  </ZCard>
</template>
