import 'maplibre-gl/dist/maplibre-gl.css'
import './style.css'
import { initMap, buildLayers, ZOOM_HEATMAP, ZOOM_GRID } from './map'
import { StreamClient, fetchVessels, fetchDarkZones, fetchVesselDark } from './api'
import { initHud, initPanel, showVesselReport, updateHud } from './ui'
import type { VesselState, GhostFrame, DarkEvent } from './types'
import { FrameType } from './types'

// ── Init ──────────────────────────────────────────────────────────────────────

const container = document.getElementById('app')!
const { overlay, getZoom, onViewportChange, onZoomEnd } = initMap(container, onVesselClick)

initHud()
initPanel()

// ── State ─────────────────────────────────────────────────────────────────────

const vessels  = new Map<number, VesselState>()
let vesselArray: VesselState[] = []
let darkEvents: DarkEvent[]    = []

// Separate flags so we only pay for what changed.
let dataDirty = false   // vessel map or dark events changed → rebuild layers
let hudDirty  = false   // vessel count changed → update HUD

// LOD tier: 0=heatmap, 1=grid, 2=scatter. -1 = unset (forces first render).
let currentLod = -1

function getLod(zoom: number): number {
  if (zoom < ZOOM_HEATMAP) return 0
  if (zoom < ZOOM_GRID)    return 1
  return 2
}

// ── rAF render loop ───────────────────────────────────────────────────────────
// MapboxOverlay syncs its viewport to MapLibre automatically — we only call
// setProps when data or layer configuration actually changes.

function renderLoop() {
  if (dataDirty) {
    dataDirty    = false
    hudDirty     = false
    vesselArray  = Array.from(vessels.values())
    currentLod   = getLod(getZoom())
    overlay.setProps({ layers: buildLayers(vesselArray, darkEvents, getZoom()) })
    updateHud(vessels.size, vesselArray.filter(v => v.is_dark).length)
  } else if (hudDirty) {
    hudDirty = false
    updateHud(vessels.size, vesselArray.filter(v => v.is_dark).length)
  }
  requestAnimationFrame(renderLoop)
}
requestAnimationFrame(renderLoop)

// ── WebSocket stream ──────────────────────────────────────────────────────────

const stream = new StreamClient((frame: GhostFrame) => {
  vessels.set(frame.mmsi, {
    mmsi:         frame.mmsi,
    vessel_name:  frame.vessel_name,
    vessel_type:  frame.vessel_type,
    lat:          frame.lat,
    lon:          frame.lon,
    sog:          frame.sog,
    cog:          frame.cog,
    heading:      frame.heading,
    last_seen_ns: frame.timestamp_utc_ns,
    is_dark:      frame.frame_type.type === FrameType.GapMarker,
    confidence:   frame.confidence,
  })
  dataDirty = true
  hudDirty  = true
})
stream.connect()

// ── Viewport → WebSocket filter (debounced 300ms) ─────────────────────────────

let vpTimer: ReturnType<typeof setTimeout>
onViewportChange(bbox => {
  clearTimeout(vpTimer)
  vpTimer = setTimeout(() => stream.sendViewport(bbox), 300)
})

// Rebuild layers only when zoom crosses an LOD boundary (heatmap/grid/scatter).
// During the gesture itself MapboxOverlay handles the viewport — no rebuild needed.
onZoomEnd(() => {
  const lod = getLod(getZoom())
  if (lod !== currentLod) {
    currentLod = lod
    dataDirty  = true
  }
})

// ── REST: initial load + dark zone poll ───────────────────────────────────────

fetchVessels().then(list => {
  for (const v of list) vessels.set(v.mmsi, v)
  dataDirty = true
  hudDirty  = true
}).catch(console.error)

async function refreshDarkZones() {
  try {
    const { dark_events } = await fetchDarkZones()
    darkEvents = dark_events
    dataDirty  = true
  } catch { /* keep last state */ }
}

refreshDarkZones()
setInterval(refreshDarkZones, 30_000)

// ── Vessel click → detail panel ───────────────────────────────────────────────

async function onVesselClick(vessel: VesselState) {
  try {
    const report = await fetchVesselDark(vessel.mmsi) as Record<string, unknown>
    showVesselReport(report)
  } catch (e) {
    console.error('vessel report failed', e)
  }
}
