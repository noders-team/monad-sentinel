import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter } from 'react-router-dom'
import { render } from '@testing-library/react'
import type { RenderOptions } from '@testing-library/react'
import type { ReactNode } from 'react'

export function renderWithProviders(
  ui: ReactNode,
  {
    initialEntries = ['/'],
    ...options
  }: RenderOptions & { initialEntries?: string[] } = {}
) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter initialEntries={initialEntries}>
        {ui}
      </MemoryRouter>
    </QueryClientProvider>,
    options
  )
}
