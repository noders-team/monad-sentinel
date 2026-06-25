import { http, HttpResponse } from 'msw'
import { screen, waitFor } from '@testing-library/react'
import { server } from '../test/mswServer'
import { renderWithProviders } from '../test/renderWithProviders'
import { Operations } from './Operations'

const rowA = {
  ts_ms: 1700001000000,
  actor: 'admin',
  op: 'restart',
  params: '{}',
  result: 'ok',
  detail: 'restarted monad-bft',
}

const rowB = {
  ts_ms: 1700000000000,
  actor: 'admin',
  op: 'restart',
  params: '{}',
  result: 'denied',
  detail: 'invalid totp',
}

beforeEach(() => {
  server.use(
    http.get('/api/auth/me', () => HttpResponse.json({ actor: 'admin' })),
    http.get('/api/audit', () => HttpResponse.json([rowB, rowA])),
  )
})

test('renders both rows newest-first (rowA before rowB)', async () => {
  renderWithProviders(<Operations />)
  await waitFor(() => {
    expect(screen.getByText('restarted monad-bft')).toBeInTheDocument()
    expect(screen.getByText('invalid totp')).toBeInTheDocument()
  })

  // Assert row A appears before row B in document order
  const cells = screen.getAllByText(/monad-bft|invalid totp/)
  const aIndex = cells.findIndex(el => el.textContent === 'restarted monad-bft')
  const bIndex = cells.findIndex(el => el.textContent === 'invalid totp')
  expect(aIndex).toBeLessThan(bIndex)
})

test('shows op, result, and detail text for each row', async () => {
  renderWithProviders(<Operations />)
  await waitFor(() => {
    // op
    const opCells = screen.getAllByText('restart')
    expect(opCells.length).toBeGreaterThanOrEqual(2)
    // result
    expect(screen.getByText('ok')).toBeInTheDocument()
    expect(screen.getByText('denied')).toBeInTheDocument()
    // detail
    expect(screen.getByText('restarted monad-bft')).toBeInTheDocument()
    expect(screen.getByText('invalid totp')).toBeInTheDocument()
  })
})

test('denied row has data-result="denied" attribute distinct from ok row', async () => {
  renderWithProviders(<Operations />)
  await waitFor(() => {
    expect(screen.getByText('denied')).toBeInTheDocument()
  })

  const deniedCell = document.querySelector('[data-result="denied"]')
  const okCell = document.querySelector('[data-result="ok"]')

  expect(deniedCell).not.toBeNull()
  expect(okCell).not.toBeNull()
  // They should be different elements
  expect(deniedCell).not.toBe(okCell)
  // denied cell should not have the ok class
  expect(deniedCell?.className).not.toEqual(okCell?.className)
})
