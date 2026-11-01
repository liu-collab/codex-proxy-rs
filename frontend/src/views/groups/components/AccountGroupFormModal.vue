<script setup lang="ts">
import type { FormRules } from '@codex-proxy/ui'
import type { AccountGroupFormValue } from '../composables/useAccountGroups'

import type { AccountGroup } from '@/api'
import { normalizeRgbaHexColor, ZButton, ZColorPicker, ZDialog, ZForm, ZFormItem, ZInput, ZSegmented, ZTextarea } from '@codex-proxy/ui'
import { computed, useTemplateRef, watch } from 'vue'

import { useI18n } from 'vue-i18n'
import { ACCOUNT_GROUP_COLOR_PRESETS } from '../constants'

const props = defineProps<{
  group: AccountGroup | null
  saving: boolean
}>()

const emit = defineEmits<{
  save: []
}>()

const { t } = useI18n({ useScope: 'global' })

const open = defineModel<boolean>({ required: true })
const form = defineModel<AccountGroupFormValue>('form', { required: true })
const formRef = useTemplateRef<InstanceType<typeof ZForm>>('formElement')
const rules = computed<FormRules<AccountGroupFormValue>>(() => ({
  name: {
    required: true,
    whitespace: true,
    trigger: ['blur', 'change'],
    message: t('groups.actions.enterGroupName'),
  },
  color: {
    validator: (_rule, value) => !!normalizeRgbaHexColor(value),
    trigger: 'change',
    message: t('groups.actions.selectValidGroupingColor'),
  },
}))
watch(open, () => formRef.value?.clearValidate())

async function requestSave() {
  if (props.saving || !await formRef.value?.validate().catch(() => false))
    return
  emit('save')
}
const title = computed(() => props.group
  ? t('groups.actions.editGroup')
  : t('groups.filters.createGroup'))
const description = computed(() => props.group
  ? t('groups.editor.editDescription')
  : t('groups.editor.createDescription'))
</script>

<template>
  <ZDialog
    v-model="open"
    :title="title"
    :description="description"
    width="36rem"
    :show-close="!saving" :close-on-click-modal="!saving" :close-on-press-escape="!saving"
  >
    <ZForm ref="formElement" :model="form" :rules="rules" scroll-to-error focus-on-error novalidate @submit="requestSave">
      <ZFormItem prop="name" :show-message="false" :label="t('groups.editor.groupName')" required>
        <ZInput
          v-model="form.name"
          :aria-label="t('groups.editor.groupName')"
          :placeholder="t('groups.editor.forExampleProductionAccount')"
          :disabled="saving"
        />
      </ZFormItem>
      <ZFormItem prop="color" :show-message="Boolean(form.color.trim())" :label="t('groups.editor.groupColor')" required>
        <ZColorPicker
          v-model="form.color"
          :aria-label="t('groups.editor.selectGroupColor')"
          :presets="ACCOUNT_GROUP_COLOR_PRESETS"
          :disabled="saving"
        />
      </ZFormItem>
      <div class="flex flex-wrap items-start gap-x-6 gap-y-3">
        <ZFormItem :label="t('groups.editor.fastMode')" class="min-w-0">
          <ZSegmented
            v-model="form.fastMode"
            class="w-64 max-w-full"
            :aria-label="t('groups.editor.fastMode')"
            :options="[
              { label: t('common.values.default'), value: 'default' },
              { label: t('common.status.enabled'), value: 'enabled' },
              { label: t('common.status.disabled'), value: 'disabled' },
            ]"
            :disabled="saving"
          />
        </ZFormItem>
        <ZFormItem :label="t('groups.editor.imageGeneration')" class="min-w-0">
          <ZSegmented
            :model-value="form.allowImageGeneration ? 'enabled' : 'disabled'"
            class="w-40 max-w-full"
            :aria-label="t('groups.editor.imageGeneration')"
            :options="[
              { label: t('common.status.enabled'), value: 'enabled' },
              { label: t('common.status.disabled'), value: 'disabled' },
            ]"
            :disabled="saving"
            @update:model-value="form.allowImageGeneration = $event === 'enabled'"
          />
        </ZFormItem>
      </div>
      <ZFormItem :label="t('groups.editor.descriptionOptional')">
        <ZTextarea
          v-model="form.description"
          :aria-label="t('groups.editor.groupDescription')"
          :rows="4"
          :placeholder="t('groups.editor.describePurposeOfThisGroup')"
          :disabled="saving"
        />
      </ZFormItem>
    </ZForm>

    <template #footer>
      <ZButton :disabled="saving" @click="open = false">
        {{ t('common.actions.cancel') }}
      </ZButton>
      <ZButton
        type="primary"
        :loading="saving"
        @click="requestSave"
      >
        {{ t('groups.editor.saveGroup') }}
      </ZButton>
    </template>
  </ZDialog>
</template>
