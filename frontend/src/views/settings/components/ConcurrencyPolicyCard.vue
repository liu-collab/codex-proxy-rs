<script setup lang="ts">
import { ZCard, ZForm, ZFormItem, ZIconButton, ZInput, ZPopover } from '@codex-proxy/ui'

import { CircleAlert } from '@lucide/vue'

import { useI18n } from 'vue-i18n'

const { t } = useI18n({ useScope: 'global' })

const maxConcurrentPerAccount = defineModel<string>('maxConcurrentPerAccount', { required: true })
const maxWaitingPerKey = defineModel<string>('maxWaitingPerKey', { required: true })
const maxWaitingPerAccount = defineModel<string>('maxWaitingPerAccount', { required: true })
const concurrencyWaitTimeoutSeconds = defineModel<string>('concurrencyWaitTimeoutSeconds', { required: true })
const openaiGuardianReservedConcurrency = defineModel<string>('openaiGuardianReservedConcurrency', { required: true })
</script>

<template>
  <ZCard :title="t('settings.concurrency.title')">
    <ZForm class="max-w-6xl sm:grid-cols-2 sm:items-end">
      <ZFormItem
        :label="t('settings.concurrency.accountLimit.label')"
        :description="t('settings.concurrency.accountLimit.help')"
      >
        <ZInput
          v-model="maxConcurrentPerAccount"
          :aria-label="t('settings.concurrency.accountLimit.label')"
          type="number"
          min="0"
          max="4294967295"
          step="1"
        />
      </ZFormItem>

      <ZFormItem :label="t('settings.concurrency.keyQueueCapacity')" :description="t('settings.concurrency.keyQueueHelp')">
        <ZInput v-model="maxWaitingPerKey" :aria-label="t('settings.concurrency.keyQueueCapacity')" type="number" min="0" max="1000" step="1" />
      </ZFormItem>
      <ZFormItem :label="t('settings.concurrency.accountQueueCapacity')" :description="t('settings.concurrency.accountQueueHelp')">
        <ZInput v-model="maxWaitingPerAccount" :aria-label="t('settings.concurrency.accountQueueCapacity')" type="number" min="0" max="1000" step="1" />
      </ZFormItem>
      <ZFormItem :label="t('settings.concurrency.queueTimeoutLabel')" :description="t('settings.concurrency.queueTimeoutHelp')">
        <template #label-extra>
          <ZPopover class="-my-1" trigger="hover-click" placement="top-start">
            <template #reference="{ open }">
              <ZIconButton :aria-label="t('settings.concurrency.queueTimeoutHelpTitle')" :title="undefined" :aria-expanded="open" class="size-6! hover:bg-transparent! active:bg-transparent!">
                <CircleAlert class="size-3.5" aria-hidden="true" />
              </ZIconButton>
            </template>
            <p class="max-w-72 px-3 py-2 text-cp-sm leading-relaxed text-cp-text-secondary">
              {{ t('settings.concurrency.queueTimeoutDetails') }}
            </p>
          </ZPopover>
        </template>
        <ZInput v-model="concurrencyWaitTimeoutSeconds" :aria-label="t('settings.concurrency.queueTimeoutLabel')" type="number" min="1" max="120" step="1" />
      </ZFormItem>
      <ZFormItem :label="t('settings.concurrency.autoReview.label')" :description="t('settings.concurrency.autoReview.description')">
        <template #label-extra>
          <ZPopover class="-my-1" trigger="hover-click" placement="top-start">
            <template #reference="{ open }">
              <ZIconButton :aria-label="t('settings.concurrency.autoReview.helpTitle')" :title="undefined" :aria-expanded="open" class="size-6! hover:bg-transparent! active:bg-transparent!">
                <CircleAlert class="size-3.5" aria-hidden="true" />
              </ZIconButton>
            </template>
            <div class="max-w-72 space-y-2 px-3 py-2 text-cp-sm leading-relaxed text-cp-text-secondary">
              <p>{{ t('settings.scheduling.autoReviewSlotsHelp') }}</p>
              <p>{{ t('settings.scheduling.autoReviewQueueHelp') }}</p>
            </div>
          </ZPopover>
        </template>
        <ZInput v-model="openaiGuardianReservedConcurrency" :aria-label="t('settings.concurrency.autoReview.label')" type="number" min="0" max="4294967295" step="1" />
      </ZFormItem>
    </ZForm>
  </ZCard>
</template>
