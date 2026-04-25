export enum FrameType {
  Real      = 'real',
  Synthetic = 'synthetic',
  GapMarker = 'gap_marker',
}

export interface GhostFrame {
  frame_id: number
  mmsi: number
  vessel_name?: string
  vessel_type: number
  lat: number
  lon: number
  sog: number       // knots
  cog: number       // degrees true
  heading: number | null  // null = AIS code 511 (not available)
  rot: number | null      // null = AIS code -128 (not available)
  timestamp_utc_ns: number
  ingested_at_ns: number
  frame_type: { type: FrameType }
  confidence: number
  source_node_id: number
}

export interface VesselState {
  mmsi: number
  vessel_name?: string
  vessel_type: number
  lat: number
  lon: number
  sog: number
  cog: number
  heading: number | null
  last_seen_ns: number
  is_dark: boolean
  confidence: number
}

export interface DarkEvent {
  dark_event_id: number
  mmsi: number
  gap_start_ns: number
  gap_end_ns?: number
  last_lat: number
  last_lon: number
  last_sog: number
  last_cog: number
  is_ongoing: boolean
}
