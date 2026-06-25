export interface Service {
  name: string
  unit: string
  kind: string
  active: boolean
  version: string | null
}

export interface MetricsResp {
  points: [number, number][]
}

export interface LogsResp {
  lines: string[]
}

export interface Alert {
  ts_ms: number
  rule: string
  message: string
}

export interface AuditRow {
  ts_ms: number
  actor: string
  op: string
  params: string
  result: string
  detail: string
}

export interface Upgrades {
  current: string | null
  candidate: string | null
  target: string | null
  deadline: string | null
  rollback_point: string | null
}

export interface Me {
  actor: string
}
