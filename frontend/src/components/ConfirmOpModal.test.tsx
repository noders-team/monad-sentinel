import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { ConfirmOpModal } from './ConfirmOpModal'

test('confirm disabled until node name matches and TOTP is 6 digits', async () => {
  const onConfirm = vi.fn()
  render(<ConfirmOpModal nodeName="ltd-t-node-02" title="Restart monad-bft" onConfirm={onConfirm} />)
  const btn = screen.getByRole('button', { name: /confirm/i })
  expect(btn).toBeDisabled()
  await userEvent.type(screen.getByLabelText(/node name/i), 'ltd-t-node-02')
  await userEvent.type(screen.getByLabelText(/totp/i), '12345')   // 5 digits
  expect(btn).toBeDisabled()
  await userEvent.type(screen.getByLabelText(/totp/i), '6')        // now 6
  expect(btn).toBeEnabled()
  await userEvent.click(btn)
  expect(onConfirm).toHaveBeenCalledWith('123456')
})
