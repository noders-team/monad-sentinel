import { useEffect, useRef, useState } from 'react'

interface ConfirmOpModalProps {
  nodeName: string
  title: string
  onConfirm: (totp: string) => void
  onCancel?: () => void
  pending?: boolean
  error?: string
}

export function ConfirmOpModal({ nodeName, title, onConfirm, onCancel, pending = false, error }: ConfirmOpModalProps) {
  const [typedName, setTypedName] = useState('')
  const [totp, setTotp] = useState('')
  const firstFieldRef = useRef<HTMLInputElement>(null)

  const isValid = typedName === nodeName && /^\d{6}$/.test(totp)

  useEffect(() => {
    firstFieldRef.current?.focus()
  }, [])

  useEffect(() => {
    if (!onCancel) return
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') onCancel!()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [onCancel])

  function handleConfirm() {
    if (isValid) onConfirm(totp)
  }

  const inputClass =
    'w-full bg-bg border border-line rounded px-[10px] py-2 text-ink text-[13px] outline-none focus:border-acc font-mono'

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label={title}
      className="fixed inset-0 z-50 flex items-center justify-center px-4"
      style={{ background: 'rgba(5,3,12,0.72)' }}
    >
      <div
        className="bg-panel border border-line rounded-[10px] w-full max-w-[400px] px-6 py-[22px]"
        style={{ boxShadow: '0 24px 64px rgba(0,0,0,0.5)' }}
      >
        <h2 className="text-[15px] font-semibold text-ink">{title}</h2>
        <p className="text-[11.5px] text-mut mt-1">
          Privileged operation — verified server-side with CSRF + TOTP, recorded in the audit log.
        </p>

        <div className="mt-5 space-y-4">
          <div>
            <label htmlFor="confirm-node-name" className="block eyebrow mb-1.5">
              Node name — type <span className="text-acc-ink font-mono normal-case tracking-normal">{nodeName}</span> to confirm
            </label>
            <input
              id="confirm-node-name"
              ref={firstFieldRef}
              type="text"
              value={typedName}
              onChange={e => setTypedName(e.target.value)}
              className={inputClass}
              placeholder={nodeName}
            />
          </div>

          <div>
            <label htmlFor="confirm-totp" className="block eyebrow mb-1.5">
              TOTP · 6-digit code
            </label>
            <input
              id="confirm-totp"
              type="text"
              inputMode="numeric"
              maxLength={6}
              value={totp}
              onChange={e => setTotp(e.target.value.replace(/\D/g, '').slice(0, 6))}
              className={`${inputClass} text-[14px]`}
              style={{ letterSpacing: '0.4em' }}
              placeholder="000000"
            />
          </div>

          {error && <p className="text-dgr text-[12.5px]">{error}</p>}
        </div>

        <div className="flex justify-end gap-3 mt-6">
          {onCancel && (
            <button
              type="button"
              onClick={onCancel}
              className="px-4 py-2 rounded border border-line text-mut text-[12.5px] hover:text-acc-ink hover:border-acc transition-colors"
            >
              Cancel
            </button>
          )}
          <button
            type="button"
            onClick={handleConfirm}
            disabled={!isValid || pending}
            className="px-4 py-2 rounded text-[12.5px] font-semibold transition
              bg-acc text-[#FBFAF9] hover:brightness-110
              disabled:bg-panel2 disabled:text-mut disabled:cursor-not-allowed disabled:hover:brightness-100"
          >
            {pending ? 'Confirming…' : 'Confirm'}
          </button>
        </div>
      </div>
    </div>
  )
}
