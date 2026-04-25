import type { ViewportBbox } from './api'
import type { VesselState } from './types'

// Main → Worker
export type WorkerInbound =
  | { type: 'connect'; wsUrl: string }
  | { type: 'seed';    vessels: VesselState[] }
  | { type: 'set-viewport'; bbox: ViewportBbox }

// Worker → Main
export interface VesselSnapshot {
  type:      'snapshot'
  count:     number
  darkCount: number
  positions: Float32Array<ArrayBuffer>  // [lon0, lat0, lon1, lat1, ...]
  colors:    Uint8Array<ArrayBuffer>    // [r, g, b, a, ...]  0-255
  mmsis:     Int32Array<ArrayBuffer>    // parallel — index → mmsi for click resolution
}

export type WorkerOutbound = VesselSnapshot
