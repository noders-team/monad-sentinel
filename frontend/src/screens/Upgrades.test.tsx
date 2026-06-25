import { http, HttpResponse } from 'msw'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { server } from '../test/mswServer'
import { renderWithProviders } from '../test/renderWithProviders'
import { Upgrades } from './Upgrades'
import { NODE_NAME } from '../constants'

let capturedUpgradeBody: unknown = null
let capturedRollbackBody: unknown = null

beforeEach(() => {
  capturedUpgradeBody = null
  capturedRollbackBody = null
  server.use(
    http.get('/api/auth/me', () => HttpResponse.json({ actor: 'admin' })),
    http.get('/api/upgrades', () =>
      HttpResponse.json({
        current: 'v0.14.5',
        candidate: '0.14.7',
        rollback_point: null,
        target: null,
        deadline: null,
      })
    ),
    http.post('/api/ops/upgrade', async ({ request }) => {
      capturedUpgradeBody = await request.json()
      return HttpResponse.json({ ok: true, detail: 'upgraded' })
    }),
    http.post('/api/ops/rollback', async ({ request }) => {
      capturedRollbackBody = await request.json()
      return HttpResponse.json({ ok: true, detail: 'rolled back' })
    }),
    http.post('/api/upgrades/plan', () => HttpResponse.json({ ok: true })),
  )
})

test('shows candidate version', async () => {
  renderWithProviders(<Upgrades />)

  await waitFor(() => {
    expect(screen.getByText('0.14.7')).toBeInTheDocument()
  })
})

test('run upgrade flow: opens modal, submits, shows toast', async () => {
  renderWithProviders(<Upgrades />)

  // Wait for upgrades to load
  await waitFor(() => expect(screen.getByText('0.14.7')).toBeInTheDocument())

  // Click "Run upgrade"
  await userEvent.click(screen.getByRole('button', { name: /run upgrade/i }))

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

  // Assert POST body
  await waitFor(() => {
    expect(capturedUpgradeBody).toEqual({ target_version: '0.14.7', totp: '123456' })
  })

  // Assert success toast shows
  await waitFor(() => {
    expect(screen.getByTestId('toast')).toBeInTheDocument()
  })
})

test('rollback flow: shows rollback button, opens modal, submits, shows toast', async () => {
  server.use(
    http.get('/api/upgrades', () =>
      HttpResponse.json({
        current: 'v0.14.5',
        candidate: null,
        rollback_point: '0.14.5',
        target: null,
        deadline: null,
      })
    ),
  )

  renderWithProviders(<Upgrades />)

  // Wait for rollback_point to appear
  await waitFor(() => expect(screen.getByText('0.14.5')).toBeInTheDocument())

  // Rollback button should be visible
  const rollbackBtn = screen.getByRole('button', { name: /rollback/i })
  expect(rollbackBtn).toBeInTheDocument()

  // Click rollback
  await userEvent.click(rollbackBtn)

  // Modal should open
  await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument())

  // Type the node name
  await userEvent.type(screen.getByLabelText(/node name/i), NODE_NAME)

  // Type 6-digit TOTP
  await userEvent.type(screen.getByLabelText(/totp/i), '654321')

  // Click confirm
  const confirmBtn = screen.getByRole('button', { name: /confirm/i })
  expect(confirmBtn).toBeEnabled()
  await userEvent.click(confirmBtn)

  // Assert POST body
  await waitFor(() => {
    expect(capturedRollbackBody).toEqual({ totp: '654321' })
  })

  // Assert success toast shows
  await waitFor(() => {
    expect(screen.getByTestId('toast')).toBeInTheDocument()
  })
})
