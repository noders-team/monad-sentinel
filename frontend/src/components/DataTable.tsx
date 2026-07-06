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
      <div className="bg-panel border border-line rounded-[10px] flex items-center justify-center py-12 text-mut text-[12.5px]">
        {empty}
      </div>
    )
  }

  return (
    <div className="overflow-x-auto rounded-[10px] border border-line bg-panel">
      <table className="w-full text-[12.5px] text-ink">
        <thead>
          <tr className="border-b border-line">
            {columns.map(col => (
              <th
                key={col.key}
                className="px-[14px] py-[9px] text-left text-[10.5px] font-semibold text-mut uppercase tracking-[0.08em]"
              >
                {col.header}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map(row => (
            <tr key={rowKey(row)} className="border-b border-line last:border-0">
              {columns.map(col => (
                <td key={col.key} className="px-[14px] py-[9px] align-top">
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
