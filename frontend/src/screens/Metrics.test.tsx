globalThis.ResizeObserver = class { observe() {}; unobserve() {}; disconnect() {} }

import { http, HttpResponse } from 'msw'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { server } from '../test/mswServer'
import { renderWithProviders } from '../test/renderWithProviders'
import { Metrics } from './Metrics'

const services = [
  { name: 'monad-bft', unit: 'monad-bft.service', kind: 'systemd', active: true, version: '1.0.0' },
]

const makePoints = (): [number, number][] => [
  [1700000000000, 42],
  [1700003600000, 55],
]

let capturedMetricRequests: { name: string; window: string }[] = []

beforeEach(() => {
  capturedMetricRequests = []
  server.use(
    http.get('/api/status', () => HttpResponse.json(services)),
    http.get('/api/metrics', ({ request }) => {
      const url = new URL(request.url)
      capturedMetricRequests.push({
        name: url.searchParams.get('name') ?? '',
        window: url.searchParams.get('window') ?? '',
      })
      return HttpResponse.json({ points: makePoints() })
    }),
  )
})

test('renders WindowPicker with 1h, 24h, 7d buttons', async () => {
  renderWithProviders(<Metrics />)
  await waitFor(() => {
    expect(screen.getByRole('button', { name: '1h' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: '24h' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: '7d' })).toBeInTheDocument()
  })
})

test('24h is the default active window', async () => {
  renderWithProviders(<Metrics />)
  await waitFor(() => {
    const btn24h = screen.getByRole('button', { name: '24h' })
    expect(btn24h).toHaveAttribute('aria-pressed', 'true')
  })
})

test('changing window from 24h to 7d updates the metrics query param', async () => {
  renderWithProviders(<Metrics />)

  // Wait for initial load with 24h window
  await waitFor(() => {
    const requests24h = capturedMetricRequests.filter(r => r.window === '24h')
    expect(requests24h.length).toBeGreaterThan(0)
  })

  capturedMetricRequests = []

  // Click 7d button
  const btn7d = await screen.findByRole('button', { name: '7d' })
  await userEvent.click(btn7d)

  // Assert the next requests use window=7d
  await waitFor(() => {
    const requests7d = capturedMetricRequests.filter(r => r.window === '7d')
    expect(requests7d.length).toBeGreaterThan(0)
  })
})

test('shows metric chart titles', async () => {
  renderWithProviders(<Metrics />)
  await waitFor(() => {
    expect(screen.getByText(/Total Uptime/i)).toBeInTheDocument()
  })
})
