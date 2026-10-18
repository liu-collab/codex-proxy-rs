import type { OutboundProxyRecord, ProxyBatchResult } from '@/api'
import { ApiError } from '@/api/request'

export const MAX_BATCH_CREATE = 100
export const MAX_BATCH_INPUT_BYTES = 512 * 1024
export const proxyProtocolOptions = [
  { label: 'HTTP', value: 'http' },
  { label: 'HTTPS', value: 'https' },
  { label: 'SOCKS5', value: 'socks5' },
  { label: 'SOCKS5H', value: 'socks5h' },
]

export type ProxyBatchStatus = 'pending' | 'running' | 'succeeded' | 'failed' | 'error' | 'conflict' | 'busy' | 'unknown' | 'skipped'

export interface ProxyBatchItem {
  key: string
  name: string
  endpoint: string
  status: ProxyBatchStatus
  message: string
  record?: OutboundProxyRecord
}

export interface ProxyBatchInput {
  line: number
  raw: string
  name: string
  proxyUrl: string
  endpoint: string
  error: string
}

export const proxyBatchStates: Record<ProxyBatchStatus, { label: string, tone: string }> = {
  pending: { label: '待执行', tone: 'text-cp-text-secondary' },
  running: { label: '执行中', tone: 'text-cp-info-text' },
  succeeded: { label: '成功', tone: 'text-cp-success-text' },
  failed: { label: '失败', tone: 'text-cp-error-text' },
  error: { label: '请求失败', tone: 'text-cp-error-text' },
  conflict: { label: '配置冲突', tone: 'text-cp-warning-text' },
  busy: { label: '测试繁忙', tone: 'text-cp-warning-text' },
  unknown: { label: '结果待确认', tone: 'text-cp-warning-text' },
  skipped: { label: '未执行', tone: 'text-cp-text-secondary' },
}

// 简写只转换录入格式，完整 URL 原样交给后端的 OutboundProxy 校验
function proxyUrlFromLine(value: string, protocol: string): string {
  if (/^[a-z][a-z\d+.-]*:\/\//iu.test(value))
    return value
  const fields = /^(\[[^\]]+\]|[^:\s]+):(\d+)(?::([^:]*):(.*))?$/u.exec(value)
  if (!fields)
    throw new Error('代理格式不正确')
  const [, host, port, username, password] = fields
  const url = new URL(`${protocol}://${host}:${port}`)
  if (url.username || url.password || (url.pathname && url.pathname !== '/') || url.search || url.hash)
    throw new Error('代理主机或端口不合法')
  url.username = encodeURIComponent(username ?? '')
  url.password = encodeURIComponent(password ?? '')
  return url.toString()
}

function safeEndpoint(proxyUrl: string): string {
  try {
    const url = new URL(proxyUrl)
    url.username = ''
    url.password = ''
    url.pathname = ''
    url.search = ''
    url.hash = ''
    return url.toString()
  }
  catch {
    return ''
  }
}

export function parseProxyBatch(text: string, prefix: string, protocol: string): ProxyBatchInput[] {
  const seen = new Map<string, number>()
  let ordinal = 0
  return text.split(/\r\n|\n|\r/u).flatMap((raw, index) => {
    const value = raw.trim()
    if (!value)
      return []
    const line = index + 1
    const name = `${prefix.trim() || '代理'}-${String(++ordinal).padStart(3, '0')}`
    let proxyUrl = ''
    let error = ''
    try {
      proxyUrl = proxyUrlFromLine(value, protocol)
      const earlier = seen.get(proxyUrl)
      if (earlier !== undefined)
        error = `与第 ${earlier} 行重复`
      else
        seen.set(proxyUrl, line)
    }
    catch {
      // 解析器的异常可能携带认证信息，界面只显示固定格式说明
      error = '格式不正确，请参考格式说明'
    }
    return [{ line, raw: value, name, proxyUrl, endpoint: safeEndpoint(proxyUrl), error }]
  })
}

export function proxyBatchInputError(text: string, count: number): string {
  if (new TextEncoder().encode(text).length > MAX_BATCH_INPUT_BYTES)
    return '代理列表不能超过 512 KiB'
  if (count > MAX_BATCH_CREATE)
    return `一次最多添加 ${MAX_BATCH_CREATE} 条代理`
  return ''
}

export function proxyRequestFailure(error: unknown): { status: ProxyBatchStatus, message: string, stop: boolean } {
  if (!(error instanceof ApiError))
    return { status: 'unknown', message: '未取得确定结果，请刷新列表核对', stop: true }
  if (error.status === 401 || error.status === 403)
    return { status: 'error', message: error.message, stop: true }
  if (error.kind === 'timeout' || error.kind === 'network' || error.kind === 'cancelled' || error.status >= 500)
    return { status: 'unknown', message: '请求中断或服务异常，请刷新列表核对结果', stop: true }
  if (error.status === 409)
    return { status: 'conflict', message: error.message, stop: false }
  if (error.status === 429)
    return { status: 'busy', message: error.message, stop: false }
  return { status: 'error', message: error.message, stop: false }
}

export function proxyBatchFailure(result: ProxyBatchResult<unknown> | undefined) {
  if (result?.status === 'skipped')
    return { status: 'skipped' as const, message: '前序操作异常，未执行', stop: true }
  return proxyRequestFailure(result?.status === 'failed'
    ? new ApiError(result.message, result.httpStatus, result.code)
    : undefined)
}

export function proxyResultData<T>(result: ProxyBatchResult<T> | undefined): T {
  if (result?.status === 'succeeded')
    return result.data
  throw new Error(proxyBatchFailure(result).message)
}
