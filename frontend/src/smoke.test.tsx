import { render, screen } from '@testing-library/react'
import App from './App'

test('renders the app shell marker', () => {
  render(<App />)
  expect(screen.getByText('Sentinel Console')).toBeInTheDocument()
})
