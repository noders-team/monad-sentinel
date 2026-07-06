import { useEffect, useState } from 'react'

/**
 * VDP (Validator Delegation Program) compliance data for the Overview widgets.
 *
 * TODO: the backend does not expose these yet. Weekly uptime %, flag count, the
 * upgrade announce timestamp, and the response-SLA summary are hardcoded here so
 * the UI is complete. Replace `useVdpCompliance` with a real `/api/vdp` query
 * once the endpoint lands — the component contract (shape below) should not change.
 */
export interface VdpCompliance {
  uptime: {
    weeklyPct: number
    thresholdPct: number
    budgetUsedPct: number
    flags: number
    flagLimit: number
    flagWindowDays: number
  }
  window: {
    fromVersion: string
    toVersion: string
    /** Announce time — start of the 48h VDP upgrade window. */
    announcedIso: string
    /** Hard deadline; the countdown and elapsed bar derive from this. */
    deadlineIso: string
  }
  sla: {
    openRequests: number
    lastAnsweredIn: string
    lastAnsweredDate: string
  }
}

const HARDCODED: VdpCompliance = {
  uptime: {
    weeklyPct: 99.42,
    thresholdPct: 98.0,
    budgetUsedPct: 29,
    flags: 0,
    flagLimit: 3,
    flagWindowDays: 90,
  },
  window: {
    fromVersion: '0.14.6',
    toVersion: '0.14.7',
    announcedIso: '2026-07-06T02:11:00Z',
    deadlineIso: '2026-07-08T02:11:00Z',
  },
  sla: {
    openRequests: 0,
    lastAnsweredIn: '3h 12m',
    lastAnsweredDate: 'Jun 29',
  },
}

export function useVdpCompliance(): VdpCompliance {
  return HARDCODED
}

export interface Countdown {
  /** "33h 46m" style remaining time, or "expired". */
  text: string
  /** Share of the window elapsed, 0–1, for the progress bar. */
  elapsed: number
  expired: boolean
}

/** Format the remaining time and elapsed fraction of a [start, deadline] window. */
export function computeCountdown(startIso: string, deadlineIso: string, nowMs: number): Countdown {
  const start = new Date(startIso).getTime()
  const end = new Date(deadlineIso).getTime()
  const remainingMs = end - nowMs
  if (remainingMs <= 0) return { text: 'expired', elapsed: 1, expired: true }

  const totalMs = Math.max(end - start, 1)
  const elapsed = Math.min(Math.max((nowMs - start) / totalMs, 0), 1)

  const totalMinutes = Math.floor(remainingMs / 60_000)
  const hours = Math.floor(totalMinutes / 60)
  const minutes = totalMinutes % 60
  return { text: `${hours}h ${minutes}m`, elapsed, expired: false }
}

/** Re-renders once a minute so time-based UI (the 48h countdown) stays fresh. */
export function useNowMinute(): number {
  const [now, setNow] = useState(() => Date.now())
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), 60_000)
    return () => clearInterval(id)
  }, [])
  return now
}

/** Format an ISO deadline as e.g. "Jul 8, 02:11 UTC". */
export function formatDeadlineUtc(deadlineIso: string): string {
  const d = new Date(deadlineIso)
  const month = d.toLocaleString('en-US', { month: 'short', timeZone: 'UTC' })
  const day = d.getUTCDate()
  const hh = String(d.getUTCHours()).padStart(2, '0')
  const mm = String(d.getUTCMinutes()).padStart(2, '0')
  return `${month} ${day}, ${hh}:${mm} UTC`
}
