export class ApiError extends Error {
  status: number
  constructor(status: number, message: string) {
    super(message)
    this.name = 'ApiError'
    this.status = status
  }
}

export function getCookie(name: string): string | undefined {
  return document.cookie.split('; ').find(c => c.startsWith(name + '='))?.split('=')[1]
}

async function handle<T>(res: Response): Promise<T> {
  if (!res.ok) throw new ApiError(res.status, await res.text().catch(() => res.statusText))
  const ct = res.headers.get('content-type') ?? ''
  return (ct.includes('application/json') ? res.json() : (undefined as T))
}

export async function apiGet<T>(path: string): Promise<T> {
  return handle<T>(await fetch(path, { credentials: 'include' }))
}

export async function apiPost<T>(path: string, body: unknown): Promise<T> {
  const csrf = getCookie('csrf')
  // x-csrf is intentionally omitted when no csrf cookie is present.
  // The login POST occurs before any csrf cookie exists, so adding the header
  // there would be a no-op at best. Privileged ops endpoints (restart, upgrade,
  // etc.) are always called post-login, at which point the cookie is set and
  // the backend enforces the csrf value matches.
  return handle<T>(await fetch(path, {
    method: 'POST',
    credentials: 'include',
    headers: {
      'content-type': 'application/json',
      ...(csrf ? { 'x-csrf': csrf } : {}),
    },
    body: JSON.stringify(body),
  }))
}
