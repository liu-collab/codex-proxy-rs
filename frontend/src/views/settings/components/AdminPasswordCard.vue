<script setup lang="ts">
import { ZButton, ZCard, ZMotionIcon } from '@codex-proxy/ui'

import { LockKeyhole } from '@lucide/vue'
import { shallowRef } from 'vue'
import { useI18n } from 'vue-i18n'
import ChangePasswordModal from './ChangePasswordModal.vue'

const { t } = useI18n({ useScope: 'global' })

const open = shallowRef(false)
</script>

<template>
  <ZCard :title="t('settings.password.administratorPassword')" :description="t('settings.password.description')">
    <template #actions>
      <ZButton @click="open = true">
        <template #icon>
          <LockKeyhole class="size-4" aria-hidden="true" />
        </template>
        {{ t('settings.password.changePassword') }}
      </ZButton>
    </template>

    <div class="flex min-h-16 max-w-6xl items-center gap-3 rounded-cp bg-cp-fill-quaternary px-4 py-3">
      <ZMotionIcon class="inline-flex size-9 shrink-0 items-center justify-center rounded-cp bg-cp-bg-container text-cp-primary-text shadow-cp-tertiary">
        <LockKeyhole class="size-4" aria-hidden="true" />
      </ZMotionIcon>
      <div class="min-w-0">
        <p class="m-0 text-cp leading-[1.15] font-bold text-cp-text">
          {{ t('settings.password.verifyCurrentPassword') }}
        </p>
        <p class="mt-1.5 mb-0 text-cp-sm leading-snug font-emphasis text-cp-text-secondary">
          {{ t('settings.password.sessionNotice') }}
        </p>
      </div>
    </div>
  </ZCard>
  <ChangePasswordModal v-model="open" />
</template>
