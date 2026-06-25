import { screen, waitFor } from '@testing-library/react'
import { Routes, Route } from 'react-router-dom'
import { http, HttpResponse } from 'msw'
import { server } from '../test/mswServer'
import { renderWithProviders } from '../test/renderWithProviders'
import RequireAuth from './RequireAuth'

function LoginPage() {
  return <div data-testid="login-page">Login</div>
}

function ProtectedPage() {
  return <div data-testid="protected-page">Protected</div>
}

test('redirects to /login when /api/auth/me returns 401', async () => {
  server.use(
    http.get('/api/auth/me', () => new HttpResponse(null, { status: 401 }))
  )
  renderWithProviders(
    <Routes>
      <Route path="/login" element={<LoginPage />} />
      <Route element={<RequireAuth />}>
        <Route path="/" element={<ProtectedPage />} />
      </Route>
    </Routes>
  )
  await waitFor(() => {
    expect(screen.getByTestId('login-page')).toBeInTheDocument()
    expect(screen.queryByTestId('protected-page')).not.toBeInTheDocument()
  })
})

test('renders outlet when /api/auth/me returns user', async () => {
  server.use(
    http.get('/api/auth/me', () => HttpResponse.json({ actor: 'admin' }))
  )
  renderWithProviders(
    <Routes>
      <Route path="/login" element={<LoginPage />} />
      <Route element={<RequireAuth />}>
        <Route path="/" element={<ProtectedPage />} />
      </Route>
    </Routes>
  )
  await waitFor(() => {
    expect(screen.getByTestId('protected-page')).toBeInTheDocument()
  })
})

test('does NOT redirect to /login when /api/auth/me returns 500 — shows error state instead', async () => {
  server.use(
    http.get('/api/auth/me', () => new HttpResponse(null, { status: 500 }))
  )
  renderWithProviders(
    <Routes>
      <Route path="/login" element={<LoginPage />} />
      <Route element={<RequireAuth />}>
        <Route path="/" element={<ProtectedPage />} />
      </Route>
    </Routes>
  )
  await waitFor(() => {
    expect(screen.queryByTestId('protected-page')).not.toBeInTheDocument()
    expect(screen.queryByTestId('login-page')).not.toBeInTheDocument()
    expect(screen.getByText(/connection error/i)).toBeInTheDocument()
  })
})
