type Window = '1h' | '24h' | '7d'

interface WindowPickerProps {
  value: Window
  onChange: (w: Window) => void
}

const WINDOWS: Window[] = ['1h', '24h', '7d']

export function WindowPicker({ value, onChange }: WindowPickerProps) {
  return (
    <div className="inline-flex rounded-lg border border-line overflow-hidden" role="group" aria-label="Time window">
      {WINDOWS.map(w => (
        <button
          key={w}
          type="button"
          aria-pressed={value === w}
          onClick={() => onChange(w)}
          className={[
            'px-3 py-1.5 text-sm font-medium transition-colors',
            value === w
              ? 'bg-neon/20 text-neon border-neon/40'
              : 'text-ink/60 hover:text-ink hover:bg-surface',
          ].join(' ')}
        >
          {w}
        </button>
      ))}
    </div>
  )
}
