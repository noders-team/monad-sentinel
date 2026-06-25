import { http, HttpResponse, delay } from 'msw'
import { screen, waitFor } from '@testing-library/react'
import { server } from '../test/mswServer'
import { renderWithProviders } from '../test/renderWithProviders'
import { Alerts } from './Alerts'

// older alert (lower ts_ms) — returned first by MSW to simulate unsorted API
const alertOld = { ts_ms: 1700000000000, rule: 'participation_loss', message: 'Vote rate dropped' }
// newer alert (higher ts_ms)
const alertNew = { ts_ms: 1700001000000, rule: 'block_lag', message: 'Block production delayed' }

beforeEach(() => {
  server.use(
    http.get('/api/auth/me', () => HttpResponse.json({ actor: 'admin' })),
  )
})

test('shows "No active alerts" when alerts array is empty', async () => {
  server.use(
    http.get('/api/alerts', () => HttpResponse.json([])),
  )
  renderWithProviders(<Alerts />)
  await waitFor(() => {
    expect(screen.getByText('No active alerts')).toBeInTheDocument()
  })
})

test('shows rule and message when alerts are present', async () => {
  server.use(
    http.get('/api/alerts', () =>
      HttpResponse.json([alertOld])
    ),
  )
  renderWithProviders(<Alerts />)
  await waitFor(() => {
    expect(screen.getByText('participation_loss')).toBeInTheDocument()
    expect(screen.getByText('Vote rate dropped')).toBeInTheDocument()
  })
  // relative-time column renders (e.g. "Xs ago", "Xm ago", "Xh ago", "Xd ago")
  expect(screen.getByText(/\d+\s*(s|m|h|d) ago/)).toBeInTheDocument()
})

test('renders alerts newest-first when API returns oldest-first', async () => {
  // API returns [alertOld, alertNew] — oldest first (unsorted)
  server.use(
    http.get('/api/alerts', () => HttpResponse.json([alertOld, alertNew])),
  )
  renderWithProviders(<Alerts />)
  await waitFor(() => {
    expect(screen.getByText('block_lag')).toBeInTheDocument()
    expect(screen.getByText('participation_loss')).toBeInTheDocument()
  })

  // alertNew (block_lag) must appear before alertOld (participation_loss) in DOM
  const cells = screen.getAllByText(/block_lag|participation_loss/)
  const newIndex = cells.findIndex(el => el.textContent === 'block_lag')
  const oldIndex = cells.findIndex(el => el.textContent === 'participation_loss')
  expect(newIndex).toBeLessThan(oldIndex)
})

test('shows loading state before data resolves', async () => {
  server.use(
    http.get('/api/alerts', async () => {
      await delay(200)
      return HttpResponse.json([alertOld])
    }),
  )
  renderWithProviders(<Alerts />)
  // Loading text appears synchronously (before fetch resolves)
  expect(screen.getByText('Loading…')).toBeInTheDocument()
  // Data eventually loads
  await waitFor(() => {
    expect(screen.getByText('participation_loss')).toBeInTheDocument()
  })
})

test('shows error state when API returns 500', async () => {
  server.use(
    http.get('/api/alerts', () => HttpResponse.json({ error: 'server error' }, { status: 500 })),
  )
  renderWithProviders(<Alerts />)
  await waitFor(() => {
    expect(screen.getByText('Failed to load alerts')).toBeInTheDocument()
  })
})
