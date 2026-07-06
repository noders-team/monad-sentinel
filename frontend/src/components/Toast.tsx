import { useEffect } from 'react'

interface ToastProps {
  message: string
  kind: 'success' | 'error' | 'info'
  onDismiss: () => void
}

const kindStyles: Record<ToastProps['kind'], { border: string; mark: string; glyph: string }> = {
  success: { border: 'border-ok', mark: 'text-ok', glyph: '✓' },
  error: { border: 'border-dgr', mark: 'text-dgr', glyph: '✕' },
  info: { border: 'border-acc', mark: 'text-acc-ink', glyph: 'ℹ' },
}

export function Toast({ message, kind, onDismiss }: ToastProps) {
  const style = kindStyles[kind]

  useEffect(() => {
    const id = setTimeout(onDismiss, 4000)
    return () => clearTimeout(id)
  }, [onDismiss])

  return (
    <div
      data-testid="toast"
      role="alert"
      className={`flex items-center gap-2.5 px-3.5 py-2.5 rounded-lg border bg-panel text-ink text-[12.5px] ${style.border}`}
      style={{ boxShadow: '0 12px 32px rgba(0,0,0,0.4)' }}
    >
      <span className={`${style.mark} font-semibold`} aria-hidden="true">{style.glyph}</span>
      <span className="flex-1">{message}</span>
      <button
        type="button"
        aria-label="dismiss"
        onClick={onDismiss}
        className="ml-1 text-mut hover:text-ink transition-colors"
      >
        ✕
      </button>
    </div>
  )
}
