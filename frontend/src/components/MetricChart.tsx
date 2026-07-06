import { ResponsiveContainer, LineChart, Line, XAxis, YAxis, Tooltip, CartesianGrid } from 'recharts'
import type { MetricsResp } from '../api/types'

interface MetricChartProps {
  title: string
  data: MetricsResp | undefined
  isLoading?: boolean
  error?: Error | null
  color?: string
  height?: number
  /** Optional current-value readout shown top-right (already formatted). */
  currentValue?: string
  /** Optional legend/units line shown centered in the footer. */
  legend?: string
}

const GRID = '#DFDAEE'
const AXIS = '#6E6890'

function fmtNum(n: number): string {
  return Number.isInteger(n) ? n.toLocaleString() : n.toFixed(3)
}

export function MetricChart({
  title,
  data,
  isLoading,
  error,
  color = '#0E9F6E',
  height = 110,
  currentValue,
  legend,
}: MetricChartProps) {
  const points = data?.points ?? []
  const chartData = points.map(([ts, value]) => ({ ts, value }))
  const values = chartData.map(d => d.value)
  const min = values.length ? Math.min(...values) : undefined
  const max = values.length ? Math.max(...values) : undefined

  return (
    <div className="bg-panel border border-line rounded-[10px] p-[14px_16px]" data-testid={`chart-${title}`}>
      <div className="flex items-baseline justify-between mb-2">
        <h3 className="text-[12px] font-semibold text-ink">{title}</h3>
        {currentValue && <span className="font-mono text-[12px] text-acc-ink">{currentValue}</span>}
      </div>
      {isLoading && <div className="flex items-center justify-center h-28 text-mut text-[11px]">Loading…</div>}
      {error && <div className="flex items-center justify-center h-28 text-dgr text-[11px]">Error: {error.message}</div>}
      {!isLoading && !error && chartData.length === 0 && (
        <div className="flex items-center justify-center h-28 text-mut text-[11px]">No data</div>
      )}
      {!isLoading && !error && chartData.length > 0 && (
        <>
          <div style={{ height }}>
            <ResponsiveContainer width="100%" height="100%">
              <LineChart data={chartData} margin={{ top: 4, right: 4, bottom: 0, left: 0 }}>
                <CartesianGrid strokeDasharray="4 4" stroke={GRID} vertical={false} />
                <XAxis
                  dataKey="ts"
                  tickFormatter={(v: number) => new Date(v).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                  stroke={GRID}
                  tick={{ fontSize: 10, fill: AXIS }}
                  minTickGap={40}
                />
                <YAxis stroke={GRID} tick={{ fontSize: 10, fill: AXIS }} width={44} />
                <Tooltip
                  contentStyle={{ background: '#FFFFFF', border: '1px solid #E5E1F2', borderRadius: 6, fontSize: 11, color: '#1C1533' }}
                  labelStyle={{ color: '#6E6890' }}
                  labelFormatter={(v) => new Date(Number(v)).toLocaleString()}
                />
                <Line type="monotone" dataKey="value" stroke={color} strokeWidth={1.5} dot={false} isAnimationActive={false} />
              </LineChart>
            </ResponsiveContainer>
          </div>
          <div className="flex justify-between mt-1.5 font-mono text-[10.5px] text-mut">
            <span>min {min !== undefined ? fmtNum(min) : '—'}</span>
            {legend && <span>{legend}</span>}
            <span>max {max !== undefined ? fmtNum(max) : '—'}</span>
          </div>
        </>
      )}
    </div>
  )
}
