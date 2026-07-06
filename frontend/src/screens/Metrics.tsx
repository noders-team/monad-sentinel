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
  { name: 'monad_total_uptime_us', label: 'Total Uptime (µs)', color: '#0E9F6E' },
  { name: 'monad_vote_rate', label: 'Vote Rate', color: '#836EF9' },
  { name: 'monad_block_height', label: 'Block Height', color: '#5A3FE8' },
]

function MetricSeries({ series, window }: { series: TrackedSeries; window: Window }) {
  const { data, isLoading, error } = useMetrics(series.name, window)
  const last = data?.points?.at(-1)?.[1]
  return (
    <MetricChart
      title={series.label}
      data={data}
      isLoading={isLoading}
      error={error instanceof Error ? error : null}
      color={series.color}
      currentValue={last !== undefined ? (Number.isInteger(last) ? last.toLocaleString() : last.toFixed(3)) : undefined}
    />
  )
}

export function Metrics() {
  const [window, setWindow] = useState<Window>('24h')

  return (
    <div className="px-5 pt-4 pb-8 space-y-3.5">
      <div className="flex items-center justify-between">
        <h1 className="text-[15px] font-semibold">Metrics</h1>
        <WindowPicker value={window} onChange={setWindow} />
      </div>
      <div className="grid gap-2.5 md:grid-cols-2 xl:grid-cols-3">
        {SERIES.map(series => (
          <MetricSeries key={series.name} series={series} window={window} />
        ))}
      </div>
    </div>
  )
}
