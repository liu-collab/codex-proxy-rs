import type { ProxyTestExecutor } from './useProxyBatchTest'
import type { OutboundProxyRecord } from '@/api'
import { ZNotification } from '@codex-proxy/ui'
import { getProxies } from '@/api'
import { useRequestState } from '@/composables/useRequestState'
import { useProxyBatchTest } from './useProxyBatchTest'

export function useProxyBatchTesting(options: { test: ProxyTestExecutor, reload: () => Promise<unknown> }) {
  const request = useRequestState()
  const tests = useProxyBatchTest(options.test)

  async function start(ids: string[]) {
    if (request.loading.value || !ids.length)
      return
    const wanted = new Set(ids)
    const requestId = request.start()
    const collected = new Map<string, OutboundProxyRecord>()
    try {
      let totalPages = 1
      for (let page = 1; page <= totalPages; page += 1) {
        const result = await getProxies({ page, pageSize: 200 }, { silent: true, signal: request.signal })
        if (!request.isCurrent(requestId))
          return
        // 只读取首次分页给出的范围，完成后固定实际名单，不追随新增记录无限加载
        if (page === 1)
          totalPages = result.page.totalPages
        for (const proxy of result.items) {
          if (wanted.has(proxy.id))
            collected.set(proxy.id, proxy)
        }
        if (collected.size === wanted.size)
          break
      }
      if (!request.isCurrent(requestId))
        return
      if (!collected.size) {
        ZNotification.warning({ message: '所选代理已不存在，请刷新列表' })
        return
      }
      await tests.run([...collected.values()])
      if (!request.isCurrent(requestId))
        return
      await options.reload()
      if (!request.isCurrent(requestId))
        return
      const succeeded = tests.items.value.filter(item => item.status === 'succeeded').length
      const incomplete = tests.items.value.filter(item => item.status !== 'succeeded')
      const locationPending = tests.items.value.filter(item => item.status === 'succeeded' && item.record?.lastTest?.location.status !== 'detected').length
      const missing = wanted.size - collected.size
      const summary = [`批量测试完成，连接正常 ${succeeded} 条`]
      if (incomplete.length)
        summary.push(`未成功 ${incomplete.length} 条`)
      if (locationPending)
        summary.push(`地区待处理 ${locationPending} 条`)
      if (missing)
        summary.push(`${missing} 条代理已不存在`)
      const firstFailure = incomplete.find(item => item.status !== 'skipped')
      if (firstFailure)
        summary.push(`${firstFailure.name}：${firstFailure.message}`)
      const notify = incomplete.length || locationPending || missing ? ZNotification.warning : ZNotification.success
      notify({ message: summary.join('，') })
    }
    catch (cause) {
      request.fail(requestId, cause)
      if (request.isCurrent(requestId) && request.error.value)
        ZNotification.error({ message: request.error.value })
    }
    finally {
      request.finish(requestId)
    }
  }

  return { busy: request.loading, start }
}
