import { useObjectUrl } from '@vueuse/core'
import { nextTick, shallowRef } from 'vue'

export function useDownload() {
  const object = shallowRef<Blob | null>(null)
  const objectUrl = useObjectUrl(object)

  async function downloadJson(payload: unknown, fileName: string) {
    return downloadBlob(new Blob([`${JSON.stringify(payload, null, 2)}\n`], {
      type: 'application/json;charset=utf-8',
    }), fileName)
  }

  async function downloadBlob(blob: Blob, fileName: string) {
    object.value = blob
    await nextTick()

    if (!objectUrl.value)
      return

    const link = document.createElement('a')
    link.href = objectUrl.value
    link.download = fileName
    document.body.appendChild(link)
    link.click()
    link.remove()
  }

  return {
    downloadJson,
    downloadBlob,
  }
}
