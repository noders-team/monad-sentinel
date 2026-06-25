interface ToastProps {
  message: string
  kind: 'success' | 'error' | 'info'
  onDismiss: () => void
}

const kindStyles: Record<ToastProps['kind'], string> = {
  success: 'border-green-500 bg-green-500/10 text-green-300',
  error: 'border-red-500 bg-red-500/10 text-red-300',
  info: 'border-sky-500 bg-sky-500/10 text-sky-300',
}

export function Toast({ message, kind, onDismiss }: ToastProps) {
  return (
    <div
      data-testid="toast"
      role="alert"
      className={`flex items-center gap-3 px-4 py-3 rounded-lg border text-sm shadow-lg ${kindStyles[kind]}`}
    >
      <span className="flex-1">{message}</span>
      <button
        type="button"
        aria-label="dismiss"
        onClick={onDismiss}
        className="ml-2 opacity-70 hover:opacity-100 transition-opacity text-current"
      >
        ✕
      </button>
    </div>
  )
}
