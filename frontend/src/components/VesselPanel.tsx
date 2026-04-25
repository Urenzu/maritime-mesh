interface Props {
  open:    boolean
  report:  Record<string, unknown> | null
  onClose: () => void
}

export default function VesselPanel({ open, report, onClose }: Props) {
  return (
    <div id="vessel-panel" className={open ? '' : 'panel-hidden'}>
      <div className="panel-header">
        <div className="panel-title-row">
          <span className="panel-sigil">▸</span>
          <span id="panel-title">{report ? titleFor(report) : '—'}</span>
        </div>
        <button id="panel-close" onClick={onClose}>[×]</button>
      </div>
      <div id="panel-body">
        {report && <ReportBody report={report} />}
      </div>
    </div>
  )
}

function titleFor(report: Record<string, unknown>): string {
  const vessel = report.vessel as Record<string, unknown> | null
  const mmsi   = report.mmsi as number
  return vessel?.ship_name ? `${vessel.ship_name} · ${mmsi}` : `MMSI ${mmsi}`
}

function ReportBody({ report }: { report: Record<string, unknown> }) {
  const vessel     = report.vessel as Record<string, unknown> | null
  const confidence = report.dark_confidence as string
  const local      = report.local_event as Record<string, unknown> | null
  const gfwGaps    = (report.gfw_gap_events as unknown[]) ?? []
  const sar        = report.sar_detections as Record<string, unknown>
  const viirs      = report.viirs_detections as Record<string, unknown>

  return (
    <>
      {vessel && (
        <section>
          <Field label="Flag"   value={vessel.flag as string}   />
          <Field label="Type"   value={vessel.vessel_type as string} />
          <Field label="Length" value={vessel.length_m ? `${vessel.length_m} m` : undefined} />
        </section>
      )}

      <section>
        <div className="section-title">Dark Activity</div>
        <div className={`field confidence confidence-${confidence}`}>
          <span>Confidence</span>
          <span>{String(confidence).toUpperCase()}</span>
        </div>
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

function Field({
  label,
  value,
  muted = false,
}: {
  label: string
  value: string | undefined
  muted?: boolean
}) {
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
