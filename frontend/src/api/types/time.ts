export type RelativeTimeView
  = { kind: 'just_now' }
    | { kind: 'elapsed', value: number, unit: 'minute' | 'hour' | 'day' }
    | { kind: 'absolute', value: string }
