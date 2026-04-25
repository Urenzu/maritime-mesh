use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Response,
};
use serde::Deserialize;

use crate::state::AppState;

const GFW_BASE: &str = "https://gateway.api.globalfishingwatch.org";

/// Query params forwarded to 4Wings (date-range, etc.)
#[derive(Deserialize)]
pub struct TileParams {
    #[serde(rename = "date-range")]
    pub date_range: Option<String>,
}

/// Proxy a GFW 4Wings vessel-presence heatmap tile.
/// GET /v1/tiles/presence/{z}/{x}/{y}
pub async fn presence_tile(
    Path((z, x, y)): Path<(u32, u32, u32)>,
    Query(params): Query<TileParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Response<axum::body::Body>, StatusCode> {
    proxy_tile("public-global-presence:latest", z, x, y, params, &state).await
}

/// Proxy a GFW 4Wings fishing-effort heatmap tile.
/// GET /v1/tiles/fishing/{z}/{x}/{y}
pub async fn fishing_tile(
    Path((z, x, y)): Path<(u32, u32, u32)>,
    Query(params): Query<TileParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Response<axum::body::Body>, StatusCode> {
    proxy_tile("public-global-fishing-effort:latest", z, x, y, params, &state).await
}

async fn proxy_tile(
    dataset: &str,
    z: u32, x: u32, y: u32,
    params: TileParams,
    state: &AppState,
) -> Result<Response<axum::body::Body>, StatusCode> {
    // Default to last 30 days if no range provided.
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let thirty_days_ago = (chrono::Utc::now() - chrono::Duration::days(30))
        .format("%Y-%m-%d")
        .to_string();
    let date_range = params.date_range
        .unwrap_or_else(|| format!("{},{}", thirty_days_ago, today));

    let url = format!(
        "{GFW_BASE}/v3/4wings/tile/heatmap/{z}/{x}/{y}\
         ?datasets[0]={dataset}\
         &date-range={date_range}\
         &temporal-aggregation=true\
         &format=PNG"
    );

    let resp = state.gfw.http
        .get(&url)
        .bearer_auth(&state.gfw.api_key)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    if !resp.status().is_success() {
        tracing::warn!("GFW tile {z}/{x}/{y} returned {}", resp.status());
        return Err(StatusCode::BAD_GATEWAY);
    }

    let content_type = resp.headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/png")
        .to_owned();

    let bytes = resp.bytes().await.map_err(|_| StatusCode::BAD_GATEWAY)?;

    let mut headers = HeaderMap::new();
    headers.insert("content-type", content_type.parse().unwrap());
    headers.insert("cache-control", "public, max-age=3600".parse().unwrap());

    Ok(Response::builder()
        .status(200)
        .header("content-type", content_type)
        .header("cache-control", "public, max-age=3600")
        .body(axum::body::Body::from(bytes))
        .unwrap())
}
