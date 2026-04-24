use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
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
    let now_ns   = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as i64;
    let from_ns  = params.from_ns.unwrap_or(now_ns - 24 * 3_600 * 1_000_000_000);
    let to_ns    = params.to_ns.unwrap_or(now_ns);

    match state.qdb.track(mmsi, from_ns, to_ns).await {
        Ok(frames) => Json(serde_json::json!({
            "mmsi":   mmsi,
            "count":  frames.len(),
            "frames": frames,
        })),
        Err(e) => Json(serde_json::json!({
            "mmsi":  mmsi,
            "error": e.to_string(),
            "frames": [],
        })),
    }
}
