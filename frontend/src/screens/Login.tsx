import { useState, type FormEvent } from 'react'
import { useNavigate } from 'react-router-dom'
import { useLogin } from '../api/hooks'
import { ApiError } from '../api/client'

function Logo() {
  return (
    <svg width="26" height="26" viewBox="0 0 24 24" aria-hidden="true">
      <rect x="4.2" y="4.2" width="15.6" height="15.6" rx="5" transform="rotate(45 12 12)" fill="#836EF9" />
      <rect x="8.4" y="8.4" width="7.2" height="7.2" rx="2.4" transform="rotate(45 12 12)" fill="#FBFAF9" />
    </svg>
  )
}

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
        onSuccess: () => { void navigate('/') },
        onError: (err) => {
          setErrorMsg(err instanceof ApiError && err.status === 401 ? 'invalid credentials' : 'login failed')
        },
      }
    )
  }

  const inputClass = 'w-full bg-bg border border-line rounded px-[10px] py-2 text-ink text-[13px] outline-none focus:border-acc'

  return (
    <div data-testid="login-page" className="min-h-screen grid place-items-center bg-bg text-ink">
      <div className="w-[340px] p-7 bg-panel rounded-[10px] border border-line">
        <div className="flex items-center gap-2">
          <Logo />
          <span className="text-[16px] font-bold tracking-[0.14em]">SENTINEL</span>
        </div>
        <p className="text-[12px] text-mut mt-2">Monad validator control plane · VPN-only</p>

        <form onSubmit={handleSubmit} className="space-y-4 mt-6">
          <div>
            <label htmlFor="username" className="block eyebrow mb-1.5">Username</label>
            <input
              id="username"
              type="text"
              value={username}
              onChange={e => setUsername(e.target.value)}
              autoComplete="username"
              required
              className={inputClass}
            />
          </div>
          <div>
            <label htmlFor="password" className="block eyebrow mb-1.5">Password</label>
            <input
              id="password"
              type="password"
              value={password}
              onChange={e => setPassword(e.target.value)}
              autoComplete="current-password"
              required
              className={inputClass}
            />
          </div>
          {errorMsg && <p className="text-dgr text-[12.5px]">{errorMsg}</p>}
          <button
            type="submit"
            disabled={login.isPending}
            className="w-full py-2 rounded bg-acc text-[#FBFAF9] text-[12.5px] font-semibold
              hover:brightness-110 transition disabled:bg-panel2 disabled:text-mut disabled:cursor-not-allowed disabled:hover:brightness-100"
          >
            {login.isPending ? 'Signing in…' : 'Sign in'}
          </button>
        </form>

        <p className="font-mono text-[11px] text-mut mt-6">noders-mainnet-01 · sentinel-web 0.6.2</p>
      </div>
    </div>
  )
}
