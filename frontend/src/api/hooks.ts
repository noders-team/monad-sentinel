import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { apiGet, apiPost } from './client'
import type { Service, MetricsResp, LogsResp, Alert, AuditRow, Upgrades, Me } from './types'

export const useMe = () =>
  useQuery({ queryKey: ['me'], queryFn: () => apiGet<Me>('/api/auth/me'), retry: false })

export const useStatus = () =>
  useQuery({ queryKey: ['status'], queryFn: () => apiGet<Service[]>('/api/status'), refetchInterval: 5000 })

export const useMetrics = (name: string, window: string) =>
  useQuery({
    queryKey: ['metrics', name, window],
    queryFn: () => apiGet<MetricsResp>(`/api/metrics?name=${encodeURIComponent(name)}&window=${encodeURIComponent(window)}`),
    refetchInterval: 5000,
  })

export const useLogs = (unit: string, lines: number) =>
  useQuery({
    queryKey: ['logs', unit, lines],
    queryFn: () => apiGet<LogsResp>(`/api/logs?unit=${encodeURIComponent(unit)}&lines=${lines}`),
    refetchInterval: 5000,
  })

export const useAlerts = () =>
  useQuery({ queryKey: ['alerts'], queryFn: () => apiGet<Alert[]>('/api/alerts'), refetchInterval: 10000 })

export const useAudit = () =>
  useQuery({ queryKey: ['audit'], queryFn: () => apiGet<AuditRow[]>('/api/audit'), refetchInterval: 15000 })

export const useUpgrades = () =>
  useQuery({ queryKey: ['upgrades'], queryFn: () => apiGet<Upgrades>('/api/upgrades'), refetchInterval: 15000 })

export const useLogin = () =>
  useMutation({
    mutationFn: (v: { username: string; password: string }) =>
      apiPost<{ actor: string }>('/api/auth/login', v),
  })

export const useLogout = () => {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: () => apiPost<{ ok: boolean }>('/api/auth/logout', {}),
    onSuccess: () => { qc.clear() },
  })
}

export const useRestart = () => {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (v: { unit: string; totp: string }) =>
      apiPost<{ ok: boolean; detail: string }>('/api/ops/restart', v),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['status'] })
      void qc.invalidateQueries({ queryKey: ['audit'] })
    },
  })
}

export const useUpgrade = () => {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (v: { target_version: string; totp: string }) =>
      apiPost<{ ok: boolean; detail: string }>('/api/ops/upgrade', v),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['upgrades'] })
      void qc.invalidateQueries({ queryKey: ['audit'] })
    },
  })
}

export const useRollback = () => {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (v: { totp: string }) =>
      apiPost<{ ok: boolean; detail: string }>('/api/ops/rollback', v),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['upgrades'] })
      void qc.invalidateQueries({ queryKey: ['audit'] })
    },
  })
}

export const useSetPlan = () => {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (v: { target_version: string; deadline: string }) =>
      apiPost<{ ok: boolean }>('/api/upgrades/plan', v),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['upgrades'] })
    },
  })
}
