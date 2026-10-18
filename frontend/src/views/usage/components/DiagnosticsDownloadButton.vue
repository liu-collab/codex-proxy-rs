<script setup lang="ts">
import { ZButton } from '@codex-proxy/ui'
import { Download } from '@lucide/vue'
import { onScopeDispose, watch } from 'vue'
import { downloadDiagnostics } from '@/api'
import { useAsyncAction } from '@/composables/useAsyncAction'
import { useDownload } from '@/composables/useDownload'

const props = defineProps<{ requestId?: string | null, errorId?: string | null, active: boolean }>()
const { downloadBlob } = useDownload()
const { loading, run } = useAsyncAction()
let controller: AbortController | undefined

watch(() => [props.requestId, props.errorId, props.active], () => controller?.abort())
onScopeDispose(() => controller?.abort())

function download() {
  const id = props.requestId || props.errorId
  if (!id || loading.value || !props.active)
    return
  const target = props.requestId ? { requestId: props.requestId } : { errorId: id }
  const current = new AbortController()
  controller = current
  return run(async () => {
    try {
      const file = await downloadDiagnostics(target, { signal: current.signal })
      if (!current.signal.aborted)
        await downloadBlob(new Blob([file.body], { type: file.contentType }), `diagnostics-${id.replace(/[^\w-]/g, '_')}.json`)
    }
    catch (cause) {
      if (!current.signal.aborted)
        throw cause
    }
    finally {
      if (controller === current)
        controller = undefined
    }
  }, { errorText: '下载诊断包失败', minimumMs: 400 })
}
</script>

<template>
  <ZButton type="primary" variant="plain" :loading="loading" :disabled="!requestId && !errorId" @click="download">
    <template #icon>
      <Download :size="16" />
    </template>
    {{ loading ? '正在生成诊断包' : '下载诊断包' }}
  </ZButton>
</template>
