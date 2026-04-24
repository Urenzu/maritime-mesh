# GhostFrame Data Model

The GhostFrame is the canonical unit of data throughout the system.
Every piece of telemetry — real or synthetic — is a GhostFrame.

## Frame Types

| Type | Description |
|------|-------------|
| `REAL` | Direct from AIS/NMEA/Radar — cryptographically signed at ingestion |
| `SYNTHETIC` | Dead-reckoned / interpolated — flagged, includes confidence score + model used |
| `GAP_MARKER` | Sentinel written when a dark event begins; closes when vessel reappears |

## Core Fields

```
GhostFrame {
  // Identity
  frame_id:       u64         // monotonic, per-vessel sequence number
  mmsi:           u32         // Maritime Mobile Service Identity
  vessel_name:    string      // optional, from AIS Class A/B
  vessel_type:    u8          // ITU vessel type code

  // Position
  lat:            f64         // WGS84 decimal degrees
  lon:            f64         // WGS84 decimal degrees
  altitude:       f32         // meters ASL (0 for surface vessels)
  position_acc:   u8          // 0=low, 1=high (RAIM)

  // Kinematics
  sog:            f32         // speed over ground, knots
  cog:            f32         // course over ground, degrees true
  heading:        f32         // true heading, degrees (from compass)
  rot:            f32         // rate of turn, deg/min

  // Temporal
  timestamp_utc:  i64         // Unix nanoseconds (source timestamp)
  ingested_at:    i64         // Unix nanoseconds (local wall clock at WAL write)

  // Frame classification
  frame_type:     FrameType   // REAL | SYNTHETIC | GAP_MARKER
  confidence:     f32         // 1.0 for REAL; 0.0–1.0 for SYNTHETIC (decays with time)
  model:          string      // for SYNTHETIC: "kalman_cv" | "kalman_ca" | "great_circle"

  // Source tracing
  source_node:    u64         // node ID that ingested/synthesized this frame
  raw_sentence:   bytes       // original NMEA sentence (REAL only, nullable)

  // Integrity
  prev_hash:      bytes[32]   // SHA-256 of previous frame (chain integrity)
  signature:      bytes[64]   // Ed25519 signature over all fields above (ingestion node key)
}
```

## Dark Activity Detection Fields (on GAP_MARKER frames)

```
GapMarker extends GhostFrame {
  gap_start:          i64     // Unix ns when last REAL frame was received
  gap_end:            i64     // Unix ns when vessel reappeared (0 if ongoing)
  last_known_pos:     LatLon
  last_known_sog:     f32
  last_known_cog:     f32
  coverage_area_id:   u32     // which AIS receiver coverage zone this occurred in
  dark_event_id:      u64     // groups all SYNTHETIC frames for this gap
}
```

## Key Design Decisions

- **Immutable after write.** GhostFrames are never mutated; corrections are new frames.
- **Chain integrity.** `prev_hash` forms a per-vessel hash chain. Breaks are detectable.
- **SYNTHETIC frames are first-class.** The visualization layer renders them identically to REAL frames but with a visual confidence gradient. Analytics treat them as labeled training data.
- **`ingested_at` vs `timestamp_utc`.** The difference is the "ingestion latency" — useful for detecting clock drift on remote nodes or replayed/spoofed data.

## Flatbuffer Schema Location

`schemas/ghost_frame.fbs` — all Flatbuffer definitions live here.
The `GhostFrame` table is the root type for WAL entries and WebGPU vertex data.
