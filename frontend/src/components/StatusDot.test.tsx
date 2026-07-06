import { render, screen } from '@testing-library/react'
import { StatusDot } from './StatusDot'

test('renders ok dot when active', () => {
  render(<StatusDot active={true} />)
  const dot = screen.getByTestId('status-dot')
  expect(dot).toHaveClass('bg-ok')
})

test('renders danger dot when inactive', () => {
  render(<StatusDot active={false} />)
  const dot = screen.getByTestId('status-dot')
  expect(dot).toHaveClass('bg-dgr')
})
