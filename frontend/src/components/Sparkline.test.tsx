import { render, screen } from '@testing-library/react'
import { Sparkline } from './Sparkline'
import type { MetricsResp } from '../api/types'

const mockData: MetricsResp = {
  points: [
    [1700000000, 0.95],
    [1700003600, 0.98],
    [1700007200, 0.91],
  ],
}

test('renders sparkline chart container', () => {
  render(<Sparkline data={mockData} />)
  expect(screen.getByTestId('sparkline')).toBeInTheDocument()
})

test('renders empty state when no points', () => {
  render(<Sparkline data={{ points: [] }} />)
  expect(screen.getByTestId('sparkline-empty')).toBeInTheDocument()
})
