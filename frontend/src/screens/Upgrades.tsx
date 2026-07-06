import { useState, type FormEvent } from 'react'
import { useUpgrades, useUpgrade, useRollback, useSetPlan } from '../api/hooks'
import { ConfirmOpModal } from '../components/ConfirmOpModal'
import { Toast } from '../components/Toast'
import { StaleBanner } from '../components/StaleBanner'
import { Card, CardHead, ProgressBar, btn } from '../components/ui'
import { useVdpCompliance, useNowMinute, computeCountdown, formatDeadlineUtc } from '../api/vdp'
import { NODE_NAME } from '../constants'

type ModalOp = 'upgrade' | 'rollback'

const STAGES = ['preflight', 'drain', 'swap binary', 'restart units', 'verify']

export function Upgrades() {
  const { data: upgrades, isError, isLoading } = useUpgrades()
  const upgrade = useUpgrade()
  const rollback = useRollback()
  const setPlan = useSetPlan()
  const vdp = useVdpCompliance()
  const now = useNowMinute()

  const [modal, setModal] = useState<{ op: ModalOp } | null>(null)
  const [toast, setToast] = useState<{ message: string; kind: 'success' | 'error' } | null>(null)
  const [manualTarget, setManualTarget] = useState('')
  const [manualDeadline, setManualDeadline] = useState('')

  const hasData = upgrades !== undefined
  const runTarget = manualTarget || upgrades?.candidate || upgrades?.target || ''
  const countdown = computeCountdown(vdp.window.announcedIso, vdp.window.deadlineIso, now)

  function handleConfirm(totp: string) {
    if (!modal) return
    if (modal.op === 'upgrade') {
      upgrade.mutate(
        { target_version: runTarget, totp },
        { onSuccess: () => { setModal(null); setToast({ message: `Upgrade to ${runTarget} initiated`, kind: 'success' }) } }
      )
    } else {
      rollback.mutate(
        { totp },
        { onSuccess: () => { setModal(null); setToast({ message: 'Rollback initiated', kind: 'success' }) } }
      )
    }
  }

  function handleSetPlan(e: FormEvent) {
    e.preventDefault()
    setPlan.mutate({ target_version: manualTarget, deadline: manualDeadline })
  }

  const activeMutation = modal?.op === 'upgrade' ? upgrade : rollback

  if (isLoading) return <div className="px-5 pt-4 text-mut text-[12.5px]">Loading…</div>
  if (isError && !hasData) return <div className="px-5 pt-4 text-dgr text-[12.5px]">Failed to load upgrade info</div>

  const inputClass = 'w-full bg-bg border border-line rounded px-[10px] py-[7px] font-mono text-[12.5px] text-ink outline-none focus:border-acc'

  return (
    <div className="px-5 pt-4 pb-8 space-y-3.5">
      {toast && (
        <div className="fixed top-4 right-4 z-40 w-80">
          <Toast message={toast.message} kind={toast.kind} onDismiss={() => setToast(null)} />
        </div>
      )}
      <StaleBanner isError={isError} hasData={hasData} />

      <h1 className="text-[15px] font-semibold">Upgrades</h1>

      <div className="grid gap-2.5 lg:grid-cols-12">
        {/* Left column */}
        <div className="lg:col-span-7 space-y-2.5">
          {/* Version compare */}
          <Card className="p-4">
            <div className="grid grid-cols-[1fr_auto_1fr] items-center gap-3">
              <div className="bg-panel2 border border-line rounded-[8px] px-3.5 py-3">
                <div className="eyebrow">Current</div>
                <div className="font-mono text-[18px] font-semibold text-ink mt-1">{upgrades?.current ?? '—'}</div>
              </div>
              <span className="text-acc text-[18px]" aria-hidden="true">→</span>
              <div className="bg-acc-soft border border-acc rounded-[8px] px-3.5 py-3">
                <div className="eyebrow text-acc-ink">Candidate</div>
                <div className="font-mono text-[18px] font-semibold text-ink mt-1">{upgrades?.candidate ?? '—'}</div>
              </div>
            </div>

            <div className="mt-4">
              <div className="flex justify-between text-[11px] mb-1.5">
                <span className="text-mut">VDP 48h window</span>
                <span className="font-mono text-warn">{countdown.text} · deadline {formatDeadlineUtc(vdp.window.deadlineIso)}</span>
              </div>
              <ProgressBar pct={countdown.elapsed * 100} tone="warn" />
            </div>

            <div className="mt-4 flex flex-wrap gap-2.5">
              <button
                type="button"
                onClick={() => { upgrade.reset(); setModal({ op: 'upgrade' }) }}
                disabled={!runTarget}
                className={`${btn.primary} disabled:bg-panel2 disabled:text-mut disabled:cursor-not-allowed disabled:hover:brightness-100`}
              >
                Run upgrade → {runTarget || '—'}
              </button>
              {upgrades?.rollback_point != null && (
                <button type="button" onClick={() => { rollback.reset(); setModal({ op: 'rollback' }) }} className={btn.warn}>
                  Rollback to {upgrades.rollback_point}
                </button>
              )}
            </div>
          </Card>

          {/* Staged pipeline */}
          <Card className="p-4">
            <div className="eyebrow mb-2.5">Staged pipeline</div>
            <div className="flex flex-wrap items-center gap-1.5">
              {STAGES.map((stage, i) => (
                <span key={stage} className="flex items-center gap-1.5">
                  <span className="font-mono text-[11.5px] border border-line text-mut rounded px-2 py-0.5">{stage}</span>
                  {i < STAGES.length - 1 && <span className="text-line">──</span>}
                </span>
              ))}
            </div>
            <p className="text-[11px] text-mut mt-2.5">
              preflight → drain → swap binary → restart units in order → verify block growth &amp; commitState progression for 10m → auto-rollback on failure.
            </p>
          </Card>
        </div>

        {/* Right column */}
        <div className="lg:col-span-5 space-y-2.5">
          <Card>
            <CardHead label="Upgrade plan" />
            <form onSubmit={handleSetPlan} className="p-3.5 space-y-3">
              <div>
                <label htmlFor="plan-target" className="block eyebrow mb-1.5">Target version</label>
                <input id="plan-target" type="text" value={manualTarget} onChange={e => setManualTarget(e.target.value)} placeholder="0.14.8" className={inputClass} />
              </div>
              <div>
                <label htmlFor="plan-deadline" className="block eyebrow mb-1.5">Deadline UTC</label>
                <input id="plan-deadline" type="text" value={manualDeadline} onChange={e => setManualDeadline(e.target.value)} placeholder="2026-07-08T02:11:00Z" className={inputClass} />
              </div>
              <button
                type="submit"
                disabled={setPlan.isPending || !manualTarget}
                className="border border-acc text-acc-ink text-[12.5px] font-semibold rounded px-3 py-1.5 hover:bg-acc-soft transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
              >
                {setPlan.isPending ? 'Saving…' : 'Save plan'}
              </button>
            </form>
          </Card>

          <Card>
            <CardHead label="Rollback point" />
            <div className="p-3.5">
              {upgrades?.rollback_point != null ? (
                <>
                  <div className="font-mono text-[12.5px] text-ink">monad-bft <span>{upgrades.rollback_point}</span></div>
                  <p className="text-[11px] text-mut mt-1">Restores the version recorded immediately before the last upgrade.</p>
                </>
              ) : (
                <div className="text-mut text-[12.5px]">No rollback point recorded yet.</div>
              )}
            </div>
          </Card>
        </div>
      </div>

      {modal && (
        <ConfirmOpModal
          nodeName={NODE_NAME}
          title={modal.op === 'upgrade' ? `Run upgrade → ${runTarget}` : `Rollback to ${upgrades?.rollback_point ?? ''}`}
          onConfirm={handleConfirm}
          onCancel={() => setModal(null)}
          pending={activeMutation.isPending}
          error={activeMutation.error instanceof Error ? activeMutation.error.message : undefined}
        />
      )}
    </div>
  )
}
