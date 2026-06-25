import { screen } from '@testing-library/react'
import { Routes, Route } from 'react-router-dom'
import { http, HttpResponse } from 'msw'
import { server } from '../test/mswServer'
import { renderWithProviders } from '../test/renderWithProviders'
import AppShell from './AppShell'
import type { Service, Alert, Upgrades } from '../api/types'

const mockServices: Service[] = [
  { name: 'monad-bft', unit: 'monad-bft.service', kind: 'validator', active: true, version: '1.0.0' },
  { name: 'monad-rpc', unit: 'monad-rpc.service', kind: 'rpc', active: false, version: null },
]

const mockAlerts: Alert[] = [
  { ts_ms: Date.now(), rule: 'participation_loss', message: 'Low participation' },
]

const mockUpgrades: Upgrades = {
  current: '1.0.0',
  candidate: '1.1.0',
  target: '1.1.0',
  deadline: '2026-07-01',
  rollback_point: '1.0.0',
}

function setupHandlers() {
  server.use(
    http.get('/api/status', () => HttpResponse.json(mockServices)),
    http.get('/api/alerts', () => HttpResponse.json(mockAlerts)),
    http.get('/api/upgrades', () => HttpResponse.json(mockUpgrades)),
  )
}

test('renders all 6 tabs', () => {
  setupHandlers()
  renderWithProviders(
    <Routes>
      <Route element={<AppShell />}>
        <Route path="/" element={<div>Home</div>} />
      </Route>
    </Routes>
  )
  expect(screen.getByTestId('tab-overview')).toBeInTheDocument()
  expect(screen.getByTestId('tab-metrics')).toBeInTheDocument()
  expect(screen.getByTestId('tab-logs')).toBeInTheDocument()
  expect(screen.getByTestId('tab-alerts')).toBeInTheDocument()
  expect(screen.getByTestId('tab-upgrades')).toBeInTheDocument()
  expect(screen.getByTestId('tab-operations')).toBeInTheDocument()
})

test('renders status bar', () => {
  setupHandlers()
  renderWithProviders(
    <Routes>
      <Route element={<AppShell />}>
        <Route path="/" element={<div>Home</div>} />
      </Route>
    </Routes>
  )
  expect(screen.getByTestId('status-bar')).toBeInTheDocument()
  expect(screen.getByTestId('tab-bar')).toBeInTheDocument()
})
