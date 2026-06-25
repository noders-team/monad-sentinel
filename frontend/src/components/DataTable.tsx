import type { ReactNode, JSX } from 'react'

export interface Column<T> {
  key: string
  header: string
  render: (row: T) => ReactNode
}

export interface DataTableProps<T> {
  columns: Column<T>[]
  rows: T[]
  rowKey: (row: T) => string | number
  empty?: string
}

export function DataTable<T>({
  columns,
  rows,
  rowKey,
  empty = 'No data',
}: DataTableProps<T>): JSX.Element {
  if (rows.length === 0) {
    return (
      <div className="flex items-center justify-center py-12 text-muted text-sm">
        {empty}
      </div>
    )
  }

  return (
    <div className="overflow-x-auto rounded-xl border border-edge bg-panel">
      <table className="w-full text-sm text-ink">
        <thead>
          <tr className="border-b border-edge">
            {columns.map(col => (
              <th
                key={col.key}
                className="px-4 py-2 text-left text-xs font-semibold text-muted uppercase tracking-wider"
              >
                {col.header}
              </th>
            ))}
          </tr>
        </thead>
        <tbody className="divide-y divide-edge">
          {rows.map(row => (
            <tr key={rowKey(row)} className="hover:bg-bg/40 transition-colors">
              {columns.map(col => (
                <td key={col.key} className="px-4 py-2 align-top">
                  {col.render(row)}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
