interface StaleBannerProps {
  isError: boolean
  hasData: boolean
}

export function StaleBanner({ isError, hasData }: StaleBannerProps) {
  if (!isError || !hasData) return null

  return (
    <div
      data-testid="stale-banner"
      className="flex items-center gap-2 px-[14px] py-2 rounded-[10px] border border-warn text-warn text-[12.5px]"
      style={{ background: 'rgba(200,130,26,0.08)' }}
    >
      <span>Live data is stale — retrying.</span>
    </div>
  )
}
