import { http, HttpResponse } from 'msw'
import { screen, waitFor } from '@testing-library/react'
import { server } from '../test/mswServer'
import { renderWithProviders } from '../test/renderWithProviders'
import { Alerts } from './Alerts'

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
      HttpResponse.json([
        { ts_ms: 1700000000000, rule: 'participation_loss', message: 'Vote rate dropped' },
      ])
    ),
  )
  renderWithProviders(<Alerts />)
  await waitFor(() => {
    expect(screen.getByText('participation_loss')).toBeInTheDocument()
    expect(screen.getByText('Vote rate dropped')).toBeInTheDocument()
  })
})
