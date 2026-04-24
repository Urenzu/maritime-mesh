use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::GfwClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GfwGapEvent {
    pub id: String,
    pub start: String,
    pub end: Option<String>,
    pub duration_hours: Option<f64>,
    pub distance_km: Option<f64>,
    pub implied_speed_knots: Option<f64>,
    pub off_position_count: Option<u32>,
    pub start_lat: Option<f64>,
    pub start_lon: Option<f64>,
    pub end_lat: Option<f64>,
    pub end_lon: Option<f64>,
}

impl GfwClient {
    /// Fetch GFW-detected AIS gap events for a vessel (by GFW vessel ID).
    /// These are GFW's own dark activity detections — cross-reference with ours.
    pub async fn gap_events(
        &self,
        gfw_vessel_id: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<GfwGapEvent>> {
        let path = format!(
            "/v3/events?types=GAP\
             &vessels[0]={gfw_vessel_id}\
             &startDate={start_date}\
             &endDate={end_date}\
             &datasets[0]=public-global-gaps-events:latest\
             &limit=100"
        );

        let Some(body) = self.get(&path).await? else {
            return Ok(vec![]);
        };

        let entries = match body["entries"].as_array() {
            Some(e) => e,
            None => return Ok(vec![]),
        };

        let events = entries
            .iter()
            .map(|e| GfwGapEvent {
                id: e["id"].as_str().unwrap_or("").to_string(),
                start: e["start"].as_str().unwrap_or("").to_string(),
                end: e["end"].as_str().map(str::to_string),
                duration_hours: e["gap"]["durationHours"].as_f64(),
                distance_km: e["gap"]["distanceKm"].as_f64(),
                implied_speed_knots: e["gap"]["impliedSpeedKnots"].as_f64(),
                off_position_count: e["gap"]["offPositionCount"].as_u64().map(|v| v as u32),
                start_lat: e["position"]["lat"].as_f64(),
                start_lon: e["position"]["lon"].as_f64(),
                end_lat: e["endPosition"]["lat"].as_f64(),
                end_lon: e["endPosition"]["lon"].as_f64(),
            })
            .collect();

        Ok(events)
    }
}
