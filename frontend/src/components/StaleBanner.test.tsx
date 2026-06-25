import { render, screen } from '@testing-library/react'
import { StaleBanner } from './StaleBanner'

test('renders stale banner when isError and hasData are both true', () => {
  render(<StaleBanner isError={true} hasData={true} />)
  const banner = screen.getByTestId('stale-banner')
  expect(banner).toBeInTheDocument()
  expect(banner).toHaveTextContent('stale — retrying')
})

test('renders nothing when isError is false and hasData is true', () => {
  const { container } = render(<StaleBanner isError={false} hasData={true} />)
  expect(container.firstChild).toBeNull()
})

test('renders nothing when isError is true but hasData is false', () => {
  const { container } = render(<StaleBanner isError={true} hasData={false} />)
  expect(container.firstChild).toBeNull()
})
