interface LogViewerProps {
  lines: string[]
  levelFilter: string
}

const LEVEL_COLORS: Record<string, string> = {
  ERROR: 'text-red-400',
  WARN: 'text-amber-400',
  WARNING: 'text-amber-400',
  INFO: 'text-sky-400',
  DEBUG: 'text-ink/50',
}

function lineColor(line: string): string {
  for (const [level, cls] of Object.entries(LEVEL_COLORS)) {
    if (line.toUpperCase().includes(level)) return cls
  }
  return 'text-ink/70'
}

export function LogViewer({ lines, levelFilter }: LogViewerProps) {
  const visible = levelFilter
    ? lines.filter(l => l.toUpperCase().includes(levelFilter.toUpperCase()))
    : lines

  if (visible.length === 0) {
    return (
      <div className="bg-void border border-line rounded-xl p-4 font-mono text-xs text-ink/30">
        No log lines
      </div>
    )
  }

  return (
    <div className="bg-void border border-line rounded-xl p-4 font-mono text-xs overflow-auto max-h-[60vh]">
      {visible.map((line, i) => (
        <div key={i} className={lineColor(line)}>
          {line}
        </div>
      ))}
    </div>
  )
}
