<script setup lang="ts">
import { ZCard, ZForm, ZFormItem, ZInput } from '@codex-proxy/ui'

const leadMinutes = defineModel<string>({ required: true })
const maxWeeklyRemainingPercent = defineModel<string>('maxWeeklyRemainingPercent', { required: true })
</script>

<template>
  <ZCard id="reset-credits" title="主动重置卡" description="在账号的「额度重置」中开启自动使用">
    <ZForm class="max-w-6xl sm:grid-cols-2">
      <ZFormItem label="提前使用时间（分钟）" description="全局设置，对已开启自动使用的账号生效">
        <ZInput v-model="leadMinutes" type="number" min="1" max="1440" step="1" aria-label="重置卡提前使用时间" />
      </ZFormItem>
      <ZFormItem label="剩余周额度上限（%）" description="高于此值时暂不自动使用，留空表示不限制">
        <ZInput v-model="maxWeeklyRemainingPercent" type="number" min="0" max="100" step="1" placeholder="不限制" aria-label="重置卡剩余周额度上限" />
      </ZFormItem>
    </ZForm>
  </ZCard>
</template>
