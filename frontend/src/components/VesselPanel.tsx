import type { VesselState } from '../types'

interface Props {
  open:    boolean
  vessel:  VesselState | null
  report:  Record<string, unknown> | null
  onClose: () => void
}

export default function VesselPanel({ open, vessel, report, onClose }: Props) {
  const title = vessel
    ? (vessel.vessel_name ? `${vessel.vessel_name} · ${vessel.mmsi}` : `MMSI ${vessel.mmsi}`)
    : (report ? titleFromReport(report) : '—')

  return (
    <div id="vessel-panel" className={open ? '' : 'panel-hidden'}>
      <div className="panel-header">
        <div className="panel-title-row">
          <span className="panel-sigil">▸</span>
          <span id="panel-title">{title}</span>
        </div>
        <button id="panel-close" onClick={onClose}>[×]</button>
      </div>
      <div id="panel-body">
        {vessel  && <PositionSection vessel={vessel} />}
        {report  && <DarkSection report={report} />}
      </div>
    </div>
  )
}

function titleFromReport(report: Record<string, unknown>): string {
  const mmsi = report.mmsi as number | undefined
  return mmsi ? `MMSI ${mmsi}` : '—'
}

function PositionSection({ vessel }: { vessel: VesselState }) {
  const ageBadge = formatAge(vessel.last_seen_ns)
  const { color, label } = ageStyle(vessel.last_seen_ns)

  return (
    <section>
      <div className="section-title">Position</div>
      <div className="field">
        <span>Status</span>
        <span style={{ color }}>{label}</span>
      </div>
      <Field label="Last seen" value={ageBadge} />
      <Field label="Lat / Lon"
        value={`${vessel.lat.toFixed(4)}° / ${vessel.lon.toFixed(4)}°`} />
      <Field label="SOG" value={`${vessel.sog.toFixed(1)} kts`} />
      <Field label="COG" value={`${vessel.cog.toFixed(1)}°`} />
      {vessel.heading != null && (
        <Field label="Heading" value={`${vessel.heading.toFixed(0)}°`} />
      )}
    </section>
  )
}

function DarkSection({ report }: { report: Record<string, unknown> }) {
  const confidence = report.dark_confidence as string | undefined
  const local      = report.local_event as Record<string, unknown> | null
  const gfwGaps    = (report.gfw_gap_events as unknown[]) ?? []
  const sar        = report.sar_detections as Record<string, unknown> | undefined
  const viirs      = report.viirs_detections as Record<string, unknown> | undefined

  return (
    <>
      <section>
        <div className="section-title">Dark Activity</div>
        {confidence && (
          <div className={`field confidence confidence-${confidence}`}>
            <span>Confidence</span>
            <span>{confidence.toUpperCase()}</span>
          </div>
        )}
        {local ? (
          <>
            <Field label="Gap start" value={formatNs(local.gap_start_ns as number)} />
            {local.gap_end_ns
              ? <Field label="Gap end" value={formatNs(local.gap_end_ns as number)} />
              : <div className="field ongoing"><span>Status</span><span>ONGOING</span></div>
            }
          </>
        ) : (
          <Field label="Status" value="Clear" muted />
        )}
      </section>

      <section>
        <div className="section-title">GFW Gap Events (7d)</div>
        {gfwGaps.length === 0 ? (
          <Field label="None detected" value="—" muted />
        ) : gfwGaps.map((g, i) => {
          const gap = g as Record<string, unknown>
          return (
            <div key={i} className="field">
              <span>{gap.start as string ?? '—'}</span>
              <span>{gap.duration_hours ? `${(gap.duration_hours as number).toFixed(1)}h` : '—'}</span>
            </div>
          )
        })}
      </section>

      <section>
        <div className="section-title">Satellite Detections</div>
        <div className="field">
          <span>SAR</span>
          <span>
            {Number(sar?.total ?? 0)}{' '}
            <span style={{ color: 'var(--text-muted)' }}>({Number(sar?.unmatched_dark ?? 0)} unmatched)</span>
          </span>
        </div>
        <div className="field">
          <span>VIIRS</span>
          <span>
            {Number(viirs?.total ?? 0)}{' '}
            <span style={{ color: 'var(--text-muted)' }}>({Number(viirs?.unmatched_dark ?? 0)} unmatched)</span>
          </span>
        </div>
      </section>
    </>
  )
}

function Field({ label, value, muted = false }: { label: string; value: string | undefined; muted?: boolean }) {
  return (
    <div className={`field${muted ? ' muted' : ''}`}>
      <span>{label}</span>
      <span>{value ?? '—'}</span>
    </div>
  )
}

function formatNs(ns: number): string {
  return new Date(ns / 1_000_000).toISOString().replace('T', ' ').slice(0, 19) + ' UTC'
}

function formatAge(lastSeenNs: number): string {
  const ms = Date.now() - lastSeenNs / 1_000_000
  if (ms < 60_000)          return 'just now'
  if (ms < 3_600_000)       return `${Math.floor(ms / 60_000)}m ago`
  if (ms < 86_400_000)      return `${Math.floor(ms / 3_600_000)}h ${Math.floor((ms % 3_600_000) / 60_000)}m ago`
  return `${Math.floor(ms / 86_400_000)}d ago`
}

function ageStyle(lastSeenNs: number): { color: string; label: string } {
  const ms = Date.now() - lastSeenNs / 1_000_000
  if (ms < 5 * 60_000)   return { color: '#00ff9d', label: 'LIVE' }
  if (ms < 30 * 60_000)  return { color: '#f5a623', label: 'RECENT' }
  if (ms < 120 * 60_000) return { color: '#ff6b35', label: 'STALE' }
  return { color: 'rgba(255,255,255,0.3)', label: 'HISTORICAL' }
}
