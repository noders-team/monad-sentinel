type Window = '1h' | '24h' | '7d'

interface WindowPickerProps {
  value: Window
  onChange: (w: Window) => void
}

const WINDOWS: Window[] = ['1h', '24h', '7d']

export function WindowPicker({ value, onChange }: WindowPickerProps) {
  return (
    <div className="inline-flex rounded-full border border-line overflow-hidden" role="group" aria-label="Time window">
      {WINDOWS.map(w => (
        <button
          key={w}
          type="button"
          aria-pressed={value === w}
          onClick={() => onChange(w)}
          className={[
            'px-3 py-[5px] text-[11px] font-semibold whitespace-nowrap transition-colors',
            value === w
              ? 'bg-acc-soft text-acc-ink'
              : 'text-mut hover:text-acc-ink',
          ].join(' ')}
        >
          {w}
        </button>
      ))}
    </div>
  )
}
