<script setup lang="ts">
import { ZCard, ZForm, ZFormItem, ZIconButton, ZInput } from '@codex-proxy/ui'

import { CircleHelp, MonitorUp, TerminalSquare } from '@lucide/vue'

import { useI18n } from 'vue-i18n'

defineOptions({ name: 'ClientVersionPolicyCard' })

withDefaults(
  defineProps<{
    loading?: boolean
    desktopError?: string
    cliError?: string
  }>(),
  {
    loading: false,
    desktopError: '',
    cliError: '',
  },
)

const emit = defineEmits<{
  help: []
}>()

const { t } = useI18n({ useScope: 'global' })

const minCodexDesktopVersion = defineModel<string>('minCodexDesktopVersion', { required: true })
const minCodexCliVersion = defineModel<string>('minCodexCliVersion', { required: true })
</script>

<template>
  <ZCard>
    <template #title>
      <span class="inline-flex items-center gap-1.5">
        <span>{{ t('settings.clientVersion.clientVersionRestrictions') }}</span>
        <ZIconButton
          :aria-label="t('settings.clientVersion.openGuide')"
          :title="undefined"
          class="size-6! hover:bg-transparent! active:bg-transparent!"
          @click="emit('help')"
        >
          <CircleHelp class="size-3.5" aria-hidden="true" />
        </ZIconButton>
      </span>
    </template>

    <ZForm class="max-w-6xl sm:grid-cols-2">
      <ZFormItem
        :label="t('settings.clientVersion.codexDesktopMinimumVersion')"
        :description="t('settings.clientVersion.onlyCheckDesktopVersionEG')"
        :error="desktopError"
      >
        <ZInput
          v-model="minCodexDesktopVersion"
          :aria-label="t('settings.clientVersion.codexDesktopMinimumVersion')"
          autocomplete="off"
          spellcheck="false"
          :placeholder="t('common.values.unlimited')"
          :disabled="loading"
        >
          <template #prefix>
            <MonitorUp class="size-4" />
          </template>
        </ZInput>
      </ZFormItem>

      <ZFormItem
        :label="t('settings.clientVersion.codexCliMinimumVersion')"
        :description="t('settings.clientVersion.minimumVersionHelp')"
        :error="cliError"
      >
        <ZInput
          v-model="minCodexCliVersion"
          :aria-label="t('settings.clientVersion.codexCliMinimumVersion')"
          autocomplete="off"
          spellcheck="false"
          :placeholder="t('common.values.unlimited')"
          :disabled="loading"
        >
          <template #prefix>
            <TerminalSquare class="size-4" />
          </template>
        </ZInput>
      </ZFormItem>
    </ZForm>
  </ZCard>
</template>
