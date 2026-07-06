import { ResponsiveContainer, AreaChart, Area, Tooltip } from 'recharts'
import type { MetricsResp } from '../api/types'

interface SparklineProps {
  data: MetricsResp | undefined
  height?: number
  color?: string
}

export function Sparkline({ data, height = 64, color = '#0E9F6E' }: SparklineProps) {
  const points = data?.points ?? []

  if (points.length === 0) {
    return (
      <div data-testid="sparkline-empty" className="flex items-center justify-center text-mut text-[11px]" style={{ height }}>
        No data
      </div>
    )
  }

  const chartData = points.map(([ts, value]) => ({ ts, value }))
  const gradientId = 'spark-fill'

  return (
    <div data-testid="sparkline" style={{ height }}>
      <ResponsiveContainer width="100%" height="100%">
        <AreaChart data={chartData} margin={{ top: 4, right: 0, bottom: 0, left: 0 }}>
          <defs>
            <linearGradient id={gradientId} x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor={color} stopOpacity={0.14} />
              <stop offset="100%" stopColor={color} stopOpacity={0} />
            </linearGradient>
          </defs>
          <Area
            type="monotone"
            dataKey="value"
            stroke={color}
            strokeWidth={1.4}
            fill={`url(#${gradientId})`}
            dot={false}
            isAnimationActive={false}
          />
          <Tooltip
            contentStyle={{ background: '#FFFFFF', border: '1px solid #E5E1F2', borderRadius: 6, fontSize: 11, color: '#1C1533' }}
            labelStyle={{ color: '#6E6890' }}
            labelFormatter={(v) => {
              const ts = typeof v === 'number' ? v : Number(v)
              return new Date(ts * 1000).toLocaleTimeString()
            }}
          />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  )
}
