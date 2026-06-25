import { Navigate, Outlet } from 'react-router-dom'
import { useMe } from '../api/hooks'

export default function RequireAuth() {
  const { data, isLoading, error } = useMe()

  if (isLoading) return null

  if (error || !data) {
    return <Navigate to="/login" replace />
  }

  return <Outlet />
}
