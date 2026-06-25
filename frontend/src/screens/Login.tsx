import { useState, type FormEvent } from 'react'
import { useNavigate } from 'react-router-dom'
import { useLogin } from '../api/hooks'
import { ApiError } from '../api/client'

export function Login() {
  const navigate = useNavigate()
  const login = useLogin()
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [errorMsg, setErrorMsg] = useState<string | null>(null)

  function handleSubmit(e: FormEvent) {
    e.preventDefault()
    setErrorMsg(null)
    login.mutate(
      { username, password },
      {
        onSuccess: () => {
          void navigate('/')
        },
        onError: (err) => {
          if (err instanceof ApiError && err.status === 401) {
            setErrorMsg('invalid credentials')
          } else {
            setErrorMsg('login failed')
          }
        },
      }
    )
  }

  return (
    <div data-testid="login-page" className="min-h-screen grid place-items-center bg-void text-ink">
      <div className="w-full max-w-sm p-8 bg-surface rounded-xl border border-line">
        <h1 className="text-2xl font-bold text-neon mb-6">Sentinel Login</h1>
        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label htmlFor="username" className="block text-sm text-ink/60 mb-1">
              Username
            </label>
            <input
              id="username"
              type="text"
              value={username}
              onChange={e => setUsername(e.target.value)}
              autoComplete="username"
              required
              className="w-full bg-void border border-line rounded px-3 py-2 text-ink text-sm outline-none focus:border-neon"
            />
          </div>
          <div>
            <label htmlFor="password" className="block text-sm text-ink/60 mb-1">
              Password
            </label>
            <input
              id="password"
              type="password"
              value={password}
              onChange={e => setPassword(e.target.value)}
              autoComplete="current-password"
              required
              className="w-full bg-void border border-line rounded px-3 py-2 text-ink text-sm outline-none focus:border-neon"
            />
          </div>
          {errorMsg && (
            <p className="text-red-400 text-sm">{errorMsg}</p>
          )}
          <button
            type="submit"
            disabled={login.isPending}
            className="w-full py-2 rounded bg-neon/10 text-neon border border-neon/30 text-sm font-medium
              disabled:opacity-40 disabled:cursor-not-allowed hover:bg-neon/20 transition-colors"
          >
            {login.isPending ? 'Logging in…' : 'Login'}
          </button>
        </form>
      </div>
    </div>
  )
}
