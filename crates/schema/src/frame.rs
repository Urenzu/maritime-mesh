use serde::{Deserialize, Serialize};

/// Every telemetry unit in the system — real or synthetic — is a GhostFrame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhostFrame {
    pub frame_id: u64,
    pub mmsi: u32,
    pub vessel_name: Option<String>,
    pub vessel_type: u8,

    // Position (WGS84)
    pub lat: f64,
    pub lon: f64,
    pub altitude: f32,
    pub position_acc: bool,

    // Kinematics
    pub sog: f32,       // speed over ground, knots
    pub cog: f32,       // course over ground, degrees true
    pub heading: f32,   // true heading, degrees
    pub rot: f32,       // rate of turn, deg/min

    // Temporal (Unix nanoseconds)
    pub timestamp_utc_ns: i64,
    pub ingested_at_ns: i64,

    // Classification
    pub frame_type: FrameType,
    pub confidence: f32,

    // Source
    pub source_node_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FrameType {
    Real,
    Synthetic { model: SynthModel },
    GapMarker(GapData),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SynthModel {
    KalmanConstantVelocity,
    KalmanConstantAcceleration,
    GreatCircle,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GapData {
    pub gap_start_ns: i64,
    pub gap_end_ns: Option<i64>,
    pub last_lat: f64,
    pub last_lon: f64,
    pub last_sog: f32,
    pub last_cog: f32,
    pub coverage_area_id: u32,
    pub dark_event_id: u64,
}

impl GhostFrame {
    pub fn is_dark(&self) -> bool {
        matches!(self.frame_type, FrameType::GapMarker(_))
    }

    pub fn is_real(&self) -> bool {
        self.frame_type == FrameType::Real
    }
}

/// Compact vessel state — latest known position for the API layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VesselState {
    pub mmsi: u32,
    pub vessel_name: Option<String>,
    pub vessel_type: u8,
    pub lat: f64,
    pub lon: f64,
    pub sog: f32,
    pub cog: f32,
    pub heading: f32,
    pub last_seen_ns: i64,
    pub is_dark: bool,
    pub confidence: f32,
}

impl From<&GhostFrame> for VesselState {
    fn from(f: &GhostFrame) -> Self {
        Self {
            mmsi: f.mmsi,
            vessel_name: f.vessel_name.clone(),
            vessel_type: f.vessel_type,
            lat: f.lat,
            lon: f.lon,
            sog: f.sog,
            cog: f.cog,
            heading: f.heading,
            last_seen_ns: f.timestamp_utc_ns,
            is_dark: f.is_dark(),
            confidence: f.confidence,
        }
    }
}
