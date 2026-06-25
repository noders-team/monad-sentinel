import { apiPost, apiGet, ApiError } from './client'
import { server } from '../test/mswServer'
import { http, HttpResponse } from 'msw'

test('apiPost sends x-csrf from the csrf cookie', async () => {
  document.cookie = 'csrf=tok123'
  let seen: string | null = null
  server.use(
    http.post('/api/ops/restart', ({ request }) => {
      seen = request.headers.get('x-csrf')
      return HttpResponse.json({ ok: true, detail: 'ok' })
    })
  )
  await apiPost('/api/ops/restart', { unit: 'monad-bft.service', totp: '123456' })
  expect(seen).toBe('tok123')
})

test('apiGet throws ApiError with status on non-2xx', async () => {
  server.use(http.get('/api/status', () => new HttpResponse(null, { status: 401 })))
  await expect(apiGet('/api/status')).rejects.toMatchObject({ name: 'ApiError', status: 401 })
})

test('ApiError has the right name and status', () => {
  const err = new ApiError(403, 'Forbidden')
  expect(err.name).toBe('ApiError')
  expect(err.status).toBe(403)
  expect(err.message).toBe('Forbidden')
  expect(err instanceof ApiError).toBe(true)
  expect(err instanceof Error).toBe(true)
})

test('apiPost does NOT send x-csrf header when csrf cookie is absent', async () => {
  // Clear the csrf cookie so it is absent (login scenario — cookie not yet issued)
  document.cookie = 'csrf=; expires=Thu, 01 Jan 1970 00:00:00 GMT; path=/'
  let seen: string | null | undefined = undefined
  server.use(
    http.post('/api/auth/login', ({ request }) => {
      seen = request.headers.get('x-csrf')
      return HttpResponse.json({ ok: true })
    })
  )
  await apiPost('/api/auth/login', { username: 'admin', password: 'secret' })
  // Header must be absent (null), not an empty string
  expect(seen).toBeNull()
})
