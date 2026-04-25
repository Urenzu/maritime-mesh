use anyhow::Result;
use schema::{FrameType, GhostFrame};
use std::time::{SystemTime, UNIX_EPOCH};

/// Parse raw NMEA AIVDM sentences (hardware AIS receiver or sim TCP feed).
pub fn sentence_to_frame(sentence: &str, frame_id: u64) -> Result<Option<GhostFrame>> {
    use crate::ais;

    let sentence = sentence.trim();
    if !sentence.starts_with("!AIVDM") && !sentence.starts_with("!AIVDO") {
        return Ok(None);
    }

    let parts: Vec<&str> = sentence.split(',').collect();
    if parts.len() < 7 { return Ok(None); }
    if parts[1].parse::<u8>().unwrap_or(1) > 1 { return Ok(None); }

    let payload   = parts[5];
    let fill_bits: u8 = parts[6].split('*').next().unwrap_or("0").parse().unwrap_or(0);

    let report = match ais::decode_position(payload, fill_bits) {
        Some(r) => r,
        None => return Ok(None),
    };

    if report.lat == 0.0 && report.lon == 0.0 { return Ok(None); }

    let now_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0);

    Ok(Some(GhostFrame {
        frame_id,
        mmsi:             report.mmsi,
        vessel_name:      None,
        vessel_type:      0,
        lat:              report.lat,
        lon:              report.lon,
        altitude:         0.0,
        position_acc:     report.position_acc,
        sog:              report.sog,
        cog:              report.cog,
        heading:          report.heading,
        rot:              report.rot,
        timestamp_utc_ns: now_ns,
        ingested_at_ns:   now_ns,
        frame_type:       FrameType::Real,
        confidence:       1.0,
        source_node_id:   0,
    }))
}
