import { useState } from 'react'
import { useStatus, useLogs } from '../api/hooks'
import { LogViewer } from '../components/LogViewer'
import { ApiError } from '../api/client'
import type { Service } from '../api/types'

const LEVEL_OPTIONS = ['', 'ERROR', 'WARN', 'INFO', 'DEBUG']
const DEFAULT_LINES = 200

function LogsBody({ unit, lines, levelFilter }: { unit: string; lines: number; levelFilter: string }) {
  const { data, isLoading, error } = useLogs(unit, lines)

  if (isLoading) {
    return <div className="text-ink/40 text-sm">Loading logs…</div>
  }

  if (error) {
    const msg = error instanceof ApiError
      ? `Error ${error.status}: ${error.message}`
      : error instanceof Error
        ? error.message
        : 'Failed to load logs'
    return (
      <div role="alert" className="px-4 py-3 rounded-lg border border-red-500/40 bg-red-500/10 text-red-400 text-sm">
        {msg}
      </div>
    )
  }

  return <LogViewer lines={data?.lines ?? []} levelFilter={levelFilter} />
}

export function Logs() {
  const { data: services } = useStatus()
  const [selectedUnit, setSelectedUnit] = useState<string>('')
  const [levelFilter, setLevelFilter] = useState<string>('')

  const managedUnits: Service[] = services ?? []

  // Auto-select first unit when services load
  const activeUnit = selectedUnit || managedUnits[0]?.unit || ''

  return (
    <div className="p-6 space-y-4 text-ink">
      <h1 className="text-xl font-semibold">Logs</h1>

      <div className="flex flex-wrap gap-3 items-center">
        <label className="flex items-center gap-2 text-sm text-ink/70">
          Unit
          <select
            aria-label="Unit"
            value={activeUnit}
            onChange={e => setSelectedUnit(e.target.value)}
            className="bg-surface border border-line rounded px-2 py-1 text-sm text-ink focus:outline-none focus:border-neon/50"
          >
            {managedUnits.map(svc => (
              <option key={svc.unit} value={svc.unit}>
                {svc.name}
              </option>
            ))}
          </select>
        </label>

        <label className="flex items-center gap-2 text-sm text-ink/70">
          Level
          <select
            aria-label="Level filter"
            value={levelFilter}
            onChange={e => setLevelFilter(e.target.value)}
            className="bg-surface border border-line rounded px-2 py-1 text-sm text-ink focus:outline-none focus:border-neon/50"
          >
            {LEVEL_OPTIONS.map(l => (
              <option key={l} value={l}>
                {l || 'All'}
              </option>
            ))}
          </select>
        </label>
      </div>

      {managedUnits.length === 0 ? (
        <div className="text-ink/40 text-sm">Loading units…</div>
      ) : (
        <LogsBody unit={activeUnit} lines={DEFAULT_LINES} levelFilter={levelFilter} />
      )}
    </div>
  )
}
