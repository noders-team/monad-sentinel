import { render, screen } from '@testing-library/react'
import { StatusDot } from './StatusDot'

test('renders green dot when active', () => {
  render(<StatusDot active={true} />)
  const dot = screen.getByTestId('status-dot')
  expect(dot).toHaveClass('bg-green-500')
})

test('renders red dot when inactive', () => {
  render(<StatusDot active={false} />)
  const dot = screen.getByTestId('status-dot')
  expect(dot).toHaveClass('bg-red-500')
})
