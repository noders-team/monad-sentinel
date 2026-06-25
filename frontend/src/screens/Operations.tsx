import { useAudit } from '../api/hooks'
import type { AuditRow } from '../api/types'
import { DataTable } from '../components/DataTable'
import type { Column } from '../components/DataTable'

function resultClassName(result: string): string {
  if (result === 'ok') return 'text-ok font-medium'
  if (result === 'denied' || result === 'error') return 'text-danger font-medium'
  return 'text-warn font-medium'
}

const columns: Column<AuditRow>[] = [
  {
    key: 'ts_ms',
    header: 'Time',
    render: (row) => (
      <span className="text-muted font-mono text-xs whitespace-nowrap">
        {new Date(row.ts_ms).toLocaleString()}
      </span>
    ),
  },
  {
    key: 'actor',
    header: 'Actor',
    render: (row) => <span className="font-mono text-xs">{row.actor}</span>,
  },
  {
    key: 'op',
    header: 'Op',
    render: (row) => <span className="font-mono text-xs">{row.op}</span>,
  },
  {
    key: 'result',
    header: 'Result',
    render: (row) => (
      <span
        className={resultClassName(row.result)}
        data-result={row.result}
      >
        {row.result}
      </span>
    ),
  },
  {
    key: 'detail',
    header: 'Detail',
    render: (row) => <span className="text-muted text-xs">{row.detail}</span>,
  },
]

export function Operations() {
  const { data, isLoading, isError } = useAudit()

  if (isLoading) {
    return (
      <div className="p-6 text-muted text-sm">Loading…</div>
    )
  }

  if (isError) {
    return (
      <div className="p-6 text-danger text-sm">Failed to load audit log</div>
    )
  }

  const sorted = data ? [...data].sort((a, b) => b.ts_ms - a.ts_ms) : []

  return (
    <div className="p-6 space-y-4 text-ink">
      <h2 className="text-sm font-semibold text-muted uppercase tracking-wider">
        Audit Log
      </h2>
      <DataTable<AuditRow>
        columns={columns}
        rows={sorted}
        rowKey={(row) => `${row.ts_ms}-${row.actor}-${row.op}`}
        empty="No audit entries"
      />
    </div>
  )
}
