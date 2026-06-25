import { useState, type FormEvent } from 'react'
import { useUpgrades, useUpgrade, useRollback, useSetPlan } from '../api/hooks'
import { ConfirmOpModal } from '../components/ConfirmOpModal'
import { Toast } from '../components/Toast'
import { StaleBanner } from '../components/StaleBanner'
import { NODE_NAME } from './Overview'

type ModalOp = 'upgrade' | 'rollback'

interface ModalState {
  op: ModalOp
}

export function Upgrades() {
  const { data: upgrades, isError, isLoading } = useUpgrades()
  const upgrade = useUpgrade()
  const rollback = useRollback()
  const setPlan = useSetPlan()

  const [modal, setModal] = useState<ModalState | null>(null)
  const [toast, setToast] = useState<{ message: string; kind: 'success' | 'error' } | null>(null)

  const [manualTarget, setManualTarget] = useState('')
  const [manualDeadline, setManualDeadline] = useState('')

  const hasData = upgrades !== undefined

  function handleRunUpgrade() {
    upgrade.reset()
    setModal({ op: 'upgrade' })
  }

  function handleRollback() {
    rollback.reset()
    setModal({ op: 'rollback' })
  }

  function handleConfirm(totp: string) {
    if (!modal) return

    if (modal.op === 'upgrade') {
      const target_version = manualTarget || upgrades?.candidate || ''
      upgrade.mutate(
        { target_version, totp },
        {
          onSuccess: () => {
            setModal(null)
            setToast({ message: 'Upgrade initiated successfully', kind: 'success' })
          },
        }
      )
    } else {
      rollback.mutate(
        { totp },
        {
          onSuccess: () => {
            setModal(null)
            setToast({ message: 'Rollback initiated successfully', kind: 'success' })
          },
        }
      )
    }
  }

  function handleSetPlan(e: FormEvent) {
    e.preventDefault()
    setPlan.mutate({ target_version: manualTarget, deadline: manualDeadline })
  }

  const activeMutation = modal?.op === 'upgrade' ? upgrade : rollback
  const modalTitle = modal?.op === 'upgrade' ? 'Run upgrade' : 'Rollback'

  if (isLoading) {
    return <div className="p-6 text-ink/60">Loading…</div>
  }

  if (isError && !hasData) {
    return <div className="p-6 text-red-400">Failed to load upgrade info</div>
  }

  return (
    <div className="p-6 space-y-6 text-ink">
      {/* Toast notifications */}
      {toast && (
        <div className="fixed top-4 right-4 z-40 w-80">
          <Toast message={toast.message} kind={toast.kind} onDismiss={() => setToast(null)} />
        </div>
      )}

      {/* Stale banner */}
      <StaleBanner isError={isError} hasData={hasData} />

      <h2 className="text-xl font-semibold text-ink">Upgrades</h2>

      {/* Version info */}
      <section className="bg-surface border border-line rounded-xl p-4 space-y-2">
        <div className="flex items-center gap-3 text-sm">
          <span className="text-ink/60 w-32">Current version</span>
          <span className="font-mono text-ink">{upgrades?.current ?? '—'}</span>
        </div>
        <div className="flex items-center gap-3 text-sm">
          <span className="text-ink/60 w-32">Candidate</span>
          <span className="font-mono text-ink">{upgrades?.candidate ?? '—'}</span>
        </div>
        {upgrades?.target && (
          <div className="flex items-center gap-3 text-sm">
            <span className="text-ink/60 w-32">Planned target</span>
            <span className="font-mono text-ink">{upgrades.target}</span>
          </div>
        )}
        {upgrades?.deadline && (
          <div className="flex items-center gap-3 text-sm">
            <span className="text-ink/60 w-32">Deadline</span>
            <span className="font-mono text-ink">{upgrades.deadline}</span>
          </div>
        )}
        {upgrades?.rollback_point != null && (
          <div className="flex items-center gap-3 text-sm">
            <span className="text-ink/60 w-32">Rollback point</span>
            <span className="font-mono text-ink">{upgrades.rollback_point}</span>
          </div>
        )}
      </section>

      {/* Actions */}
      <section className="flex gap-3">
        <button
          type="button"
          onClick={handleRunUpgrade}
          className="px-4 py-2 rounded bg-neon/10 text-neon border border-neon/30 text-sm font-medium hover:bg-neon/20 transition-colors"
        >
          Run upgrade
        </button>
        {upgrades?.rollback_point != null && (
          <button
            type="button"
            onClick={handleRollback}
            className="px-4 py-2 rounded bg-amber-500/10 text-amber-300 border border-amber-500/30 text-sm font-medium hover:bg-amber-500/20 transition-colors"
          >
            Rollback
          </button>
        )}
      </section>

      {/* Manual plan form */}
      <section className="bg-surface border border-line rounded-xl p-4">
        <h3 className="text-sm font-semibold text-ink/60 uppercase tracking-wider mb-3">
          Set Upgrade Plan
        </h3>
        <form onSubmit={handleSetPlan} className="space-y-3">
          <div>
            <label htmlFor="plan-target" className="block text-sm text-ink/60 mb-1">
              Target version
            </label>
            <input
              id="plan-target"
              type="text"
              value={manualTarget}
              onChange={e => setManualTarget(e.target.value)}
              placeholder="e.g. 0.14.8"
              className="w-full bg-void border border-line rounded px-3 py-2 text-ink text-sm outline-none focus:border-neon font-mono"
            />
          </div>
          <div>
            <label htmlFor="plan-deadline" className="block text-sm text-ink/60 mb-1">
              Deadline
            </label>
            <input
              id="plan-deadline"
              type="text"
              value={manualDeadline}
              onChange={e => setManualDeadline(e.target.value)}
              placeholder="e.g. 2024-12-31T00:00:00Z"
              className="w-full bg-void border border-line rounded px-3 py-2 text-ink text-sm outline-none focus:border-neon font-mono"
            />
          </div>
          <button
            type="submit"
            disabled={setPlan.isPending || !manualTarget}
            className="px-4 py-2 rounded bg-neon/10 text-neon border border-neon/30 text-sm font-medium
              disabled:opacity-40 disabled:cursor-not-allowed hover:bg-neon/20 transition-colors"
          >
            {setPlan.isPending ? 'Saving…' : 'Save plan'}
          </button>
        </form>
      </section>

      {/* Confirm operation modal */}
      {modal && (
        <ConfirmOpModal
          nodeName={NODE_NAME}
          title={modalTitle}
          onConfirm={handleConfirm}
          pending={activeMutation.isPending}
          error={activeMutation.error instanceof Error ? activeMutation.error.message : undefined}
        />
      )}
    </div>
  )
}
