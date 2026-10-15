<script setup lang="ts">
import { ZButton, ZCard, ZMotionIcon } from '@codex-proxy/ui'
import { LockKeyhole } from '@lucide/vue'
import { shallowRef } from 'vue'
import ChangePasswordModal from './ChangePasswordModal.vue'

const open = shallowRef(false)
</script>

<template>
  <ZCard title="管理员密码" description="用于登录管理后台">
    <template #actions>
      <ZButton @click="open = true">
        <template #icon>
          <LockKeyhole class="size-4" aria-hidden="true" />
        </template>
        修改密码
      </ZButton>
    </template>

    <div class="flex min-h-16 max-w-6xl items-center gap-3 rounded-cp bg-cp-fill-quaternary px-4 py-3">
      <ZMotionIcon class="inline-flex size-9 shrink-0 items-center justify-center rounded-cp bg-cp-bg-container text-cp-primary-text shadow-cp-tertiary">
        <LockKeyhole class="size-4" aria-hidden="true" />
      </ZMotionIcon>
      <div class="min-w-0">
        <p class="m-0 text-cp leading-[1.15] font-bold text-cp-text">
          验证当前密码
        </p>
        <p class="mt-1.5 mb-0 text-cp-sm leading-snug font-emphasis text-cp-text-secondary">
          修改后，所有管理员需重新登录
        </p>
      </div>
    </div>
  </ZCard>
  <ChangePasswordModal v-model="open" />
</template>
