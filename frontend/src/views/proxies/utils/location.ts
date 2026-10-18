import type { OutboundProxyRecord } from '@/api'

const countryNames = new Intl.DisplayNames(['zh-CN'], { type: 'region', fallback: 'code' })
const flags = import.meta.glob<string>('/node_modules/circle-flags/flags/??.svg', { eager: true, import: 'default', query: '?url&no-inline' })

export function proxyExitLocation(proxy: OutboundProxyRecord) {
  const detected = proxy.detectedLocation
  if (!detected)
    return null
  const test = proxy.lastTest
  // 没有重新解析位置的单条测试也可能换出口，旧地区不能套到新的 IP 上
  if (test?.success && (test.exitIpv4 !== detected.exitIpv4 || test.exitIpv6 !== detected.exitIpv6))
    return null
  return detected.location
}

export function countryName(code: string): string {
  return /^[A-Z]{2}$/u.test(code) ? countryNames.of(code) ?? code : code
}

export function countryFlagUrl(code: string): string | undefined {
  return /^[A-Z]{2}$/u.test(code) ? flags[`/node_modules/circle-flags/flags/${code.toLowerCase()}.svg`] : undefined
}
