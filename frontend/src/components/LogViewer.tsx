interface LogViewerProps {
  lines: string[]
  levelFilter: string
}

const LEVEL_COLORS: Record<string, string> = {
  ERROR: 'text-dgr',
  WARN: 'text-warn',
  WARNING: 'text-warn',
  INFO: 'text-acc-ink',
  DEBUG: 'text-mut',
}

function lineColor(line: string): string {
  for (const [level, cls] of Object.entries(LEVEL_COLORS)) {
    if (line.toUpperCase().includes(level)) return cls
  }
  return 'text-mut'
}

const boxClass =
  'bg-bg border border-line rounded-[10px] px-[14px] py-3 font-mono text-[11.5px] overflow-auto max-h-[64vh]'

export function LogViewer({ lines, levelFilter }: LogViewerProps) {
  const visible = levelFilter
    ? lines.filter(l => l.toUpperCase().includes(levelFilter.toUpperCase()))
    : lines

  if (visible.length === 0) {
    return <div className={`${boxClass} text-mut`}>No log lines</div>
  }

  return (
    <div className={boxClass} style={{ lineHeight: 1.75 }}>
      {visible.map((line, i) => (
        <div key={i} className={lineColor(line)}>
          {line}
        </div>
      ))}
    </div>
  )
}
