<script setup lang="ts">
import type { AccountCreateForm } from '../../utils/accountCreate'

import type { AccountGroupRef } from '@/api'
import { ZFormItem, ZTextarea } from '@codex-proxy/ui'
import { useI18n } from 'vue-i18n'
import AccountSettingsFields from '../AccountSettingsFields.vue'
import AccountProviderChooser from './AccountProviderChooser.vue'

defineProps<{
  groups: AccountGroupRef[]
  groupsLoading: boolean
  disabled: boolean
  proxyError?: string
}>()

const { t } = useI18n({ useScope: 'global' })

const form = defineModel<AccountCreateForm>({ required: true })
</script>

<template>
  <div class="grid gap-6">
    <fieldset class="m-0 min-w-0 border-0 p-0">
      <legend class="mb-3 p-0 text-cp font-medium text-cp-text-secondary">
        {{ t('accounts.onboarding.accountPlatform') }}
      </legend>
      <AccountProviderChooser v-model="form.source" :disabled="disabled" />
    </fieldset>
    <AccountSettingsFields
      v-model:enabled="form.enabled"
      v-model:concurrency-limit="form.concurrencyLimit"
      v-model:weight="form.weight"
      v-model:model-access="form.modelAccess"
      v-model:selected-group-ids="form.groupIds"
      v-model:proxy-mode="form.proxyMode"
      v-model:proxy-id="form.proxyId"
      preserve-model-access
      :groups="groups"
      :groups-loading="groupsLoading"
      :preserve-proxy="false"
      :disabled="disabled"
      :proxy-error="proxyError"
    />
    <ZFormItem :label="t('common.fields.notes')">
      <ZTextarea
        v-model="form.notes"
        :rows="3"
        :maxlength="500"
        :placeholder="t('accounts.onboarding.maximumWordsOptional')"
        :disabled="disabled"
      />
    </ZFormItem>
  </div>
</template>
