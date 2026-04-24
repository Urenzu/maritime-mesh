// ── Stats HUD ─────────────────────────────────────────────────────────────────

let hudEl: HTMLElement

export function initHud(): void {
  hudEl = document.createElement('div')
  hudEl.id = 'hud'
  hudEl.innerHTML = `
    <div class="hud-row"><span class="hud-label">VESSELS</span><span id="hud-count">—</span></div>
    <div class="hud-row"><span class="hud-label">DARK</span><span id="hud-dark" class="hud-dark">—</span></div>
    <div class="hud-row"><span class="hud-label">UPDATED</span><span id="hud-age">—</span></div>
  `
  document.body.appendChild(hudEl)
}

export function updateHud(vesselCount: number, darkCount: number) {
  const countEl = document.getElementById('hud-count')
  const darkEl  = document.getElementById('hud-dark')
  const ageEl   = document.getElementById('hud-age')
  if (countEl) countEl.textContent = vesselCount.toLocaleString()
  if (darkEl)  darkEl.textContent  = darkCount.toString()
  if (ageEl)   ageEl.textContent   = new Date().toISOString().slice(11, 19) + ' UTC'
}

// ── Vessel detail panel ───────────────────────────────────────────────────────

let panelEl: HTMLElement

export function initPanel(): void {
  panelEl = document.createElement('div')
  panelEl.id = 'vessel-panel'
  panelEl.classList.add('panel-hidden')
  panelEl.innerHTML = `
    <div class="panel-header">
      <span id="panel-title">—</span>
      <button id="panel-close">✕</button>
    </div>
    <div id="panel-body"></div>
  `
  document.body.appendChild(panelEl)
  document.getElementById('panel-close')!.onclick = closePanel
}

export function closePanel() {
  panelEl.classList.add('panel-hidden')
}

export function showVesselReport(report: Record<string, unknown>) {
  const vessel = report.vessel as Record<string, unknown> | null
  const mmsi   = report.mmsi as number

  const title = vessel?.ship_name
    ? `${vessel.ship_name} · ${mmsi}`
    : `MMSI ${mmsi}`

  document.getElementById('panel-title')!.textContent = title

  const confidence = report.dark_confidence as string
  const local      = report.local_event as Record<string, unknown> | null
  const gfwGaps    = (report.gfw_gap_events as unknown[]) ?? []
  const sar        = report.sar_detections as Record<string, unknown>
  const viirs      = report.viirs_detections as Record<string, unknown>

  const body = document.getElementById('panel-body')!
  body.innerHTML = `
    ${vessel ? `
    <section>
      <div class="field"><span>Flag</span><span>${vessel.flag ?? '—'}</span></div>
      <div class="field"><span>Type</span><span>${vessel.vessel_type ?? '—'}</span></div>
      <div class="field"><span>Length</span><span>${vessel.length_m ? vessel.length_m + ' m' : '—'}</span></div>
    </section>` : ''}

    <section>
      <div class="section-title">DARK ACTIVITY</div>
      <div class="field confidence confidence-${confidence}">
        <span>Confidence</span><span>${confidence.toUpperCase()}</span>
      </div>
      ${local ? `
      <div class="field"><span>Gap start</span><span>${formatNs(local.gap_start_ns as number)}</span></div>
      ${local.gap_end_ns ? `<div class="field"><span>Gap end</span><span>${formatNs(local.gap_end_ns as number)}</span></div>` : '<div class="field ongoing"><span>Status</span><span>ONGOING</span></div>'}
      ` : '<div class="field"><span>Status</span><span>Clear</span></div>'}
    </section>

    <section>
      <div class="section-title">GFW GAP EVENTS (7d)</div>
      ${gfwGaps.length === 0
        ? '<div class="field muted"><span>None detected</span></div>'
        : gfwGaps.map((g: unknown) => {
            const gap = g as Record<string, unknown>
            return `<div class="field">
              <span>${gap.start as string ?? '—'}</span>
              <span>${gap.duration_hours ? (gap.duration_hours as number).toFixed(1) + 'h' : '—'}</span>
            </div>`
          }).join('')
      }
    </section>

    <section>
      <div class="section-title">SATELLITE DETECTIONS</div>
      <div class="field"><span>SAR hits</span><span>${sar?.total ?? 0} (${sar?.unmatched_dark ?? 0} unmatched)</span></div>
      <div class="field"><span>VIIRS hits</span><span>${viirs?.total ?? 0} (${viirs?.unmatched_dark ?? 0} unmatched)</span></div>
    </section>
  `

  panelEl.classList.remove('panel-hidden')
}

function formatNs(ns: number): string {
  return new Date(ns / 1_000_000).toISOString().replace('T', ' ').slice(0, 19) + ' UTC'
}
