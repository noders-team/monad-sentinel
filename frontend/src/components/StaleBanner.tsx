interface StaleBannerProps {
  isError: boolean
  hasData: boolean
}

export function StaleBanner({ isError, hasData }: StaleBannerProps) {
  if (!isError || !hasData) return null

  return (
    <div
      data-testid="stale-banner"
      className="flex items-center gap-2 px-4 py-2 rounded-lg border border-amber-500 bg-amber-500/10 text-amber-300 text-sm"
    >
      <span>stale — retrying</span>
    </div>
  )
}
