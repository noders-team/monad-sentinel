import { ResponsiveContainer, LineChart, Line, Tooltip } from 'recharts'
import type { MetricsResp } from '../api/types'

interface SparklineProps {
  data: MetricsResp | undefined
  height?: number
  color?: string
}

export function Sparkline({ data, height = 48, color = '#4ade80' }: SparklineProps) {
  const points = data?.points ?? []

  if (points.length === 0) {
    return (
      <div data-testid="sparkline-empty" className="flex items-center justify-center h-12 text-ink/30 text-xs">
        No data
      </div>
    )
  }

  const chartData = points.map(([ts, value]) => ({ ts, value }))

  return (
    <div data-testid="sparkline" style={{ height }}>
      <ResponsiveContainer width="100%" height="100%">
        <LineChart data={chartData}>
          <Line
            type="monotone"
            dataKey="value"
            stroke={color}
            strokeWidth={1.5}
            dot={false}
            isAnimationActive={false}
          />
          <Tooltip
            contentStyle={{ background: '#0d1117', border: '1px solid #30363d', borderRadius: 4, fontSize: 11 }}
            labelFormatter={(v) => {
              const ts = typeof v === 'number' ? v : Number(v)
              return new Date(ts * 1000).toLocaleTimeString()
            }}
          />
        </LineChart>
      </ResponsiveContainer>
    </div>
  )
}
