import { useState } from 'react'
import { WindowPicker } from '../components/WindowPicker'
import { MetricChart } from '../components/MetricChart'
import { useMetrics } from '../api/hooks'

type Window = '1h' | '24h' | '7d'

interface TrackedSeries {
  name: string
  label: string
  color: string
}

const SERIES: TrackedSeries[] = [
  { name: 'monad_total_uptime_us', label: 'Total Uptime (µs)', color: '#4ade80' },
  { name: 'monad_vote_rate', label: 'Vote Rate', color: '#60a5fa' },
  { name: 'monad_block_height', label: 'Block Height', color: '#f59e0b' },
]

function MetricSeries({ series, window }: { series: TrackedSeries; window: Window }) {
  const { data, isLoading, error } = useMetrics(series.name, window)
  return (
    <MetricChart
      title={series.label}
      data={data}
      isLoading={isLoading}
      error={error instanceof Error ? error : null}
      color={series.color}
    />
  )
}

export function Metrics() {
  const [window, setWindow] = useState<Window>('24h')

  return (
    <div className="p-6 space-y-6 text-ink">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">Metrics</h1>
        <WindowPicker value={window} onChange={setWindow} />
      </div>
      <div className="grid gap-4 sm:grid-cols-1 lg:grid-cols-2 xl:grid-cols-3">
        {SERIES.map(series => (
          <MetricSeries key={series.name} series={series} window={window} />
        ))}
      </div>
    </div>
  )
}
