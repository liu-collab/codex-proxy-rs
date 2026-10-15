<script setup lang="ts">
import { ZButton, ZFormItem, ZTextarea } from '@codex-proxy/ui'
import { Upload } from '@lucide/vue'
import { useFileDialog } from '@vueuse/core'
import { onScopeDispose, ref } from 'vue'

defineProps<{
  label: string
  placeholder: string
  uploadable: boolean
  disabled: boolean
}>()
const text = defineModel<string>({ required: true })
const fileError = ref('')
const { open: openFile, onChange } = useFileDialog({ accept: 'application/json,.json', multiple: false, reset: true })

let readVersion = 0
onScopeDispose(() => {
  readVersion += 1
})

onChange(async (files) => {
  const file = files?.[0]
  if (!file)
    return
  const version = ++readVersion
  fileError.value = ''
  try {
    const contents = await file.text()
    if (version !== readVersion)
      return
    text.value = contents
  }
  catch {
    if (version === readVersion)
      fileError.value = '文件读取失败，请重新选择'
  }
})

function updateText(value: string) {
  text.value = value
  readVersion += 1
  fileError.value = ''
}
</script>

<template>
  <ZFormItem :label="label" required :error="fileError || undefined">
    <template v-if="uploadable" #extra>
      <ZButton size="small" :disabled="disabled" @click="openFile()">
        <template #icon>
          <Upload class="size-3.5" aria-hidden="true" />
        </template>
        上传文件
      </ZButton>
    </template>
    <ZTextarea
      :model-value="text"
      :aria-label="label"
      :rows="9"
      :placeholder="placeholder"
      :disabled="disabled"
      @update:model-value="updateText"
    />
  </ZFormItem>
</template>
