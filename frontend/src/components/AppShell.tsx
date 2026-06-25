import { NavLink, Outlet } from 'react-router-dom'
import { useStatus, useUpgrades, useAlerts } from '../api/hooks'

const tabs = [
  { to: '/dashboard', label: 'Overview' },
  { to: '/metrics', label: 'Metrics' },
  { to: '/logs', label: 'Logs' },
  { to: '/alerts', label: 'Alerts' },
  { to: '/upgrades', label: 'Upgrades' },
  { to: '/operations', label: 'Operations' },
]

export default function AppShell() {
  const { data: services } = useStatus()
  const { data: upgrades } = useUpgrades()
  const { data: alerts } = useAlerts()

  const activeCount = services?.filter(s => s.active).length ?? 0
  const totalCount = services?.length ?? 0
  const alertCount = alerts?.length ?? 0
  const upgradeTarget = upgrades?.target ?? null

  return (
    <div className="min-h-screen flex flex-col bg-void text-ink">
      <header data-testid="status-bar" className="flex items-center gap-4 px-6 py-2 bg-surface border-b border-line text-sm">
        <span className="font-semibold text-neon">Sentinel</span>
        <span data-testid="service-status">
          Services: {activeCount}/{totalCount} active
        </span>
        {alertCount > 0 && (
          <span data-testid="alert-count" className="text-amber-400">
            {alertCount} alert{alertCount !== 1 ? 's' : ''}
          </span>
        )}
        {upgradeTarget && (
          <span data-testid="upgrade-target" className="text-sky-400">
            Upgrade: {upgradeTarget}
          </span>
        )}
      </header>

      <nav data-testid="tab-bar" className="flex gap-1 px-6 py-1 bg-surface border-b border-line">
        {tabs.map(tab => (
          <NavLink
            key={tab.to}
            to={tab.to}
            data-testid={`tab-${tab.label.toLowerCase()}`}
            className={({ isActive }) =>
              `px-4 py-2 rounded text-sm font-medium transition-colors ${
                isActive
                  ? 'bg-neon/10 text-neon'
                  : 'text-ink/60 hover:text-ink hover:bg-surface-alt'
              }`
            }
          >
            {tab.label}
          </NavLink>
        ))}
      </nav>

      <main className="flex-1 overflow-auto">
        <Outlet />
      </main>
    </div>
  )
}
