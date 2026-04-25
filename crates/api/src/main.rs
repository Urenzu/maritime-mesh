mod aisstream;
mod gfw;
mod routes;
mod state;
mod ws;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use anyhow::Result;
use axum::{Router, routing::get};
use dark_zone::DarkZoneDetector;
use ghost_log::{FrameBus, FjallStore, Store};
use schema::{FrameType, GhostFrame, VesselState};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

use gfw::GfwClient;
use state::{AppState, VesselMap};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt().with_env_filter("info").init();

    let bind_addr  = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into());
    let gfw_key    = std::env::var("GFW_API_KEY").unwrap_or_default();
    let store_path = std::env::var("GHOST_LOG_PATH").unwrap_or_else(|_| "./data/ghost-log".into());
    let dark_mins: u64 = std::env::var("DARK_ZONE_THRESHOLD_MINS")
        .ok().and_then(|v| v.parse().ok()).unwrap_or(15);
    let ais_key    = std::env::var("AISSTREAM_API_KEY").unwrap_or_default();

    let vessels: VesselMap = Arc::new(RwLock::new(HashMap::new()));
    let bus                = FrameBus::new(8192);
    let dark_zone          = Arc::new(DarkZoneDetector::new(Duration::from_secs(dark_mins * 60)));
    let gfw                = Arc::new(GfwClient::new(gfw_key)?);

    // Ghost-log: persistent history store (cold-start + track queries).
    std::fs::create_dir_all(&store_path)?;
    let ghost_log = Arc::new(FjallStore::open(store_path.as_ref())?);

    // Cold-start: seed vessel map from ghost-log.
    match ghost_log.vessels() {
        Ok(mmsis) => {
            let mut map = vessels.write().unwrap();
            let mut n = 0usize;
            for mmsi in mmsis {
                if let Ok(Some(state)) = ghost_log.latest(mmsi) {
                    map.insert(mmsi, state);
                    n += 1;
                }
            }
            info!("cold-start: loaded {n} vessel states from ghost-log");
        }
        Err(e) => info!("cold-start: ghost-log empty ({e})"),
    }

    // Live AISStream feed — direct WebSocket, reconnects on failure.
    if !ais_key.is_empty() {
        let vessels_w  = vessels.clone();
        let bus_w      = bus.clone();
        let ghost_w    = ghost_log.clone();
        let key        = ais_key.clone();
        tokio::spawn(async move {
            let mut retry_ms = 3_000u64;
            let mut counter  = 0u64;
            loop {
                match aisstream::connect(&key).await {
                    Err(e) => {
                        warn!("AISStream connect failed: {e}, retrying in {retry_ms}ms");
                        tokio::time::sleep(tokio::time::Duration::from_millis(retry_ms)).await;
                        retry_ms = (retry_ms * 2).min(30_000);
                    }
                    Ok(mut rx) => {
                        info!("AISStream live feed active");
                        retry_ms = 3_000;
                        while let Some(msg) = rx.recv().await {
                            counter += 1;
                            let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
                            let state = VesselState {
                                mmsi:         msg.mmsi,
                                vessel_name:  msg.ship_name.clone(),
                                vessel_type:  0,
                                lat:          msg.lat,
                                lon:          msg.lon,
                                sog:          msg.sog,
                                cog:          msg.cog,
                                heading:      msg.true_heading.map(|h| h as f32),
                                last_seen_ns: now_ns,
                                is_dark:      false,
                                confidence:   1.0,
                            };
                            vessels_w.write().unwrap().insert(msg.mmsi, state);

                            let frame = GhostFrame {
                                frame_id:         counter,
                                mmsi:             msg.mmsi,
                                vessel_name:      msg.ship_name,
                                vessel_type:      0,
                                lat:              msg.lat,
                                lon:              msg.lon,
                                altitude:         0.0,
                                position_acc:     false,
                                sog:              msg.sog,
                                cog:              msg.cog,
                                heading:          msg.true_heading.map(|h| h as f32),
                                rot:              msg.rate_of_turn.map(|r| r as f32),
                                timestamp_utc_ns: now_ns,
                                ingested_at_ns:   now_ns,
                                frame_type:       FrameType::Real,
                                confidence:       1.0,
                                source_node_id:   0,
                            };
                            let _ = ghost_w.write(&frame);
                            bus_w.publish(frame);
                        }
                        warn!("AISStream disconnected, reconnecting in {retry_ms}ms");
                        tokio::time::sleep(tokio::time::Duration::from_millis(retry_ms)).await;
                        retry_ms = (retry_ms * 2).min(30_000);
                    }
                }
            }
        });
    } else {
        warn!("AISSTREAM_API_KEY not set — live feed disabled, running on cold-start data only");
    }

    // Dark-zone sweep every 30s.
    {
        let dz         = dark_zone.clone();
        let vessels_dz = vessels.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                let map = vessels_dz.read().unwrap().clone();
                dz.sweep(&map);
            }
        });
    }

    let state = Arc::new(AppState { vessels, bus, dark_zone, gfw, ghost_log });

    let app = Router::new()
        .route("/v1/vessels",                      get(routes::vessels::list))
        .route("/v1/vessels/:mmsi",                get(routes::vessels::get_vessel))
        .route("/v1/vessels/:mmsi/track",          get(routes::vessels::track))
        .route("/v1/vessels/:mmsi/dark",           get(routes::dark::vessel_dark))
        .route("/v1/dark-zones",                   get(routes::dark::dark_zones))
        .route("/v1/tiles/presence/:z/:x/:y",      get(routes::tiles::presence_tile))
        .route("/v1/tiles/fishing/:z/:x/:y",       get(routes::tiles::fishing_tile))
        .route("/v1/stream",                       get(ws::stream_handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    info!("listening on {bind_addr} (dark threshold: {dark_mins}min)");
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
