interface KpiTileProps {
  label: string
  value: string | number | undefined
  subLabel?: string
}

export function KpiTile({ label, value, subLabel }: KpiTileProps) {
  return (
    <div className="bg-panel border border-line rounded-[10px] px-[13px] py-2.5 flex flex-col gap-1">
      <span className="eyebrow">{label}</span>
      {value !== undefined ? (
        <span className="text-[17px] font-semibold text-ink font-mono leading-none">{value}</span>
      ) : (
        <span data-testid="kpi-loading" className="text-[17px] font-semibold text-mut/40 font-mono leading-none">—</span>
      )}
      {subLabel && (
        <span className="text-[11px] text-mut">{subLabel}</span>
      )}
    </div>
  )
}
