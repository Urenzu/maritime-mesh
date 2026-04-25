// Runs in a Web Worker.
// Owns the WebSocket connection and all vessel state.
// Every 50ms it packs state into typed arrays and transfers them to the main
// thread — zero copy via Transferable, main thread is never blocked by parsing.

import { FrameType } from './types'
import type { GhostFrame, VesselState } from './types'
import type { WorkerInbound, VesselSnapshot } from './worker-types'
import type { ViewportBbox } from './api'

const COLOR_REAL:      readonly [number, number, number, number] = [0,   210, 255, 220]
const COLOR_SYNTHETIC: readonly [number, number, number, number] = [255, 180, 0,   220]
const COLOR_DARK:      readonly [number, number, number, number] = [255, 60,  60,  255]

const vessels = new Map<number, VesselState>()
let ws: WebSocket | null = null
let wsUrl = ''

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
    } catch { /* ignore malformed frame */ }
  }

  ws.onclose = () => setTimeout(connect, 3_000)
}

function sendViewport(bbox: ViewportBbox) {
  if (ws?.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify({ type: 'viewport', ...bbox }))
  }
}

function postSnapshot() {
  const count = vessels.size
  if (count === 0) return

  const positions = new Float32Array(count * 2)
  const colors    = new Uint8Array(count * 4)
  const mmsis     = new Int32Array(count)

  let i = 0
  let darkCount = 0

  for (const v of vessels.values()) {
    positions[i * 2]     = v.lon
    positions[i * 2 + 1] = v.lat

    const c = v.is_dark          ? COLOR_DARK
            : v.confidence < 1.0 ? COLOR_SYNTHETIC
            :                      COLOR_REAL
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
      break
    case 'set-viewport':
      sendViewport(data.bbox)
      break
  }
}

setInterval(postSnapshot, 50)
