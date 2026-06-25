import { ResponsiveContainer, LineChart, Line, XAxis, YAxis, Tooltip, CartesianGrid } from 'recharts'
import type { MetricsResp } from '../api/types'

interface MetricChartProps {
  title: string
  data: MetricsResp | undefined
  isLoading?: boolean
  error?: Error | null
  color?: string
  height?: number
}

export function MetricChart({ title, data, isLoading, error, color = '#4ade80', height = 180 }: MetricChartProps) {
  const points = data?.points ?? []
  const chartData = points.map(([ts, value]) => ({ ts, value }))

  return (
    <div className="bg-surface border border-line rounded-xl p-4" data-testid={`chart-${title}`}>
      <h3 className="text-sm font-semibold text-ink/70 mb-3">{title}</h3>
      {isLoading && <div className="flex items-center justify-center h-28 text-ink/40 text-xs">Loading…</div>}
      {error && <div className="flex items-center justify-center h-28 text-red-400 text-xs">Error: {error.message}</div>}
      {!isLoading && !error && chartData.length === 0 && (
        <div className="flex items-center justify-center h-28 text-ink/30 text-xs">No data</div>
      )}
      {!isLoading && !error && chartData.length > 0 && (
        <div style={{ height }}>
          <ResponsiveContainer width="100%" height="100%">
            <LineChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" stroke="#30363d" />
              <XAxis
                dataKey="ts"
                tickFormatter={(v: number) => new Date(v).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                stroke="#6e7681"
                tick={{ fontSize: 10, fill: '#6e7681' }}
              />
              <YAxis stroke="#6e7681" tick={{ fontSize: 10, fill: '#6e7681' }} width={50} />
              <Tooltip
                contentStyle={{ background: '#0d1117', border: '1px solid #30363d', borderRadius: 4, fontSize: 11 }}
                labelFormatter={(v) => new Date(Number(v)).toLocaleString()}
              />
              <Line
                type="monotone"
                dataKey="value"
                stroke={color}
                strokeWidth={1.5}
                dot={false}
                isAnimationActive={false}
              />
            </LineChart>
          </ResponsiveContainer>
        </div>
      )}
    </div>
  )
}
