import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Toast } from './Toast'

test('renders message and dismiss button', () => {
  const onDismiss = vi.fn()
  render(<Toast message="Operation complete" kind="success" onDismiss={onDismiss} />)
  expect(screen.getByText('Operation complete')).toBeInTheDocument()
  expect(screen.getByRole('button', { name: /dismiss/i })).toBeInTheDocument()
})

test('calls onDismiss when dismiss button clicked', async () => {
  const onDismiss = vi.fn()
  render(<Toast message="Error occurred" kind="error" onDismiss={onDismiss} />)
  await userEvent.click(screen.getByRole('button', { name: /dismiss/i }))
  expect(onDismiss).toHaveBeenCalledTimes(1)
})

test('applies success styling', () => {
  render(<Toast message="Done!" kind="success" onDismiss={() => {}} />)
  expect(screen.getByTestId('toast')).toHaveClass('border-ok')
})

test('applies error styling', () => {
  render(<Toast message="Failed!" kind="error" onDismiss={() => {}} />)
  expect(screen.getByTestId('toast')).toHaveClass('border-dgr')
})
