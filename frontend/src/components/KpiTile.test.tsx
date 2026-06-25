import { render, screen } from '@testing-library/react'
import { KpiTile } from './KpiTile'

test('renders label and value', () => {
  render(<KpiTile label="Block Height" value="1,234,567" />)
  expect(screen.getByText('Block Height')).toBeInTheDocument()
  expect(screen.getByText('1,234,567')).toBeInTheDocument()
})

test('renders loading state when value is undefined', () => {
  render(<KpiTile label="Uptime" value={undefined} />)
  expect(screen.getByText('Uptime')).toBeInTheDocument()
  expect(screen.getByTestId('kpi-loading')).toBeInTheDocument()
})

test('renders sub-label when provided', () => {
  render(<KpiTile label="Vote Rate" value="98.5%" subLabel="last 24h" />)
  expect(screen.getByText('last 24h')).toBeInTheDocument()
})
