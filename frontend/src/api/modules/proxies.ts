import type { RequestOptions } from '../request'
import type { AccountGroupRef } from './groups'
import type { RequestLocation } from './settings'
import request from '../request'

export type ProxyLocationDetection
  = | { status: 'notRequested' }
    | { status: 'detected', location: RequestLocation }
    | { status: 'failed', message: string }
    | { status: 'conflict' }

export interface DetectedProxyLocation {
  location: RequestLocation
  exitIpv4: string | null
  exitIpv6: string | null
  detectedAt: string
}

export interface OutboundProxyTest {
  location: ProxyLocationDetection
  success: boolean
  latencyMs: number
  exitIp: string | null
  exitIpv4: string | null
  exitIpv6: string | null
  message: string
}

export interface OutboundProxyRecord {
  autoLocation: boolean
  detectedLocation: DetectedProxyLocation | null
  location: RequestLocation | null
  id: string
  name: string
  endpoint: string
  hasAuthentication: boolean
  revision: number
  accountCount: number
  lastTestAt: string | null
  lastTestAtDisplay: string | null
  lastTest: OutboundProxyTest | null
  createdAt: string
  createdAtDisplay: string
  updatedAt: string
  updatedAtDisplay: string
}

/** 代理地址可选的协议；与后端 `OutboundProxy::parse` 的白名单一致。 */
export const PROXY_PROTOCOL_OPTIONS = [
  { label: 'SOCKS5', value: 'socks5' },
  { label: 'SOCKS5H', value: 'socks5h' },
  { label: 'HTTP', value: 'http' },
  { label: 'HTTPS', value: 'https' },
]

/** 从已保存代理的脱敏地址里取回协议，用于编辑时回显；取不到时用 SOCKS5。 */
export function proxyProtocolOf(endpoint: string): string {
  const separator = endpoint.indexOf('://')
  const protocol = separator > 0 ? endpoint.slice(0, separator).toLowerCase() : ''
  return PROXY_PROTOCOL_OPTIONS.some(option => option.value === protocol) ? protocol : 'socks5'
}

interface ProxyPage {
  items: OutboundProxyRecord[]
  page: { page: number, pageSize: number, total: number, totalPages: number }
}

export interface OutboundProxyAccount {
  id: string
  name: string
  email: string | null
  provider: string
  authenticationKind: string
  planType: string | null
  planTypeDisplay: string | null
  groups: AccountGroupRef[]
  enabled: boolean
}

interface ProxyAccountPage {
  items: OutboundProxyAccount[]
  page: ProxyPage['page']
}

export function getProxyAccounts(data: { proxyId: string, page: number, pageSize: number, search?: string }, options: RequestOptions = {}) {
  return request<ProxyAccountPage>({
    url: '/api/admin/proxies/accounts',
    method: 'GET',
    params: data,
    ...options,
  })
}

export function removeProxyAccount(data: { proxyId: string, accountId: string }) {
  return request<{ configRevision: number }>({
    url: '/api/admin/proxies/accounts/remove',
    method: 'POST',
    data,
  })
}

interface ProxyMutation {
  record: OutboundProxyRecord
  configRevision: number
}

export function getProxies(data: { page: number, pageSize: number, search?: string }, options: RequestOptions = {}) {
  return request<ProxyPage>({
    url: '/api/admin/proxies',
    method: 'GET',
    params: data,
    ...options,
  })
}

export function createProxy(data: { name: string, proxyUrl: string, proxyProtocol?: string, autoLocation?: boolean, location?: RequestLocation | null }) {
  return request<ProxyMutation>({
    url: '/api/admin/proxies/create',
    timeout: 25000,
    method: 'POST',
    data,
  })
}

export function updateProxy(data: { id: string, revision: number, name: string, proxyUrl?: string, proxyProtocol?: string, autoLocation?: boolean, location?: RequestLocation | null }) {
  return request<ProxyMutation>({
    url: '/api/admin/proxies/update',
    timeout: 25000,
    method: 'POST',
    data,
  })
}

export function deleteProxy(data: { id: string, revision: number }) {
  return request<{ configRevision: number }>({
    url: '/api/admin/proxies/delete',
    method: 'POST',
    data,
  })
}

export function testProxy(data: { id: string, revision: number, detectLocation?: boolean }) {
  return request<OutboundProxyRecord>({
    url: '/api/admin/proxies/test',
    method: 'POST',
    data,
    timeout: 25000,
  })
}

export function probeProxy(data: { proxyUrl: string, proxyProtocol?: string, detectLocation?: boolean }) {
  return request<OutboundProxyTest>({
    url: '/api/admin/proxies/probe',
    method: 'POST',
    data,
    timeout: 25000,
  })
}
