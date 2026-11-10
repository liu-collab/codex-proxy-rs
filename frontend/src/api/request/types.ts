import type { AxiosRequestConfig, AxiosResponse } from 'axios'

export interface RequestOptions {
  // 静默只关闭提示，不吞掉异常或绕过会话恢复
  silent?: boolean
  signal?: AbortSignal
  timeout?: number
  // 读取默认重试，写操作须由调用方确认幂等后开启
  retry?: boolean
  // 认证接口自身跳过会话恢复，避免循环等待
  skipSessionRecovery?: boolean
}

export type RequestConfig = AxiosRequestConfig & RequestOptions

export interface RawResponseMeta {
  status: number
  header: (name: string) => string | undefined
}

export interface RawResponse {
  status: number
  contentType: string
  body: ArrayBuffer
}

export interface RawResponseOptions {
  // 原始字节或宿主错误的正文上限；成功流由消费方约束，不在请求层缓冲
  maximumBytes: number
  accept: (response: RawResponseMeta) => boolean
}

export interface RequestContext {
  config: RequestConfig
  raw?: RawResponseOptions
  stream?: boolean
  // 会话插件补充身份代际检查，重试与提示消费同一请求有效性
  isCurrent: () => boolean
}

export type RequestNext = () => Promise<AxiosResponse<unknown>>

export type RequestPlugin = (context: RequestContext, next: RequestNext) => Promise<AxiosResponse<unknown>>
