<script setup lang="ts">
import { ZButton, ZDialog } from '@codex-proxy/ui'

import { I18nT, useI18n } from 'vue-i18n'

const { t } = useI18n({ useScope: 'global' })

const open = defineModel<boolean>({ default: false })
</script>

<template>
  <ZDialog
    v-model="open"
    :title="t('settings.backup.r2.cloudflareR2OnboardingGuide')"
    width="48rem"
  >
    <div class="flex flex-col gap-4 text-cp leading-relaxed text-cp-text-secondary">
      <section class="flex flex-col gap-1.5">
        <h3 class="text-cp-lg font-heavy text-cp-primary-text">
          {{ t('settings.backup.r2.createBucket') }}
        </h3>
        <I18nT keypath="settings.backup.r2.bucketDescription" tag="p" scope="global" class="m-0">
          <template #bucket>
            <code class="rounded bg-cp-fill-quaternary px-1.5 py-0.5">codex-proxy-backups</code>
          </template>
        </I18nT>
      </section>

      <section class="flex flex-col gap-1.5">
        <h3 class="text-cp-lg font-heavy text-cp-primary-text">
          {{ t('settings.backup.r2.createApiToken') }}
        </h3>
        <p class="m-0">
          {{ t('settings.backup.r2.tokenDescription') }}
        </p>
      </section>

      <section class="flex flex-col gap-1.5">
        <h3 class="text-cp-lg font-heavy text-cp-primary-text">
          {{ t('settings.backup.r2.fillInConnectionParameters') }}
        </h3>
        <ul class="m-0 list-disc space-y-1 pl-5">
          <li>
            {{ t('settings.backup.r2.endpointLabel') }} <code class="rounded bg-cp-fill-quaternary px-1.5 py-0.5">https://&#123;ACCOUNT_ID&#125;.r2.cloudflarestorage.com</code>
          </li>
          <I18nT keypath="settings.backup.r2.regionDescription" tag="li" scope="global">
            <template #region>
              <code class="rounded bg-cp-fill-quaternary px-1.5 py-0.5">auto</code>
            </template>
          </I18nT>
          <li>{{ t('settings.backup.r2.credentialsHelp') }}</li>
          <li>{{ t('settings.backup.r2.forcePathStyleOff') }}</li>
        </ul>
      </section>

      <section class="flex flex-col gap-1.5">
        <h3 class="text-cp-lg font-heavy text-cp-primary-text">
          {{ t('settings.backup.r2.completeConnectionTest') }}
        </h3>
        <p class="m-0">
          {{ t('settings.backup.r2.testDescription') }}
        </p>
      </section>
    </div>

    <template #footer>
      <ZButton type="primary" @click="open = false">
        {{ t('settings.backup.r2.gotIt') }}
      </ZButton>
    </template>
  </ZDialog>
</template>
