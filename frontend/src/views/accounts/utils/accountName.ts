export function accountNameUpdate(name: string, savedName: string): string | undefined {
  // 未修改的导入名称可能不符合新增约束，省略字段以保留原值
  if (name === savedName || name.trim() === savedName)
    return undefined
  return name.trim()
}

export function accountNameValid(name: string, savedName?: string): boolean {
  const value = savedName === undefined
    ? name.trim()
    : accountNameUpdate(name, savedName)
  return value === undefined || (new TextEncoder().encode(value).length <= 512 && !/\p{Cc}/u.test(value))
}
