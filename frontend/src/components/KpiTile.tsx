interface KpiTileProps {
  label: string
  value: string | number | undefined
  subLabel?: string
}

export function KpiTile({ label, value, subLabel }: KpiTileProps) {
  return (
    <div className="bg-surface border border-line rounded-xl p-4 flex flex-col gap-1">
      <span className="text-xs text-ink/60 uppercase tracking-wider">{label}</span>
      {value !== undefined ? (
        <span className="text-2xl font-bold text-ink font-mono">{value}</span>
      ) : (
        <span data-testid="kpi-loading" className="text-2xl font-bold text-ink/30">—</span>
      )}
      {subLabel && (
        <span className="text-xs text-ink/40">{subLabel}</span>
      )}
    </div>
  )
}
