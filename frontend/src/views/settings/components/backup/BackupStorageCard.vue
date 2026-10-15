<script setup lang="ts">
import { ZButton, ZCard, ZCheckbox, ZForm, ZFormItem, ZInput } from '@codex-proxy/ui'

import { CircleAlert, CircleCheck, DatabaseZap, Save } from '@lucide/vue'

interface StorageForm {
  endpoint: string
  region: string
  bucket: string
  accessKeyId: string
  secretAccessKey: string
  prefix: string
  forcePathStyle: boolean
}

defineProps<{
  disabled: boolean
  saving: boolean
  testing: boolean
  verified: boolean
}>()

const emit = defineEmits<{
  save: []
  test: []
  openR2Guide: []
}>()

const storage = defineModel<StorageForm>('storage', { required: true })

function updateStorage<Key extends keyof StorageForm>(key: Key, value: StorageForm[Key]) {
  storage.value = { ...storage.value, [key]: value }
}
</script>

<template>
  <ZCard title="S3 存储配置">
    <template #description>
      <span class="text-cp-text-secondary">
        配置 S3 兼容存储（支持
        <button
          type="button"
          class="cursor-pointer border-0 bg-transparent p-0 text-cp-link underline underline-offset-2 hover:text-cp-link-hover"
          @click="emit('openR2Guide')"
        >
          Cloudflare R2
        </button>
        ）
      </span>
    </template>

    <template #actions>
      <div class="flex flex-wrap items-center gap-2">
        <ZButton
          :loading="testing"
          :disabled="disabled"
          :title="verified ? '已通过连接测试' : '尚未通过连接测试'"
          @click="emit('test')"
        >
          <template #icon>
            <CircleCheck v-if="verified" class="size-4 text-cp-success-text" />
            <CircleAlert v-else class="size-4 text-cp-warning-text" />
          </template>
          {{ testing ? '测试中...' : '测试连接' }}
        </ZButton>
        <ZButton type="primary" :loading="saving" :disabled="disabled" @click="emit('save')">
          <template #icon>
            <Save class="size-4" />
          </template>
          {{ saving ? '保存中...' : '保存' }}
        </ZButton>
      </div>
    </template>

    <div class="@container">
      <ZForm class="max-w-6xl @min-[640px]:grid-cols-2">
        <ZFormItem label="端点地址" description="S3 兼容服务的 HTTPS 地址">
          <ZInput
            :model-value="storage.endpoint" :disabled="disabled"
            aria-label="端点地址"
            placeholder="https://<account_id>.r2.cloudflarestorage.com"
            @update:model-value="updateStorage('endpoint', $event)"
          >
            <template #prefix>
              <DatabaseZap class="size-4" />
            </template>
          </ZInput>
        </ZFormItem>

        <ZFormItem label="区域" description="R2 使用固定值 auto，其它服务按提供方填写">
          <ZInput :model-value="storage.region" :disabled="disabled" aria-label="区域" @update:model-value="updateStorage('region', $event)" />
        </ZFormItem>

        <ZFormItem label="存储桶" description="私有存储桶名称">
          <ZInput :model-value="storage.bucket" :disabled="disabled" aria-label="存储桶" @update:model-value="updateStorage('bucket', $event)" />
        </ZFormItem>

        <ZFormItem label="对象键前缀" description="备份对象的存储路径前缀，不影响已有备份">
          <ZInput :model-value="storage.prefix" :disabled="disabled" aria-label="对象键前缀" @update:model-value="updateStorage('prefix', $event)" />
        </ZFormItem>

        <ZFormItem label="Access Key ID" description="对象存储专用凭据">
          <ZInput
            :model-value="storage.accessKeyId" :disabled="disabled"
            aria-label="Access Key ID"
            show-password
            autocomplete="off"
            @update:model-value="updateStorage('accessKeyId', $event)"
          />
        </ZFormItem>

        <ZFormItem label="Secret Access Key" description="对象存储专用 Secret">
          <ZInput
            :model-value="storage.secretAccessKey" :disabled="disabled"
            aria-label="Secret Access Key"
            show-password
            autocomplete="new-password"
            @update:model-value="updateStorage('secretAccessKey', $event)"
          />
        </ZFormItem>

        <div class="col-span-2 flex items-center gap-4 @max-[640px]:col-span-1">
          <ZCheckbox :model-value="storage.forcePathStyle" :disabled="disabled" label="强制路径式访问" @update:model-value="updateStorage('forcePathStyle', $event)" />
        </div>
      </ZForm>
    </div>
  </ZCard>
</template>
