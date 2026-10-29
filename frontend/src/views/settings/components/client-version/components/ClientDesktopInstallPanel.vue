<script setup lang="ts">
import type { CodexDesktopWindowsDownloads } from '@/api'

import { ExternalLink } from '@lucide/vue'

import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import ClientDesktopOfflineDownloads from './ClientDesktopOfflineDownloads.vue'
import ClientInstallCommandList from './ClientInstallCommandList.vue'

defineOptions({ name: 'ClientDesktopInstallPanel' })

withDefaults(
  defineProps<{
    downloads?: CodexDesktopWindowsDownloads | null
    loading?: boolean
  }>(),
  {
    downloads: null,
    loading: false,
  },
)

const emit = defineEmits<{
  refresh: []
}>()

const { t } = useI18n({ useScope: 'global' })

const desktopCommands = computed(() => ([
  {
    label: 'Windows · Microsoft Store',
    command: 'winget install --id 9PLM9XGG6VKS -s msstore',
  },
  {
    label: t('settings.clientVersion.ubuntuDebianUpgrade'),
    command: 'sudo apt update && sudo apt install --only-upgrade chatgpt',
  },
  {
    label: t('settings.clientVersion.fedoraRhelUpgrade'),
    command: 'sudo dnf upgrade --refresh chatgpt',
  },
]))
</script>

<template>
  <section class="grid gap-4" aria-labelledby="codex-desktop-online-title">
    <div class="grid gap-3 rounded-cp-card bg-cp-fill-quaternary p-4">
      <div class="flex flex-wrap items-start justify-between gap-3">
        <div class="min-w-0">
          <h3 id="codex-desktop-online-title" class="m-0 text-cp-sm font-heavy text-cp-text">
            {{ t('settings.clientVersion.onlineInstallationAndUpgrade') }}
          </h3>
          <p class="mt-1.5 mb-0 text-cp-sm leading-[1.45] font-semibold text-cp-text-secondary">
            {{ t('settings.clientVersion.desktopUpgradeHelp') }}
          </p>
        </div>
        <a
          href="https://chatgpt.com/download/"
          target="_blank"
          rel="noopener noreferrer"
          class="inline-flex h-cp-control shrink-0 items-center gap-2 rounded-cp-sm bg-cp-fill-tertiary px-3.5 text-cp font-bold text-cp-text shadow-cp-tertiary transition-[background-color,transform] hover:bg-cp-bg-text-active motion-safe:active:translate-y-px"
        >
          {{ t('settings.clientVersion.officialDownloadPage') }}
          <ExternalLink class="size-3.5" />
        </a>
      </div>
      <ClientInstallCommandList :commands="desktopCommands" />
    </div>

    <ClientDesktopOfflineDownloads
      :downloads="downloads"
      :loading="loading"
      @refresh="emit('refresh')"
    />
  </section>
</template>
