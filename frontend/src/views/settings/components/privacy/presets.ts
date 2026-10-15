import type { PrivacyAction, PrivacyRule, PrivacySample, PrivacyScope, PrivacyStep } from '@/api/modules/settings/privacy'
import { cloneDeep } from 'es-toolkit'

export const scopes: { value: PrivacyScope, label: string }[] = [
  { value: 'turn_metadata', label: '工作区附加信息' },
  { value: 'desktop_git_context', label: 'Desktop Git 信息' },
  { value: 'environment_text', label: '自动环境上下文' },
  { value: 'request_body', label: '指定请求字段' },
  { value: 'request_header', label: '指定请求头' },
]
export const actions: { value: PrivacyAction, label: string }[] = [
  { value: 'regex_replace', label: '正则替换' },
  { value: 'set_value', label: '固定值' },
  { value: 'rename_key', label: '字段名替换' },
  { value: 'remove_field', label: '删除字段 / 数组项' },
]
export const fieldsByScope: Partial<Record<PrivacyScope, { value: string, label: string }[]>> = {
  turn_metadata: [
    { value: '', label: '自定义字段路径' },
    { value: '$.workspaces.*.associated_remote_urls.*', label: 'Git 远端地址' },
    { value: '$.workspaces', label: '工作区路径键' },
    { value: '$.workspaces.*.latest_git_commit_hash', label: 'Git 提交标识' },
  ],
  desktop_git_context: [
    { value: '', label: '自定义字段路径' },
    { value: '$.remotes', label: 'Git 远端集合' },
    { value: '$.remotes[*].fetchUrl', label: 'Git 远端地址' },
    { value: '$.headCommitSha', label: 'Git 提交标识' },
    { value: '$.worktreeId', label: '工作区指纹' },
  ],
}

export function newStep(): PrivacyStep {
  return { scope: 'turn_metadata', selector: '$.workspaces.*.associated_remote_urls.*', action: 'regex_replace', pattern: '', replacement: '', value: '', replaceAll: true, caseInsensitive: false, multiLine: false }
}

export function newRule(): PrivacyRule {
  return { id: Array.from(crypto.getRandomValues(new Uint8Array(16)), byte => byte.toString(16).padStart(2, '0')).join(''), name: '', enabled: false, steps: [newStep()] }
}

export const presets: { key: string, name: string, steps: Partial<PrivacyStep>[] }[] = [
  {
    key: 'workspace',
    name: '统一工作区信息',
    steps: [
      { scope: 'turn_metadata', selector: '$.workspaces', action: 'rename_key', pattern: String.raw`(?i)^((?:[a-z]:)?[/\\](?:home|users)[/\\])[^/\\]+`, replacement: `\${1}user` },
      { scope: 'environment_text', selector: '$', action: 'regex_replace', pattern: String.raw`(?i)((?:[a-z]:)?[/\\](?:home|users)[/\\])[^/\\<>"'\r\n]+`, replacement: `\${1}user` },
      // 路径隐藏本机目录布局，项目名和 Git 状态仍取自实际请求
      { scope: 'turn_metadata', selector: '$.workspaces', action: 'rename_key', pattern: String.raw`(?s)^(?:[a-zA-Z]:)?[/\\](?:.*[/\\])?([^/\\]+)[/\\]*$`, replacement: `/workspace/\${1}` },
      { scope: 'environment_text', selector: '$', action: 'regex_replace', pattern: String.raw`(<cwd>\s*)(?:[a-zA-Z]:)?[/\\](?:[^<>\r\n]*[/\\])?([^/\\<>\r\n]+?)[/\\]*\s*(</cwd>)`, replacement: `\${1}/workspace/\${2}\${3}` },
    ],
  },
]

export function createPreset(key: string): PrivacyRule {
  const preset = presets.find(item => item.key === key)!
  return { ...newRule(), name: preset.name, steps: preset.steps.map(step => ({ ...newStep(), ...cloneDeep(step) })) }
}

// 样本使用虚构仓库、路径与标识，字段形状对应官方 Core 和 Desktop Git 上下文
const metadata = { workspaces: { '/home/alex/projects/demo': { associated_remote_urls: { origin: 'https://git.example.invalid/private-org/repo.git', public: 'git@git.example.invalid:public/repo.git' }, latest_git_commit_hash: 'a'.repeat(40), has_changes: true } }, request_kind: 'turn' }
const desktopGit = {
  schemaVersion: 1,
  phase: 'before_turn_request',
  hostId: 'local',
  sessionId: 'example-session',
  threadId: 'example-thread',
  clientUserMessageId: 'example-message',
  appVersion: '26.1007.21434',
  appServerVersion: '0.162.0-alpha.17.2',
  status: 'partial',
  capturedAt: '2026-10-09T12:00:00Z',
  worktreeId: 'b'.repeat(40),
  headCommitSha: 'a'.repeat(40),
  branch: 'main',
  detached: false,
  upstream: { remoteName: 'origin', mergeRef: 'refs/heads/main', trackingRef: 'refs/remotes/origin/main' },
  remotes: [
    { name: 'origin', fetchUrl: 'https://git.example.invalid/private-org/repo.git' },
    { name: 'upstream', fetchUrl: 'ssh://git.example.invalid/public/repo.git' },
    { name: 'unavailable', fetchUrl: null, unavailableReason: 'unsafe_url' },
  ],
  remotesOmitted: 0,
}

function desktopContext(value: unknown): string {
  const text = JSON.stringify(value).replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;')
  return `<external_codex_git_state><environment_context><git source="app" trust="untrusted">${text}</git></environment_context></external_codex_git_state>`
}

function contextMessage(text: string) {
  return { role: 'user', content: [{ type: 'input_text', text }] }
}

export function exampleRequest(): PrivacySample {
  const environment = '<environment_context>\n<cwd>/home/alex/projects/demo</cwd>\n<filesystem><workspace_roots><root>/home/alex/projects/demo</root></workspace_roots></filesystem>\n<timezone>Asia/Shanghai</timezone>\n</environment_context>'
  return { body: { input: [contextMessage(environment), contextMessage(desktopContext(desktopGit))], client_metadata: { 'x-codex-turn-metadata': JSON.stringify(metadata) } }, headers: { 'x-codex-turn-metadata': [JSON.stringify(metadata)] }, turnMetadata: null }
}
