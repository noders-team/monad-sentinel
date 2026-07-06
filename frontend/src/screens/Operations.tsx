import { useAudit } from '../api/hooks'
import type { AuditRow } from '../api/types'
import { DataTable } from '../components/DataTable'
import type { Column } from '../components/DataTable'

function resultClass(result: string): string {
  if (result === 'ok') return 'text-ok font-semibold'
  if (result === 'denied' || result === 'error') return 'text-dgr font-semibold'
  return 'text-warn font-semibold'
}

const columns: Column<AuditRow>[] = [
  {
    key: 'ts_ms',
    header: 'When',
    render: (row) => (
      <span className="text-mut font-mono text-[11.5px] whitespace-nowrap">{new Date(row.ts_ms).toLocaleString()}</span>
    ),
  },
  {
    key: 'actor',
    header: 'Actor',
    render: (row) => <span className="font-mono text-[11.5px]">{row.actor}</span>,
  },
  {
    key: 'op',
    header: 'Operation',
    render: (row) => <span className="font-mono text-[11.5px] text-acc-ink">{row.op}</span>,
  },
  {
    key: 'result',
    header: 'Result',
    render: (row) => (
      <span className={`font-mono text-[11.5px] ${resultClass(row.result)}`} data-result={row.result}>
        {row.result}
      </span>
    ),
  },
  {
    key: 'detail',
    header: 'Detail',
    render: (row) => <span className="text-mut text-[11.5px]">{row.detail}</span>,
  },
]

export function Operations() {
  const { data, isLoading, isError } = useAudit()

  if (isLoading) return <div className="px-5 pt-4 text-mut text-[12.5px]">Loading…</div>
  if (isError) return <div className="px-5 pt-4 text-dgr text-[12.5px]">Failed to load audit log</div>

  const sorted = data ? [...data].sort((a, b) => b.ts_ms - a.ts_ms) : []

  return (
    <div className="px-5 pt-4 pb-8 space-y-3.5">
      <div className="flex flex-wrap items-baseline gap-2">
        <h1 className="text-[15px] font-semibold">Operations</h1>
        <span className="text-[11.5px] text-mut">every privileged call · TOTP + CSRF verified server-side</span>
      </div>
      <DataTable<AuditRow>
        columns={columns}
        rows={sorted}
        rowKey={(row) => `${row.ts_ms}-${row.actor}-${row.op}`}
        empty="No audit entries"
      />
    </div>
  )
}
