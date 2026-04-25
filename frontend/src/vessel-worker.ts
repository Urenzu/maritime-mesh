// Runs in a Web Worker.
// Owns the WebSocket connection and all vessel state.
// Every 50ms it packs state into typed arrays and transfers them to the main
// thread — zero copy via Transferable, main thread is never blocked by parsing.

import { FrameType } from './types'
import type { GhostFrame, VesselState } from './types'
import type { WorkerInbound, VesselSnapshot } from './worker-types'
import type { ViewportBbox } from './api'

// Age-based colors: how fresh the vessel's last AIS ping is.
const COLOR_LIVE:    readonly [number, number, number, number] = [0,   255, 157, 230]  // <5 min  — cyan-green
const COLOR_RECENT:  readonly [number, number, number, number] = [245, 166, 35,  220]  // 5–30 min — amber
const COLOR_STALE:   readonly [number, number, number, number] = [255, 107, 53,  200]  // 30min–2h — orange
const COLOR_OLD:     readonly [number, number, number, number] = [180, 60,  60,  100]  // >2h      — dim red
const COLOR_DARK:    readonly [number, number, number, number] = [255, 60,  60,  255]  // dark zone

const MS_5MIN  =   5 * 60 * 1_000
const MS_30MIN =  30 * 60 * 1_000
const MS_2H    = 120 * 60 * 1_000

function vesselColor(v: VesselState): readonly [number, number, number, number] {
  if (v.is_dark) return COLOR_DARK
  const ageMs = Date.now() - v.last_seen_ns / 1_000_000
  if (ageMs < MS_5MIN)  return COLOR_LIVE
  if (ageMs < MS_30MIN) return COLOR_RECENT
  if (ageMs < MS_2H)    return COLOR_STALE
  return COLOR_OLD
}

const vessels = new Map<number, VesselState>()
let ws:      WebSocket | null = null
let wsUrl    = ''
let dirty    = false   // true when any vessel changed since last snapshot
let retryMs  = 3_000   // exponential backoff, capped at 30s

function connect() {
  ws = new WebSocket(wsUrl)

  ws.onmessage = ({ data }: MessageEvent<string>) => {
    try {
      const frame = JSON.parse(data) as GhostFrame
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
      dirty = true
    } catch { /* ignore malformed frame */ }
  }

  ws.onopen  = () => { retryMs = 3_000 }
  ws.onclose = () => {
    retryMs = Math.min(retryMs * 2, 30_000)
    setTimeout(connect, retryMs)
  }
}

function sendViewport(bbox: ViewportBbox) {
  if (ws?.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify({ type: 'viewport', ...bbox }))
  }
}

function postSnapshot() {
  if (!dirty || vessels.size === 0) return
  dirty = false
  const count = vessels.size

  const positions = new Float32Array(count * 2)
  const colors    = new Uint8Array(count * 4)
  const mmsis     = new Int32Array(count)

  let i = 0
  let darkCount = 0

  for (const v of vessels.values()) {
    positions[i * 2]     = v.lon
    positions[i * 2 + 1] = v.lat

    const c = vesselColor(v)
    colors[i * 4]     = c[0]
    colors[i * 4 + 1] = c[1]
    colors[i * 4 + 2] = c[2]
    colors[i * 4 + 3] = c[3]

    mmsis[i] = v.mmsi
    if (v.is_dark) darkCount++
    i++
  }

  const snap: VesselSnapshot = { type: 'snapshot', count, darkCount, positions, colors, mmsis }
  // TypeScript types `self` as Window in module workers; cast to reach the
  // DedicatedWorkerGlobalScope overload that accepts a Transferable[] array.
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  ;(self as any).postMessage(snap, [positions.buffer, colors.buffer, mmsis.buffer])
}

self.onmessage = ({ data }: MessageEvent<WorkerInbound>) => {
  switch (data.type) {
    case 'connect':
      wsUrl = data.wsUrl
      connect()
      break
    case 'seed':
      for (const v of data.vessels) vessels.set(v.mmsi, v)
      dirty = true
      break
    case 'set-viewport':
      sendViewport(data.bbox)
      break
  }
}

setInterval(postSnapshot, 50)
