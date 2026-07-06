interface StatusDotProps {
  active: boolean
  size?: 'sm' | 'md'
  pulse?: boolean
}

export function StatusDot({ active, size = 'md', pulse = false }: StatusDotProps) {
  const sizeClass = size === 'sm' ? 'w-[7px] h-[7px]' : 'w-2.5 h-2.5'
  const colorClass = active ? 'bg-ok' : 'bg-dgr'

  return (
    <span
      data-testid="status-dot"
      className={`inline-block rounded-full ${sizeClass} ${colorClass} shrink-0 ${pulse ? 'animate-pulse' : ''}`}
      aria-label={active ? 'active' : 'inactive'}
    />
  )
}
