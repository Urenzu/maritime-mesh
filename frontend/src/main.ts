import 'maplibre-gl/dist/maplibre-gl.css'
import './style.css'

if ('serviceWorker' in navigator) {
  navigator.serviceWorker.register('/sw.js').catch(() => {/* non-fatal */})
}

import { initMap, buildLayers, ZOOM_HEATMAP, ZOOM_GRID } from './map'
import { fetchVessels, fetchDarkZones, fetchVesselDark } from './api'
import { initHud, initPanel, showVesselReport, updateHud } from './ui'
import type { DarkEvent } from './types'
import type { VesselSnapshot } from './worker-types'

// ── Init ──────────────────────────────────────────────────────────────────────

const container = document.getElementById('app')!
const { overlay, getZoom, onViewportChange, onZoomEnd } = initMap(container)

initHud()
initPanel()

// ── Vessel state (typed arrays owned by main thread, filled by worker) ────────

let positions   = new Float32Array(0)
let colors      = new Uint8Array(0)
let mmsis       = new Int32Array(0)
let vesselCount = 0
let darkCount   = 0
let darkEvents: DarkEvent[] = []

let dataDirty = false
let hudDirty  = false
let currentLod = -1

function getLod(zoom: number): number {
  if (zoom < ZOOM_HEATMAP) return 0
  if (zoom < ZOOM_GRID)    return 1
  return 2
}

// ── rAF render loop ───────────────────────────────────────────────────────────

function renderLoop() {
  if (dataDirty) {
    dataDirty  = false
    hudDirty   = false
    currentLod = getLod(getZoom())
    overlay.setProps({ layers: buildLayers(vesselCount, positions, colors, mmsis, darkEvents, getZoom(), onVesselClick) })
    updateHud(vesselCount, darkCount)
  } else if (hudDirty) {
    hudDirty = false
    updateHud(vesselCount, darkCount)
  }
  requestAnimationFrame(renderLoop)
}
requestAnimationFrame(renderLoop)

// ── Web Worker — vessel data pipeline ────────────────────────────────────────
// Worker owns WebSocket + vessel Map. Posts typed array snapshots every 50ms.
// Main thread never touches raw frames or JSON parsing.

const worker = new Worker(new URL('./vessel-worker.ts', import.meta.url), { type: 'module' })

worker.onmessage = ({ data }: MessageEvent<VesselSnapshot>) => {
  positions   = data.positions
  colors      = data.colors
  mmsis       = data.mmsis
  vesselCount = data.count
  darkCount   = data.darkCount
  dataDirty   = true
  hudDirty    = true
}

worker.postMessage({
  type:  'connect',
  wsUrl: `${location.protocol === 'https:' ? 'wss' : 'ws'}://${location.host}/v1/stream`,
})

// ── REST: initial vessel seed + dark zone poll ────────────────────────────────

fetchVessels().then(list => {
  worker.postMessage({ type: 'seed', vessels: list })
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

// ── Viewport → Worker → WebSocket (debounced 300ms) ───────────────────────────

let vpTimer: ReturnType<typeof setTimeout>
onViewportChange(bbox => {
  clearTimeout(vpTimer)
  vpTimer = setTimeout(() => worker.postMessage({ type: 'set-viewport', bbox }), 300)
})

onZoomEnd(() => {
  const lod = getLod(getZoom())
  if (lod !== currentLod) {
    currentLod = lod
    dataDirty  = true
  }
})

// ── Vessel click ──────────────────────────────────────────────────────────────

async function onVesselClick(mmsi: number) {
  try {
    const report = await fetchVesselDark(mmsi) as Record<string, unknown>
    showVesselReport(report)
  } catch (e) {
    console.error('vessel report failed', e)
  }
}
