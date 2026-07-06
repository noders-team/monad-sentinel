import type { ReactNode } from 'react'

/** White panel used for every card, table and section container. */
export function Card({ className = '', children }: { className?: string; children: ReactNode }) {
  return <div className={`bg-panel border border-line rounded-[10px] ${className}`}>{children}</div>
}

/** Card header row: uppercase label left, optional node (link/value) right. */
export function CardHead({ label, right }: { label: string; right?: ReactNode }) {
  return (
    <div className="flex items-center justify-between px-3.5 py-2.5 border-b border-line">
      <span className="eyebrow">{label}</span>
      {right}
    </div>
  )
}

/** Standalone uppercase section label. */
export function SectionLabel({ children }: { children: ReactNode }) {
  return <div className="eyebrow mb-2">{children}</div>
}

type Severity = 'crit' | 'warn'
const SEV: Record<Severity, string> = {
  crit: 'text-dgr border-dgr',
  warn: 'text-warn border-warn',
}

/** Severity chip (alerts). */
export function Chip({ severity, children }: { severity: Severity; children: ReactNode }) {
  return (
    <span
      className={`inline-block text-[10px] font-bold uppercase tracking-[0.06em] rounded-[3px] px-1.5 py-px border ${SEV[severity]}`}
    >
      {children}
    </span>
  )
}

type PillKind = 'active' | 'resolved' | 'ok' | 'open'
const PILL: Record<PillKind, string> = {
  active: 'text-dgr border-dgr',
  resolved: 'text-mut border-line',
  ok: 'text-ok border-ok',
  open: 'text-warn border-warn',
}

/** State pill (alert status, VDP card status). */
export function Pill({ kind, children }: { kind: PillKind; children: ReactNode }) {
  return (
    <span className={`inline-block text-[10.5px] font-semibold rounded-full px-2 py-px border ${PILL[kind]}`}>
      {children}
    </span>
  )
}

/** Thin progress bar; `tone` picks the fill color, `pct` is 0–100. */
export function ProgressBar({ pct, tone = 'ok' }: { pct: number; tone?: 'ok' | 'warn' | 'acc' }) {
  const fill = tone === 'warn' ? 'bg-warn' : tone === 'acc' ? 'bg-acc' : 'bg-ok'
  return (
    <div className="h-[5px] rounded-full bg-panel2 overflow-hidden">
      <div className={`h-full rounded-full ${fill}`} style={{ width: `${Math.min(Math.max(pct, 0), 100)}%` }} />
    </div>
  )
}

/** Primary / secondary / warn-outline button styles as class strings. */
export const btn = {
  primary:
    'inline-flex items-center gap-1.5 bg-acc text-[#FBFAF9] font-semibold text-[12.5px] rounded px-4 py-2 hover:brightness-110 transition',
  outline:
    'inline-flex items-center gap-1.5 border border-line text-mut text-[12.5px] rounded px-3 py-1.5 hover:text-acc-ink hover:border-acc transition-colors',
  warn:
    'inline-flex items-center gap-1.5 border border-warn text-warn text-[12.5px] rounded px-3 py-1.5 transition-colors hover:bg-[rgba(200,130,26,0.1)]',
}
