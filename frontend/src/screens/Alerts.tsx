import { useAlerts } from '../api/hooks'
import type { Alert } from '../api/types'
import { DataTable } from '../components/DataTable'
import type { Column } from '../components/DataTable'
import { Chip, Pill } from '../components/ui'

function formatRelativeTime(ts_ms: number): string {
  const diffSec = Math.floor((Date.now() - ts_ms) / 1000)
  if (diffSec < 60) return `${diffSec}s ago`
  const diffMin = Math.floor(diffSec / 60)
  if (diffMin < 60) return `${diffMin}m ago`
  const diffHour = Math.floor(diffMin / 60)
  if (diffHour < 24) return `${diffHour}h ago`
  return `${Math.floor(diffHour / 24)}d ago`
}

function severity(rule: string): 'crit' | 'warn' {
  return /loss|stall|absent|down/i.test(rule) ? 'crit' : 'warn'
}

const columns: Column<Alert>[] = [
  {
    key: 'sev',
    header: 'Severity',
    render: (row) => <Chip severity={severity(row.rule)}>{severity(row.rule)}</Chip>,
  },
  {
    key: 'rule',
    header: 'Rule',
    render: (row) => <span className="font-mono text-[11.5px] text-acc-ink">{row.rule}</span>,
  },
  {
    key: 'ts_ms',
    header: 'When',
    render: (row) => (
      <span className="text-mut font-mono text-[11.5px] whitespace-nowrap">{formatRelativeTime(row.ts_ms)}</span>
    ),
  },
  {
    key: 'message',
    header: 'Message',
    render: (row) => <span className="text-[12.5px] leading-[1.5]">{row.message}</span>,
  },
  {
    key: 'status',
    header: 'Status',
    render: () => <Pill kind="active">active</Pill>,
  },
]

export function Alerts() {
  const { data, isLoading, isError } = useAlerts()

  if (isLoading) return <div className="px-5 pt-4 text-mut text-[12.5px]">Loading…</div>
  if (isError) return <div className="px-5 pt-4 text-dgr text-[12.5px]">Failed to load alerts</div>

  const count = data?.length ?? 0

  return (
    <div className="px-5 pt-4 pb-8 space-y-3.5">
      <div className="flex flex-wrap items-baseline gap-2">
        <h1 className="text-[15px] font-semibold">Alerts</h1>
        <span className="text-[11.5px] text-mut">
          {count} active · rules synced with VDP thresholds
        </span>
      </div>
      <DataTable<Alert>
        columns={columns}
        rows={data ? [...data].sort((a, b) => b.ts_ms - a.ts_ms) : []}
        rowKey={(row) => `${row.ts_ms}-${row.rule}`}
        empty="No active alerts"
      />
    </div>
  )
}
