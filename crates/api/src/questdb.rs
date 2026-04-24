use anyhow::Result;
use reqwest::Client;
use schema::{FrameType, GhostFrame, SynthModel, VesselState};
use serde::Deserialize;
use tracing::warn;

/// Read-only QuestDB client for the API layer.
/// Queries via the HTTP REST endpoint (port 9000).
pub struct QuestDbReader {
    base_url: String,
    client:   Client,
}

#[derive(Deserialize)]
struct QdbResponse {
    dataset: Vec<Vec<serde_json::Value>>,
}

impl QuestDbReader {
    pub fn new(base_url: String) -> Self {
        Self { base_url, client: Client::new() }
    }

    /// Most recent VesselState per MMSI seen in the last `lookback_hours`.
    /// Called once at startup to hydrate the in-memory VesselMap.
    pub async fn latest_vessel_states(&self, lookback_hours: u32) -> Result<Vec<VesselState>> {
        let sql = format!(
            "SELECT mmsi, vessel_name, vessel_type, lat, lon, sog, cog, heading, timestamp \
             FROM ghost_frames \
             WHERE timestamp > dateadd('h', -{lookback_hours}, now()) \
             AND frame_type != 'gap_marker' \
             LATEST ON timestamp PARTITION BY mmsi"
        );

        let rows = self.query_raw(&sql).await?;
        let mut states = Vec::with_capacity(rows.len());

        for row in rows {
            if row.len() < 9 { continue; }
            let mmsi = parse_u32(&row[0]).unwrap_or(0);
            if mmsi == 0 { continue; }

            states.push(VesselState {
                mmsi,
                vessel_name:  row[1].as_str().filter(|s| !s.is_empty() && *s != "unknown").map(|s| s.to_owned()),
                vessel_type:  parse_u8(&row[2]).unwrap_or(0),
                lat:          parse_f64(&row[3]).unwrap_or(0.0),
                lon:          parse_f64(&row[4]).unwrap_or(0.0),
                sog:          parse_f32(&row[5]).unwrap_or(0.0),
                cog:          parse_f32(&row[6]).unwrap_or(0.0),
                heading:      parse_f32(&row[7]).unwrap_or(0.0),
                last_seen_ns: parse_i64(&row[8]).unwrap_or(0),
                is_dark:      false,
                confidence:   1.0,
            });
        }

        Ok(states)
    }

    /// GhostFrames for a vessel in a time window, ordered oldest-first.
    pub async fn track(&self, mmsi: u32, from_ns: i64, to_ns: i64) -> Result<Vec<GhostFrame>> {
        let sql = format!(
            "SELECT frame_id, mmsi, vessel_name, vessel_type, lat, lon, \
             sog, cog, heading, rot, timestamp, frame_type, confidence, source_node \
             FROM ghost_frames \
             WHERE mmsi = {mmsi} \
             AND timestamp BETWEEN {from_ns} AND {to_ns} \
             ORDER BY timestamp ASC \
             LIMIT 5000"
        );

        let rows = self.query_raw(&sql).await?;
        let mut frames = Vec::with_capacity(rows.len());

        for row in rows {
            if row.len() < 14 { continue; }
            let frame_type = match row[11].as_str().unwrap_or("real") {
                "synthetic" => FrameType::Synthetic { model: SynthModel::GreatCircle },
                "gap_marker" => continue, // gap markers not useful in track response
                _ => FrameType::Real,
            };

            let ts = parse_i64(&row[10]).unwrap_or(0);
            frames.push(GhostFrame {
                frame_id:         parse_u64(&row[0]).unwrap_or(0),
                mmsi:             parse_u32(&row[1]).unwrap_or(0),
                vessel_name:      row[2].as_str().filter(|s| !s.is_empty()).map(|s| s.to_owned()),
                vessel_type:      parse_u8(&row[3]).unwrap_or(0),
                lat:              parse_f64(&row[4]).unwrap_or(0.0),
                lon:              parse_f64(&row[5]).unwrap_or(0.0),
                altitude:         0.0,
                position_acc:     false,
                sog:              parse_f32(&row[6]).unwrap_or(0.0),
                cog:              parse_f32(&row[7]).unwrap_or(0.0),
                heading:          parse_f32(&row[8]).unwrap_or(0.0),
                rot:              parse_f32(&row[9]).unwrap_or(0.0),
                timestamp_utc_ns: ts,
                ingested_at_ns:   ts,
                frame_type,
                confidence:       parse_f32(&row[12]).unwrap_or(1.0),
                source_node_id:   parse_u64(&row[13]).unwrap_or(0),
            });
        }

        Ok(frames)
    }

    async fn query_raw(&self, sql: &str) -> Result<Vec<Vec<serde_json::Value>>> {
        let url = format!("{}/exec", self.base_url);
        let resp = self.client
            .get(&url)
            .query(&[("query", sql)])
            .send()
            .await;

        match resp {
            Ok(r) if r.status().is_success() => {
                let qr: QdbResponse = r.json().await?;
                Ok(qr.dataset)
            }
            Ok(r) => {
                warn!("questdb query returned {}", r.status());
                Ok(vec![])
            }
            Err(e) => {
                warn!("questdb unreachable: {e}");
                Ok(vec![])
            }
        }
    }
}

fn parse_u32(v: &serde_json::Value) -> Option<u32> { v.as_u64().map(|n| n as u32) }
fn parse_u8(v: &serde_json::Value)  -> Option<u8>  { v.as_u64().map(|n| n as u8) }
fn parse_u64(v: &serde_json::Value) -> Option<u64> { v.as_u64() }
fn parse_i64(v: &serde_json::Value) -> Option<i64> { v.as_i64() }
fn parse_f64(v: &serde_json::Value) -> Option<f64> { v.as_f64() }
fn parse_f32(v: &serde_json::Value) -> Option<f32> { v.as_f64().map(|n| n as f32) }
