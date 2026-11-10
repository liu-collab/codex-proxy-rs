export interface RequestLocation {
  country: string
  region: string
  city: string
  timezone: string
}

export type RequestLocationDetection
  = | { status: 'notRequested' }
    | { status: 'detected', location: RequestLocation }
    | { status: 'failed', message: string }
    | { status: 'conflict' }
