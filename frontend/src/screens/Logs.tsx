import { useState } from 'react'
import { useStatus, useLogs } from '../api/hooks'
import { LogViewer } from '../components/LogViewer'
import { ApiError } from '../api/client'
import type { Service } from '../api/types'

const LEVEL_OPTIONS = ['', 'ERROR', 'WARN', 'INFO', 'DEBUG']
const DEFAULT_LINES = 200

const selectClass = 'bg-panel border border-line rounded px-2 py-[5px] font-mono text-[12px] text-ink focus:outline-none focus:border-acc'

function LogsBody({ unit, lines, levelFilter }: { unit: string; lines: number; levelFilter: string }) {
  const { data, isLoading, error } = useLogs(unit, lines)

  if (isLoading) {
    return <div className="text-mut text-[12.5px]">Loading logs…</div>
  }

  if (error) {
    const msg = error instanceof ApiError
      ? `Error ${error.status}: ${error.message}`
      : error instanceof Error
        ? error.message
        : 'Failed to load logs'
    return (
      <div role="alert" className="px-[14px] py-3 rounded-[10px] border border-dgr text-dgr text-[12.5px]" style={{ background: 'rgba(214,69,69,0.08)' }}>
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
  const activeUnit = selectedUnit || managedUnits[0]?.unit || ''

  return (
    <div className="px-5 pt-4 pb-8 space-y-3.5">
      <div className="flex flex-wrap items-center gap-3">
        <h1 className="text-[15px] font-semibold flex-1">Logs</h1>

        <label className="flex items-center gap-2 text-[11.5px] text-mut">
          Unit
          <select
            aria-label="Unit"
            value={activeUnit}
            onChange={e => setSelectedUnit(e.target.value)}
            className={selectClass}
          >
            {managedUnits.map(svc => (
              <option key={svc.unit} value={svc.unit}>{svc.name}</option>
            ))}
          </select>
        </label>

        <label className="flex items-center gap-2 text-[11.5px] text-mut">
          Level
          <select
            aria-label="Level filter"
            value={levelFilter}
            onChange={e => setLevelFilter(e.target.value)}
            className={selectClass}
          >
            {LEVEL_OPTIONS.map(l => (
              <option key={l} value={l}>{l || 'All'}</option>
            ))}
          </select>
        </label>

        <span className="font-mono text-[11px] text-mut">tail -n {DEFAULT_LINES} · poll 5s</span>
      </div>

      {managedUnits.length === 0 ? (
        <div className="text-mut text-[12.5px]">Loading units…</div>
      ) : (
        <LogsBody unit={activeUnit} lines={DEFAULT_LINES} levelFilter={levelFilter} />
      )}
    </div>
  )
}
