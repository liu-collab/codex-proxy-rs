import type { Ref } from 'vue'
import { useConfirmDialog, useEventListener } from '@vueuse/core'
import { onScopeDispose, shallowRef, watch } from 'vue'
import { onBeforeRouteLeave } from 'vue-router'

export function useSettingsLeaveGuard(hasChanges: Readonly<Ref<boolean>>) {
  const showLeaveConfirm = shallowRef(false)
  const { reveal, confirm: confirmLeave, cancel: cancelLeave } = useConfirmDialog(showLeaveConfirm)
  let pending: Promise<boolean> | undefined

  onBeforeRouteLeave((to) => {
    if (to.matched.some(record => record.path === '/settings') || !hasChanges.value)
      return true
    pending ??= reveal().then(result => !result.isCanceled).finally(() => {
      pending = undefined
    })
    return pending
  })

  // 关闭按钮、遮罩与 Esc 都按继续编辑处理，不能留下未完成的路由导航
  watch(showLeaveConfirm, (open) => {
    if (!open)
      cancelLeave()
  })
  useEventListener(() => hasChanges.value ? window : undefined, 'beforeunload', (event: BeforeUnloadEvent) => {
    event.preventDefault()
    event.returnValue = ''
  })
  onScopeDispose(cancelLeave)

  return { showLeaveConfirm, confirmLeave }
}
