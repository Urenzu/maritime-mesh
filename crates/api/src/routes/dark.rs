use std::sync::Arc;

use axum::{extract::{Path, State}, http::StatusCode, Json};
use chrono::{Duration, Utc};
use tracing::warn;

use crate::gfw::sar::Bbox;
use crate::state::AppState;

pub async fn vessel_dark(
    Path(mmsi): Path<u32>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let local_event  = state.dark_zone.vessel_event(mmsi);
    let vessel_state = state.vessels.read().unwrap().get(&mmsi).cloned();

    let gfw_vessel = state.gfw.vessel_by_mmsi(mmsi).await.unwrap_or(None);

    let end       = Utc::now();
    let start     = end - Duration::days(7);
    let start_str = start.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    let end_str   = end.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

    let gfw_gaps = if let Some(ref v) = gfw_vessel {
        state.gfw.gap_events(&v.gfw_id, &start_str, &end_str)
            .await.unwrap_or_else(|e| { warn!("GFW gaps: {e}"); vec![] })
    } else { vec![] };

    let (sar, viirs) = match local_event.as_ref().or(vessel_state.as_ref().map(|_| ()).and(None::<&_>)) {
        _ if local_event.is_some() => {
            let e = local_event.as_ref().unwrap();
            let bbox = Bbox::around(e.last_lat, e.last_lon, 100.0);
            let s = state.gfw.sar_detections(&bbox, &start_str, &end_str)
                .await.unwrap_or_else(|e| { warn!("SAR: {e}"); vec![] });
            let v = state.gfw.viirs_detections(&bbox, &start_str, &end_str)
                .await.unwrap_or_else(|e| { warn!("VIIRS: {e}"); vec![] });
            (s, v)
        }
        _ => (vec![], vec![]),
    };

    let unmatched_sar   = sar.iter().filter(|d| !d.ais_matched).count();
    let unmatched_viirs = viirs.iter().filter(|d| !d.ais_matched).count();

    let confidence = match (local_event.as_ref(), gfw_gaps.is_empty(), unmatched_sar) {
        (None, _, _)        => "none",
        (Some(_), true,  0) => "low",
        (Some(_), false, 0) => "medium",
        (Some(_), _,     _) => "high",
    };

    Ok(Json(serde_json::json!({
        "mmsi":             mmsi,
        "vessel":           gfw_vessel,
        "dark_confidence":  confidence,
        "local_event":      local_event,
        "gfw_gap_events":   gfw_gaps,
        "sar_detections":   { "total": sar.len(),   "unmatched_dark": unmatched_sar,   "detections": sar   },
        "viirs_detections": { "total": viirs.len(), "unmatched_dark": unmatched_viirs, "detections": viirs },
    })))
}

pub async fn dark_zones(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let events = state.dark_zone.active_events();
    Json(serde_json::json!({ "count": events.len(), "dark_events": events }))
}
