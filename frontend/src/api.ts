import type { VesselState, DarkEvent, GhostFrame } from './types'

const BASE = '/v1'

export async function fetchVessels(): Promise<VesselState[]> {
  const res = await fetch(`${BASE}/vessels`)
  if (!res.ok) throw new Error(`vessels: ${res.status}`)
  const data = await res.json()
  return data.vessels ?? data
}

export async function fetchDarkZones(): Promise<{ dark_events: DarkEvent[] }> {
  const res = await fetch(`${BASE}/dark-zones`)
  if (!res.ok) throw new Error(`dark-zones: ${res.status}`)
  return res.json()
}

export async function fetchVesselDark(mmsi: number): Promise<unknown> {
  const res = await fetch(`${BASE}/vessels/${mmsi}/dark`)
  if (!res.ok) throw new Error(`vessel dark: ${res.status}`)
  return res.json()
}

export interface ViewportBbox {
  min_lat: number
  min_lon: number
  max_lat: number
  max_lon: number
}

export class StreamClient {
  private ws: WebSocket | null = null
  private onFrame: (frame: GhostFrame) => void

  constructor(onFrame: (frame: GhostFrame) => void) {
    this.onFrame = onFrame
  }

  connect() {
    const ws = new WebSocket(`ws://${location.host}/v1/stream`)
    ws.onmessage = (e) => {
      try { this.onFrame(JSON.parse(e.data)) } catch { /* malformed */ }
    }
    ws.onerror = (e) => console.error('stream error', e)
    ws.onclose = () => {
      // Reconnect after 3s on unexpected close.
      setTimeout(() => this.connect(), 3000)
    }
    this.ws = ws
  }

  sendViewport(bbox: ViewportBbox) {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify({ type: 'viewport', ...bbox }))
    }
  }
}
