import { useAlerts } from '../api/hooks'
import type { Alert } from '../api/types'
import { DataTable } from '../components/DataTable'
import type { Column } from '../components/DataTable'

function formatRelativeTime(ts_ms: number): string {
  const diffMs = Date.now() - ts_ms
  const diffSec = Math.floor(diffMs / 1000)
  if (diffSec < 60) return `${diffSec}s ago`
  const diffMin = Math.floor(diffSec / 60)
  if (diffMin < 60) return `${diffMin}m ago`
  const diffHour = Math.floor(diffMin / 60)
  if (diffHour < 24) return `${diffHour}h ago`
  const diffDay = Math.floor(diffHour / 24)
  return `${diffDay}d ago`
}

const columns: Column<Alert>[] = [
  {
    key: 'ts_ms',
    header: 'When',
    render: (row) => (
      <span className="text-muted font-mono text-xs whitespace-nowrap">
        {formatRelativeTime(row.ts_ms)}
      </span>
    ),
  },
  {
    key: 'rule',
    header: 'Rule',
    render: (row) => (
      <span className="font-mono text-xs text-warn">{row.rule}</span>
    ),
  },
  {
    key: 'message',
    header: 'Message',
    render: (row) => <span className="text-sm">{row.message}</span>,
  },
]

export function Alerts() {
  const { data, isLoading, isError } = useAlerts()

  if (isLoading) {
    return (
      <div className="p-6 text-muted text-sm">Loading…</div>
    )
  }

  if (isError) {
    return (
      <div className="p-6 text-danger text-sm">Failed to load alerts</div>
    )
  }

  return (
    <div className="p-6 space-y-4 text-ink">
      <h2 className="text-sm font-semibold text-muted uppercase tracking-wider">
        Active Alerts
      </h2>
      <DataTable<Alert>
        columns={columns}
        rows={data ? [...data].sort((a, b) => b.ts_ms - a.ts_ms) : []}
        rowKey={(row) => `${row.ts_ms}-${row.rule}`}
        empty="No active alerts"
      />
    </div>
  )
}
