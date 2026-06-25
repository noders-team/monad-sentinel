import { render, screen, waitFor } from '@testing-library/react'
import { http, HttpResponse } from 'msw'
import { server } from './test/mswServer'
import App from './App'

test('renders login page when not authenticated', async () => {
  server.use(
    http.get('/api/auth/me', () => new HttpResponse(null, { status: 401 })),
    http.get('/api/status', () => HttpResponse.json([])),
    http.get('/api/alerts', () => HttpResponse.json([])),
    http.get('/api/upgrades', () => HttpResponse.json({ current: null, candidate: null, target: null, deadline: null, rollback_point: null })),
  )
  render(<App />)
  await waitFor(() => {
    expect(screen.getByTestId('login-page')).toBeInTheDocument()
  })
})
