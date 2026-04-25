use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use ghost_log::Store;
use serde::Deserialize;

use crate::state::AppState;

pub async fn list(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let vessels: Vec<_> = state.vessels.read().unwrap().values().cloned().collect();
    Json(serde_json::json!({ "count": vessels.len(), "vessels": vessels }))
}

pub async fn get_vessel(
    Path(mmsi): Path<u32>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.vessels.read().unwrap().get(&mmsi).cloned() {
        Some(v) => Ok(Json(serde_json::to_value(v).unwrap_or_default())),
        None    => Err(StatusCode::NOT_FOUND),
    }
}

#[derive(Deserialize)]
pub struct TrackParams {
    from_ns: Option<i64>,
    to_ns:   Option<i64>,
}

pub async fn track(
    Path(mmsi): Path<u32>,
    Query(params): Query<TrackParams>,
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let now_ns  = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let from_ns = params.from_ns.unwrap_or(now_ns - 24 * 3_600 * 1_000_000_000);
    let to_ns   = params.to_ns.unwrap_or(now_ns);

    match state.ghost_log.track(mmsi, from_ns, to_ns) {
        Ok(frames) => Json(serde_json::json!({
            "mmsi":   mmsi,
            "count":  frames.len(),
            "frames": frames,
        })),
        Err(e) => Json(serde_json::json!({
            "mmsi":   mmsi,
            "error":  e.to_string(),
            "frames": [],
        })),
    }
}
