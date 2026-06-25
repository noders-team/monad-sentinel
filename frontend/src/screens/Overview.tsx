import { useState } from 'react'
import { useStatus, useUpgrades, useAlerts, useMetrics, useRestart } from '../api/hooks'
import type { Service } from '../api/types'
import { ConfirmOpModal } from '../components/ConfirmOpModal'
import { StatusDot } from '../components/StatusDot'
import { KpiTile } from '../components/KpiTile'
import { Sparkline } from '../components/Sparkline'
import { Toast } from '../components/Toast'

// Constant node name used for the ConfirmOpModal safety check.
// In a future iteration this could be derived from a runtime config endpoint.
export const NODE_NAME = 'monad-sentinel-node'

interface ModalState {
  service: Service
}

export function Overview() {
  const { data: services } = useStatus()
  const { data: upgrades } = useUpgrades()
  const { data: alerts } = useAlerts()
  const { data: metrics } = useMetrics('monad_vote_rate', '24h')
  const restart = useRestart()

  const [modal, setModal] = useState<ModalState | null>(null)
  const [toast, setToast] = useState<{ message: string; kind: 'success' | 'error' } | null>(null)

  const showUpgradeBanner =
    upgrades?.candidate != null && upgrades.candidate !== upgrades.current

  // Derived KPI values
  const activeCount = services?.filter(s => s.active).length
  const totalCount = services?.length
  const serviceStatusValue =
    activeCount !== undefined && totalCount !== undefined
      ? `${activeCount}/${totalCount}`
      : undefined

  function handleRestart(service: Service) {
    restart.reset()
    setModal({ service })
  }

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
    <div className="p-6 space-y-6 text-ink">
      {/* Toast notifications */}
      {toast && (
        <div className="fixed top-4 right-4 z-40 w-80">
          <Toast message={toast.message} kind={toast.kind} onDismiss={() => setToast(null)} />
        </div>
      )}

      {/* Upgrade banner */}
      {showUpgradeBanner && (
        <div
          data-testid="upgrade-banner"
          className="flex items-center gap-3 px-4 py-3 rounded-lg border border-sky-500 bg-sky-500/10 text-sky-300 text-sm"
        >
          <span className="font-semibold">Upgrade available:</span>
          <span>
            {upgrades!.current} → <strong>{upgrades!.candidate}</strong>
          </span>
        </div>
      )}

      {/* KPI tiles */}
      <section>
        <h2 className="text-sm font-semibold text-ink/60 uppercase tracking-wider mb-3">Overview</h2>
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
          <KpiTile label="Services" value={serviceStatusValue} subLabel="active / total" />
          <KpiTile label="Version" value={upgrades?.current ?? undefined} />
          <KpiTile label="Alerts" value={alerts?.length} />
          <KpiTile label="Candidate" value={upgrades?.candidate ?? undefined} />
        </div>
      </section>

      {/* Sparkline */}
      <section>
        <h2 className="text-sm font-semibold text-ink/60 uppercase tracking-wider mb-2">
          Vote Rate — 24h
        </h2>
        <div className="bg-surface border border-line rounded-xl p-4">
          <Sparkline data={metrics} height={80} />
        </div>
      </section>

      {/* Services panel */}
      <section>
        <h2 className="text-sm font-semibold text-ink/60 uppercase tracking-wider mb-3">Services</h2>
        <div className="bg-surface border border-line rounded-xl divide-y divide-line">
          {services === undefined ? (
            <div className="px-4 py-3 text-ink/40 text-sm">Loading…</div>
          ) : (
            services.map(svc => (
              <div key={svc.unit} className="flex items-center gap-3 px-4 py-3">
                <StatusDot active={svc.active} />
                <span className="flex-1 font-medium text-sm">{svc.name}</span>
                {svc.version && (
                  <span className="text-xs text-ink/40 font-mono">{svc.version}</span>
                )}
                <button
                  type="button"
                  aria-label={`restart ${svc.name}`}
                  onClick={() => handleRestart(svc)}
                  className="px-3 py-1 text-xs rounded border border-line text-ink/60 hover:text-ink hover:border-neon/40 transition-colors"
                >
                  Restart
                </button>
              </div>
            ))
          )}
        </div>
      </section>

      {/* Recent alerts */}
      <section>
        <h2 className="text-sm font-semibold text-ink/60 uppercase tracking-wider mb-3">Recent Alerts</h2>
        {!alerts || alerts.length === 0 ? (
          <p className="text-ink/40 text-sm">No alerts</p>
        ) : (
          <div className="bg-surface border border-line rounded-xl divide-y divide-line">
            {alerts.map((alert) => (
              <div key={alert.ts_ms} className="px-4 py-3 text-sm">
                <div className="flex items-center gap-2">
                  <span className="font-mono text-amber-400 text-xs">{alert.rule}</span>
                  <span className="text-ink/40 text-xs">
                    {new Date(alert.ts_ms).toLocaleString()}
                  </span>
                </div>
                <p className="text-ink/80 mt-0.5">{alert.message}</p>
              </div>
            ))}
          </div>
        )}
      </section>

      {/* Confirm restart modal */}
      {modal && (
        <ConfirmOpModal
          nodeName={NODE_NAME}
          title={`Restart ${modal.service.name}`}
          onConfirm={handleConfirm}
          pending={restart.isPending}
          error={restart.error instanceof Error ? restart.error.message : undefined}
        />
      )}
    </div>
  )
}
