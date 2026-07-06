import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useStatus, useUpgrades, useAlerts, useMetrics, useRestart } from '../api/hooks'
import type { Service, Alert } from '../api/types'
import { ConfirmOpModal } from '../components/ConfirmOpModal'
import { StatusDot } from '../components/StatusDot'
import { KpiTile } from '../components/KpiTile'
import { Sparkline } from '../components/Sparkline'
import { Toast } from '../components/Toast'
import { Card, CardHead, SectionLabel, Chip, Pill, ProgressBar, btn } from '../components/ui'
import { useVdpCompliance, useNowMinute, computeCountdown, formatDeadlineUtc } from '../api/vdp'
import { NODE_NAME } from '../constants'

// TODO: these KPI values come from node telemetry the frontend cannot read yet.
// Wire them to real /api/metrics series once the endpoints expose them; for now
// they mirror the design handoff so the strip is complete.
const KPIS = [
  { label: 'Participation', value: '99.1%', sub: '24h · voted rounds' },
  { label: 'Vote / Round', value: '0.982', sub: 'EWMA baseline 0.978' },
  { label: 'Block height', value: '48,412,096', sub: 'Finalized · 2.5 blk/s' },
  { label: 'Round', value: '51,203,441', sub: 'epoch 812' },
  { label: 'Peers', value: '146', sub: 'min 142 last 24h' },
  { label: 'TrieDB', value: '61%', sub: '1.22 / 2.0 TB NVMe' },
]

function ago(ms: number): string {
  const s = Math.max(0, (Date.now() - ms) / 1000)
  if (s < 60) return `${Math.floor(s)}s ago`
  if (s < 3600) return `${Math.floor(s / 60)}m ago`
  if (s < 86400) return `${Math.floor(s / 3600)}h ago`
  return `${Math.floor(s / 86400)}d ago`
}

function alertSeverity(rule: string): 'crit' | 'warn' {
  return /loss|stall|absent|down/i.test(rule) ? 'crit' : 'warn'
}

export function Overview() {
  const navigate = useNavigate()
  const { data: services } = useStatus()
  const { data: upgrades } = useUpgrades()
  const { data: alerts } = useAlerts()
  const { data: metrics } = useMetrics('monad_vote_rate', '24h')
  const restart = useRestart()
  const vdp = useVdpCompliance()
  const now = useNowMinute()

  const [modal, setModal] = useState<{ service: Service } | null>(null)
  const [toast, setToast] = useState<{ message: string; kind: 'success' | 'error' } | null>(null)

  const showUpgradeBanner = upgrades?.candidate != null && upgrades.candidate !== upgrades.current
  const countdown = computeCountdown(vdp.window.announcedIso, vdp.window.deadlineIso, now)

  const votePoints = metrics?.points ?? []
  const voteAvg =
    votePoints.length > 0
      ? (votePoints.reduce((a, [, v]) => a + v, 0) / votePoints.length).toFixed(3)
      : '—'

  function handleConfirm(totp: string) {
    if (!modal) return
    restart.mutate(
      { unit: modal.service.unit, totp },
      {
        onSuccess: () => {
          setModal(null)
          setToast({ message: `Restarted ${modal.service.name}`, kind: 'success' })
        },
      }
    )
  }

  return (
    <div className="px-5 pt-4 pb-8 space-y-3.5">
      {toast && (
        <div className="fixed top-4 right-4 z-40 w-80">
          <Toast message={toast.message} kind={toast.kind} onDismiss={() => setToast(null)} />
        </div>
      )}

      {/* Upgrade banner */}
      {showUpgradeBanner && (
        <div
          data-testid="upgrade-banner"
          className="flex flex-wrap items-center gap-x-3 gap-y-1 border border-acc rounded-[10px] px-[14px] py-[9px] text-[12.5px] bg-acc-soft"
        >
          <span className="font-semibold text-acc-ink">Upgrade available</span>
          <span className="font-mono text-ink">monad-bft {upgrades!.current} → {upgrades!.candidate}</span>
          <span className="text-mut">announced {ago(new Date(vdp.window.announcedIso).getTime())} · VDP window closes in {countdown.text}</span>
          <button type="button" onClick={() => navigate('/upgrades')} className={`ml-auto ${btn.primary}`}>
            Review upgrade
          </button>
        </div>
      )}

      {/* VDP compliance */}
      <section>
        <SectionLabel>VDP compliance · validator delegation program</SectionLabel>
        <div className="grid gap-2.5 md:grid-cols-3">
          {/* Uptime budget */}
          <Card className="p-4">
            <div className="flex items-center justify-between">
              <span className="eyebrow">Uptime budget · weekly</span>
              <Pill kind="ok">OK</Pill>
            </div>
            <div className="mt-2 flex items-baseline gap-2">
              <span className="font-mono text-[26px] font-semibold leading-none text-ink">{vdp.uptime.weeklyPct.toFixed(2)}%</span>
              <span className="text-[11px] text-mut">vs {vdp.uptime.thresholdPct.toFixed(2)}% threshold</span>
            </div>
            <div className="mt-3">
              <ProgressBar pct={vdp.uptime.budgetUsedPct} tone="ok" />
            </div>
            <div className="mt-2 flex justify-between text-[11px] text-mut">
              <span>{vdp.uptime.budgetUsedPct}% of downtime budget used</span>
              <span className="font-mono">flags {vdp.uptime.flags} / {vdp.uptime.flagLimit} · {vdp.uptime.flagWindowDays}d</span>
            </div>
          </Card>

          {/* 48h upgrade window */}
          <Card className="p-4 border-warn">
            <div className="flex items-center justify-between">
              <span className="eyebrow">48h upgrade window</span>
              <Pill kind="open">OPEN</Pill>
            </div>
            <div className="mt-2 flex items-baseline gap-2">
              <span className="font-mono text-[26px] font-semibold leading-none text-warn">{countdown.text}</span>
              <span className="text-[11px] text-mut">remaining</span>
            </div>
            <div className="mt-3">
              <ProgressBar pct={countdown.elapsed * 100} tone="warn" />
            </div>
            <div className="mt-2 flex justify-between text-[11px] text-mut">
              <span className="font-mono">{vdp.window.fromVersion} → {vdp.window.toVersion}</span>
              <span>deadline {formatDeadlineUtc(vdp.window.deadlineIso)}</span>
            </div>
          </Card>

          {/* 24h response SLA */}
          <Card className="p-4">
            <div className="flex items-center justify-between">
              <span className="eyebrow">24h response SLA</span>
              <Pill kind="ok">OK</Pill>
            </div>
            <div className="mt-2 flex items-baseline gap-2">
              <span className="font-mono text-[26px] font-semibold leading-none text-ink">{vdp.sla.openRequests}</span>
              <span className="text-[11px] text-mut">open foundation requests</span>
            </div>
            <div className="mt-[26px] flex justify-between text-[11px] text-mut">
              <span>last request answered in {vdp.sla.lastAnsweredIn}</span>
              <span>{vdp.sla.lastAnsweredDate}</span>
            </div>
          </Card>
        </div>
      </section>

      {/* KPI strip */}
      <div className="grid gap-2.5 grid-cols-2 md:grid-cols-3 xl:grid-cols-6">
        {KPIS.map(k => (
          <KpiTile key={k.label} label={k.label} value={k.value} subLabel={k.sub} />
        ))}
      </div>

      {/* Two-column: services + vote rate / recent alerts */}
      <div className="grid gap-2.5 lg:grid-cols-12">
        <div className="lg:col-span-7 space-y-2.5">
          <Card>
            <CardHead label="Services" />
            {services === undefined ? (
              <div className="px-[14px] py-2 text-mut text-[12.5px]">Loading…</div>
            ) : (
              services.map(svc => (
                <div key={svc.unit} className="flex items-center gap-3 px-[14px] py-2 border-b border-line last:border-0">
                  <StatusDot active={svc.active} size="sm" />
                  <span className="font-mono text-[12.5px] w-[150px] truncate">{svc.name}</span>
                  <span className="flex-1 text-[11.5px] text-mut truncate">{svc.unit}</span>
                  {svc.version && <span className="font-mono text-[11px] text-mut">{svc.version}</span>}
                  <button
                    type="button"
                    aria-label={`restart ${svc.name}`}
                    onClick={() => { restart.reset(); setModal({ service: svc }) }}
                    className="border border-line text-mut text-[11.5px] rounded px-2.5 py-1 hover:text-acc-ink hover:border-acc transition-colors"
                  >
                    Restart
                  </button>
                </div>
              ))
            )}
          </Card>

          <Card>
            <CardHead label="Vote rate · 24h" right={<span className="font-mono text-[11.5px] text-ok">{voteAvg} avg</span>} />
            <div className="px-[14px] py-3">
              <Sparkline data={metrics} height={64} />
            </div>
          </Card>
        </div>

        <div className="lg:col-span-5">
          <Card>
            <CardHead
              label="Recent alerts"
              right={
                <button type="button" onClick={() => navigate('/alerts')} className="text-[11px] text-acc-ink hover:underline">
                  View all →
                </button>
              }
            />
            {!alerts || alerts.length === 0 ? (
              <div className="px-[14px] py-3 text-mut text-[12.5px]">No alerts</div>
            ) : (
              alerts.slice(0, 4).map((alert: Alert) => (
                <div key={alert.ts_ms} className="px-[14px] py-2.5 border-b border-line last:border-0">
                  <div className="flex items-center gap-2">
                    <Chip severity={alertSeverity(alert.rule)}>{alertSeverity(alert.rule)}</Chip>
                    <span className="font-mono text-[11.5px] text-acc-ink">{alert.rule}</span>
                    <span className="ml-auto font-mono text-[10.5px] text-mut">{ago(alert.ts_ms)}</span>
                  </div>
                  <p className="text-[11.5px] text-mut mt-1 leading-[1.45]">{alert.message}</p>
                </div>
              ))
            )}
          </Card>
        </div>
      </div>

      {modal && (
        <ConfirmOpModal
          nodeName={NODE_NAME}
          title={`Restart ${modal.service.name}`}
          onConfirm={handleConfirm}
          onCancel={() => setModal(null)}
          pending={restart.isPending}
          error={restart.error instanceof Error ? restart.error.message : undefined}
        />
      )}
    </div>
  )
}
