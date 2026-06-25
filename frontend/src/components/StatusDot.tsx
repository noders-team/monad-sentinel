interface StatusDotProps {
  active: boolean
  size?: 'sm' | 'md'
}

export function StatusDot({ active, size = 'md' }: StatusDotProps) {
  const sizeClass = size === 'sm' ? 'w-2 h-2' : 'w-3 h-3'
  const colorClass = active ? 'bg-green-500' : 'bg-red-500'

  return (
    <span
      data-testid="status-dot"
      className={`inline-block rounded-full ${sizeClass} ${colorClass} flex-shrink-0`}
      aria-label={active ? 'active' : 'inactive'}
    />
  )
}
