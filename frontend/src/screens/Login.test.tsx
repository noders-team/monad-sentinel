import { http, HttpResponse } from 'msw'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Routes, Route } from 'react-router-dom'
import { server } from '../test/mswServer'
import { renderWithProviders } from '../test/renderWithProviders'
import { Login } from './Login'

beforeEach(() => {
  server.use(
    http.post('/api/auth/login', () =>
      HttpResponse.json({ actor: 'admin' })
    ),
  )
})

test('shows invalid credentials on 401', async () => {
  server.use(
    http.post('/api/auth/login', () =>
      HttpResponse.json({ error: 'unauthorized' }, { status: 401 })
    ),
  )

  renderWithProviders(<Login />)

  await userEvent.type(screen.getByLabelText(/username/i), 'admin')
  await userEvent.type(screen.getByLabelText(/password/i), 'wrongpassword')
  await userEvent.click(screen.getByRole('button', { name: /login/i }))

  await waitFor(() => {
    expect(screen.getByText(/invalid credentials/i)).toBeInTheDocument()
  })
})

test('navigates to / on successful login', async () => {
  renderWithProviders(
    <Routes>
      <Route path="/login" element={<Login />} />
      <Route path="/" element={<div>dashboard-home</div>} />
    </Routes>,
    { initialEntries: ['/login'] },
  )

  await userEvent.type(screen.getByLabelText(/username/i), 'admin')
  await userEvent.type(screen.getByLabelText(/password/i), 'correctpassword')
  await userEvent.click(screen.getByRole('button', { name: /login/i }))

  await waitFor(() => {
    expect(screen.getByText('dashboard-home')).toBeInTheDocument()
  })
})
