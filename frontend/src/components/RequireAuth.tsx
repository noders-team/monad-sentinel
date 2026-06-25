import { Navigate, Outlet } from 'react-router-dom'
import { useMe } from '../api/hooks'
import { ApiError } from '../api/client'

export default function RequireAuth() {
  const { data, isLoading, error } = useMe()

  if (isLoading) return null

  // Only redirect to login on explicit 401 Unauthorized.
  // Other errors (500, network failure, etc.) are transient — the underlying
  // useMe query will keep retrying/refetching. Bouncing the user to /login
  // on a backend outage would mask the real problem.
  if (error instanceof ApiError && error.status === 401) {
    return <Navigate to="/login" replace />
  }

  if (error) {
    return (
      <div className="min-h-screen grid place-items-center text-ink/60 text-sm">
        Connection error — retrying…
      </div>
    )
  }

  if (!data) {
    return <Navigate to="/login" replace />
  }

  return <Outlet />
}
