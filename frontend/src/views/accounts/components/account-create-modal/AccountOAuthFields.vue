<script setup lang="ts">
import type { AccountAuthorizationView } from '../../composables/useAccountAuthorization'

import { ZButton, ZForm, ZFormItem, ZIconButton, ZScrollbar, ZTextarea } from '@codex-proxy/ui'
import { Copy, KeyRound } from '@lucide/vue'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useCopyText } from '@/composables/useCopyText'

const props = defineProps<{
  authUrl: string
  panelTitle: string
  panelDescription: string
  callbackLabel: string
  callbackPlaceholder: string
  loading: boolean
  disabled: boolean
  authorization: AccountAuthorizationView
  canStart: boolean
}>()

const emit = defineEmits<{ regenerate: [] }>()

const { t } = useI18n({ useScope: 'global' })

const callback = defineModel<string>({ required: true })
const copyWithToast = useCopyText()
const statusText = computed(() => {
  if (props.authorization.status === 'expired')
    return t('accounts.onboarding.authorizationExpired')
  if (props.authorization.status === 'paused')
    return t('accounts.onboarding.pollingPaused')
  return ''
})
</script>

<template>
  <div class="flex flex-col gap-4">
    <div class="rounded-cp bg-cp-fill-quaternary px-4 py-3">
      <div class="flex items-start gap-3">
        <div
          class="mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-cp bg-cp-bg-container text-cp-primary-text"
        >
          <KeyRound class="size-4" />
        </div>
        <div class="min-w-0 flex-1">
          <p class="m-0 text-cp font-bold text-cp-text">
            {{ panelTitle }}
          </p>
          <p class="m-0 mt-1 text-cp-sm leading-[1.55] font-medium text-cp-text-secondary">
            {{ panelDescription }}
          </p>
        </div>
      </div>
    </div>

    <div class="flex flex-wrap items-center gap-2">
      <ZButton
        :loading="loading"
        :disabled="disabled || (!authorization.flow && !canStart)"
        @click="emit('regenerate')"
      >
        {{ authUrl ? t('accounts.onboarding.regenerateAuthorizationLink') : t('accounts.onboarding.generateAuthorizationLink') }}
      </ZButton>
    </div>

    <ZForm v-if="authUrl">
      <ZFormItem :label="t('accounts.onboarding.authorizationLink')">
        <template #extra>
          <ZIconButton
            variant="solid"
            size="small"
            :title="t('accounts.onboarding.copyLink')"
            :aria-label="t('accounts.onboarding.copyLink')"
            :disabled="disabled"
            @click="copyWithToast(authUrl, { successText: t('accounts.onboarding.authorizationLinkCopied') })"
          >
            <Copy class="size-3.5" />
          </ZIconButton>
        </template>
        <ZScrollbar max-height="92px">
          <div class="rounded-cp bg-(--cp-input-bg) px-3.5 py-3 shadow-cp-tertiary">
            <pre
              class="m-0 whitespace-pre-wrap wrap-break-word font-mono text-cp-sm leading-[1.6] font-emphasis text-cp-text-secondary"
              v-text="authUrl"
            />
          </div>
        </ZScrollbar>
      </ZFormItem>
    </ZForm>

    <p v-if="statusText" class="m-0 text-cp-sm text-cp-text-secondary" role="status">
      {{ statusText }}
    </p>
    <p v-if="authorization.error" class="m-0 text-cp-sm text-cp-error" role="alert">
      {{ authorization.error }}
    </p>

    <ZForm v-if="authorization.flow">
      <ZFormItem :label="callbackLabel" required>
        <ZTextarea
          v-model="callback"
          :aria-label="callbackLabel"
          :rows="4"
          :placeholder="callbackPlaceholder"
          :disabled="disabled"
        />
      </ZFormItem>
    </ZForm>
  </div>
</template>
