import { http, HttpResponse } from 'msw'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { server } from '../test/mswServer'
import { renderWithProviders } from '../test/renderWithProviders'
import { Logs } from './Logs'

const services = [
  { name: 'monad-bft', unit: 'monad-bft.service', kind: 'systemd', active: true, version: '1.0.0' },
  { name: 'monad-exec', unit: 'monad-exec.service', kind: 'systemd', active: true, version: '1.0.0' },
]

let capturedLogsUnit = ''

beforeEach(() => {
  capturedLogsUnit = ''
  server.use(
    http.get('/api/status', () => HttpResponse.json(services)),
    http.get('/api/logs', ({ request }) => {
      const url = new URL(request.url)
      const unit = url.searchParams.get('unit') ?? ''
      capturedLogsUnit = unit

      // Simulate 400 for disallowed unit
      if (unit === 'disallowed.service') {
        return HttpResponse.text('unit not in allowlist', { status: 400 })
      }
      return HttpResponse.json({ lines: [`INFO Started ${unit}`, `DEBUG Ready`] })
    }),
  )
})

test('renders unit selector populated from useStatus', async () => {
  renderWithProviders(<Logs />)
  await waitFor(() => {
    expect(screen.getByRole('combobox', { name: /unit/i })).toBeInTheDocument()
    expect(screen.getByRole('option', { name: 'monad-bft' })).toBeInTheDocument()
    expect(screen.getByRole('option', { name: 'monad-exec' })).toBeInTheDocument()
  })
})

test('selecting a unit fetches logs with ?unit=<that unit>', async () => {
  renderWithProviders(<Logs />)

  // Wait for the select to populate
  await waitFor(() => expect(screen.getByRole('option', { name: 'monad-bft' })).toBeInTheDocument())

  const select = screen.getByRole('combobox', { name: /unit/i })
  await userEvent.selectOptions(select, 'monad-bft.service')

  await waitFor(() => {
    expect(capturedLogsUnit).toBe('monad-bft.service')
  })
})

test('shows log lines in the viewer', async () => {
  renderWithProviders(<Logs />)

  await waitFor(() => expect(screen.getByRole('option', { name: 'monad-bft' })).toBeInTheDocument())

  const select = screen.getByRole('combobox', { name: /unit/i })
  await userEvent.selectOptions(select, 'monad-bft.service')

  await waitFor(() => {
    expect(screen.getByText(/INFO Started monad-bft\.service/)).toBeInTheDocument()
  })
})

test('shows inline error on 400 without crashing', async () => {
  // Add handler for disallowed unit directly via MSW
  server.use(
    http.get('/api/logs', ({ request }) => {
      const url = new URL(request.url)
      const unit = url.searchParams.get('unit') ?? ''
      if (unit === 'disallowed.service') {
        return HttpResponse.text('unit not in allowlist', { status: 400 })
      }
      return HttpResponse.json({ lines: [] })
    }),
    http.get('/api/status', () =>
      HttpResponse.json([
        ...services,
        { name: 'disallowed', unit: 'disallowed.service', kind: 'systemd', active: false, version: null },
      ])
    ),
  )

  renderWithProviders(<Logs />)

  await waitFor(() => expect(screen.getByRole('option', { name: 'disallowed' })).toBeInTheDocument())

  const select = screen.getByRole('combobox', { name: /unit/i })
  await userEvent.selectOptions(select, 'disallowed.service')

  await waitFor(() => {
    expect(screen.getByRole('alert')).toBeInTheDocument()
  })

  // App should still be functional, not crash
  expect(screen.getByRole('combobox', { name: /unit/i })).toBeInTheDocument()
})
