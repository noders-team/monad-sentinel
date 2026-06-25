import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import RequireAuth from './components/RequireAuth'
import AppShell from './components/AppShell'
import { Overview } from './screens/Overview'

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { retry: 1 },
  },
})

function LoginPage() {
  return (
    <div data-testid="login-page" className="min-h-screen grid place-items-center bg-void text-ink">
      <div className="w-full max-w-sm p-8 bg-surface rounded-xl border border-line">
        <h1 className="text-2xl font-bold text-neon mb-6">Sentinel Login</h1>
        <p className="text-ink/60 text-sm">Authentication placeholder</p>
      </div>
    </div>
  )
}

function PlaceholderPage({ name }: { name: string }) {
  return (
    <div className="p-6">
      <h2 className="text-xl font-semibold text-ink">{name}</h2>
      <p className="text-ink/60 mt-2">Coming soon</p>
    </div>
  )
}

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <Routes>
          <Route path="/login" element={<LoginPage />} />
          <Route element={<RequireAuth />}>
            <Route element={<AppShell />}>
              <Route path="/" element={<Navigate to="/dashboard" replace />} />
              <Route path="/dashboard" element={<Overview />} />
              <Route path="/metrics" element={<PlaceholderPage name="Metrics" />} />
              <Route path="/logs" element={<PlaceholderPage name="Logs" />} />
              <Route path="/alerts" element={<PlaceholderPage name="Alerts" />} />
              <Route path="/upgrades" element={<PlaceholderPage name="Upgrades" />} />
              <Route path="/operations" element={<PlaceholderPage name="Operations" />} />
            </Route>
          </Route>
          <Route path="*" element={<Navigate to="/dashboard" replace />} />
        </Routes>
      </BrowserRouter>
    </QueryClientProvider>
  )
}
