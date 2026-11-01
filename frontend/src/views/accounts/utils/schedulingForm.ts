export function accountSchedulingValues(concurrencyLimit: string, weight: string) {
  return {
    concurrencyLimit: concurrencyLimit.trim() === ''
      ? null
      : Number(concurrencyLimit),
    weight: Number(weight),
  }
}

export function concurrencyLimitInput(value: number | null) {
  return value === null
    ? ''
    : String(value)
}
