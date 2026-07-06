import { NavLink, Outlet, useNavigate } from 'react-router-dom'
import { useStatus, useUpgrades, useAlerts, useLogout } from '../api/hooks'
import { StatusDot } from './StatusDot'
import { NODE_NAME } from '../constants'

const tabs = [
  { to: '/dashboard', label: 'Overview' },
  { to: '/metrics', label: 'Metrics' },
  { to: '/logs', label: 'Logs' },
  { to: '/alerts', label: 'Alerts' },
  { to: '/upgrades', label: 'Upgrades' },
  { to: '/operations', label: 'Operations' },
]

function Logo() {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" aria-hidden="true">
      <rect x="4.2" y="4.2" width="15.6" height="15.6" rx="5" transform="rotate(45 12 12)" fill="#836EF9" />
      <rect x="8.4" y="8.4" width="7.2" height="7.2" rx="2.4" transform="rotate(45 12 12)" fill="#FBFAF9" />
    </svg>
  )
}

export default function AppShell() {
  const navigate = useNavigate()
  const { data: services } = useStatus()
  const { data: upgrades } = useUpgrades()
  const { data: alerts } = useAlerts()
  const logout = useLogout()

  const activeCount = services?.filter(s => s.active).length ?? 0
  const totalCount = services?.length ?? 0
  const allUp = totalCount > 0 && activeCount === totalCount
  const alertCount = alerts?.length ?? 0
  const upgradeTarget = upgrades?.candidate && upgrades.candidate !== upgrades.current ? upgrades.candidate : null

  function signOut() {
    logout.mutate(undefined, { onSettled: () => navigate('/login') })
  }

  return (
    <div className="min-h-screen flex flex-col bg-bg text-ink">
      {/* Row 1 — status bar */}
      <header
        data-testid="status-bar"
        className="flex flex-wrap items-center gap-3.5 px-5 py-[9px] bg-panel border-b border-line"
      >
        <div className="flex items-center gap-2">
          <Logo />
          <span className="text-[13px] font-bold tracking-[0.14em]">SENTINEL</span>
        </div>

        <div className="flex items-center gap-2 bg-panel2 border border-line rounded-full pl-2 pr-2.5 py-1">
          <StatusDot active size="sm" pulse />
          <span className="font-mono text-[11.5px] text-ink">{NODE_NAME}</span>
          <span className="text-[11px] text-mut">mainnet</span>
        </div>

        <span data-testid="service-status" className={`font-mono text-[11.5px] ${allUp ? 'text-ok' : 'text-warn'}`}>
          services {activeCount}/{totalCount}
        </span>

        {alertCount > 0 && (
          <button
            type="button"
            data-testid="alert-count"
            onClick={() => navigate('/alerts')}
            className="font-mono text-[11.5px] text-warn hover:text-acc-ink transition-colors"
          >
            ▲ {alertCount} alert{alertCount !== 1 ? 's' : ''}
          </button>
        )}

        {upgradeTarget && (
          <span data-testid="upgrade-target" className="font-mono text-[11.5px] text-acc-ink">
            upgrade {upgradeTarget}
          </span>
        )}

        <div className="ml-auto flex items-center gap-2">
          <span className="font-mono text-[11.5px] text-mut">admin</span>
          <span className="text-mut">·</span>
          <button
            type="button"
            onClick={signOut}
            className="text-[12px] text-mut hover:text-acc-ink border border-line hover:border-acc rounded px-2.5 py-1 transition-colors"
          >
            sign out
          </button>
        </div>
      </header>

      {/* Row 2 — tabs */}
      <nav data-testid="tab-bar" className="flex flex-wrap gap-1 px-5 bg-panel border-b border-line">
        {tabs.map(tab => (
          <NavLink
            key={tab.to}
            to={tab.to}
            data-testid={`tab-${tab.label.toLowerCase()}`}
            className={({ isActive }) =>
              `text-[12.5px] px-3.5 py-2.5 border-b-2 -mb-px transition-colors ${
                isActive
                  ? 'text-acc-ink font-semibold border-acc'
                  : 'text-mut border-transparent hover:text-acc-ink'
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
