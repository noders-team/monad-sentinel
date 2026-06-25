import { useState } from 'react'

interface ConfirmOpModalProps {
  nodeName: string
  title: string
  onConfirm: (totp: string) => void
  pending?: boolean
  error?: string
}

export function ConfirmOpModal({ nodeName, title, onConfirm, pending = false, error }: ConfirmOpModalProps) {
  const [typedName, setTypedName] = useState('')
  const [totp, setTotp] = useState('')

  const isValid = typedName === nodeName && /^\d{6}$/.test(totp)

  function handleConfirm() {
    if (isValid) {
      onConfirm(totp)
    }
  }

  return (
    <div role="dialog" aria-modal="true" className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="bg-surface border border-line rounded-xl p-6 w-full max-w-md shadow-xl">
        <h2 className="text-lg font-semibold text-ink mb-4">{title}</h2>

        <div className="space-y-4">
          <div>
            <label htmlFor="confirm-node-name" className="block text-sm text-ink/60 mb-1">
              Node name — type <span className="text-neon font-mono">{nodeName}</span> to confirm
            </label>
            <input
              id="confirm-node-name"
              type="text"
              value={typedName}
              onChange={e => setTypedName(e.target.value)}
              className="w-full bg-void border border-line rounded px-3 py-2 text-ink text-sm outline-none focus:border-neon"
              placeholder={nodeName}
            />
          </div>

          <div>
            <label htmlFor="confirm-totp" className="block text-sm text-ink/60 mb-1">
              TOTP (6-digit code)
            </label>
            <input
              id="confirm-totp"
              type="text"
              inputMode="numeric"
              maxLength={6}
              value={totp}
              onChange={e => setTotp(e.target.value.replace(/\D/g, '').slice(0, 6))}
              className="w-full bg-void border border-line rounded px-3 py-2 text-ink text-sm outline-none focus:border-neon font-mono tracking-widest"
              placeholder="000000"
            />
          </div>

          {error && (
            <p className="text-red-400 text-sm">{error}</p>
          )}
        </div>

        <div className="flex justify-end gap-3 mt-6">
          <button
            type="button"
            onClick={handleConfirm}
            disabled={!isValid || pending}
            className="px-4 py-2 rounded bg-neon/10 text-neon border border-neon/30 text-sm font-medium
              disabled:opacity-40 disabled:cursor-not-allowed hover:bg-neon/20 transition-colors"
          >
            {pending ? 'Confirming…' : 'Confirm'}
          </button>
        </div>
      </div>
    </div>
  )
}
