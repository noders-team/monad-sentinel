import { http, HttpResponse } from 'msw'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { server } from '../test/mswServer'
import { renderWithProviders } from '../test/renderWithProviders'
import { Overview, NODE_NAME } from './Overview'

const services = [
  { name: 'monad-bft', unit: 'monad-bft.service', kind: 'systemd', active: true, version: '1.0.0' },
  { name: 'monad-exec', unit: 'monad-exec.service', kind: 'systemd', active: true, version: '1.0.0' },
  { name: 'monad-rpc', unit: 'monad-rpc.service', kind: 'systemd', active: false, version: null },
]

let capturedRestartBody: unknown = null

beforeEach(() => {
  capturedRestartBody = null
  server.use(
    http.get('/api/auth/me', () => HttpResponse.json({ actor: 'admin' })),
    http.get('/api/status', () => HttpResponse.json(services)),
    http.get('/api/upgrades', () =>
      HttpResponse.json({ current: '1.0.0', candidate: '1.1.0', target: null, deadline: null, rollback_point: null })
    ),
    http.get('/api/alerts', () =>
      HttpResponse.json([{ ts_ms: 1700000000000, rule: 'participation_loss', message: 'Vote rate dropped' }])
    ),
    http.get('/api/metrics', () => HttpResponse.json({ points: [[1700000000, 0.95], [1700003600, 0.98]] })),
    http.post('/api/ops/restart', async ({ request }) => {
      capturedRestartBody = await request.json()
      return HttpResponse.json({ ok: true, detail: 'restarted' })
    }),
  )
})

test('renders 3 services', async () => {
  renderWithProviders(<Overview />)
  await waitFor(() => {
    expect(screen.getByText('monad-bft')).toBeInTheDocument()
    expect(screen.getByText('monad-exec')).toBeInTheDocument()
    expect(screen.getByText('monad-rpc')).toBeInTheDocument()
  })
})

test('shows upgrade banner with candidate version', async () => {
  renderWithProviders(<Overview />)
  await waitFor(() => {
    expect(screen.getByTestId('upgrade-banner')).toBeInTheDocument()
    expect(screen.getByTestId('upgrade-banner')).toHaveTextContent('1.1.0')
  })
})

test('shows alert message', async () => {
  renderWithProviders(<Overview />)
  await waitFor(() => {
    expect(screen.getByText(/Vote rate dropped/i)).toBeInTheDocument()
  })
})

test('restart flow: opens modal, fills form, submits to API', async () => {
  renderWithProviders(<Overview />)

  // Wait for services to load
  await waitFor(() => expect(screen.getByText('monad-bft')).toBeInTheDocument())

  // Click restart for monad-bft
  const restartBtns = screen.getAllByRole('button', { name: /restart/i })
  await userEvent.click(restartBtns[0])

  // Modal should open
  await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument())

  // Type the node name
  await userEvent.type(screen.getByLabelText(/node name/i), NODE_NAME)

  // Type 6-digit TOTP
  await userEvent.type(screen.getByLabelText(/totp/i), '123456')

  // Click confirm
  const confirmBtn = screen.getByRole('button', { name: /confirm/i })
  expect(confirmBtn).toBeEnabled()
  await userEvent.click(confirmBtn)

  // Assert the POST body
  await waitFor(() => {
    expect(capturedRestartBody).toEqual({ unit: 'monad-bft.service', totp: '123456' })
  })
})
