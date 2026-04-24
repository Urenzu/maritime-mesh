mod gfw;
mod questdb;
mod routes;
mod state;
mod ws;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use anyhow::Result;
use axum::{Router, routing::get};
use dark_zone::DarkZoneDetector;
use ghost_log::FrameBus;
use schema::VesselState;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

use gfw::GfwClient;
use questdb::QuestDbReader;
use state::{AppState, VesselMap};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt().with_env_filter("info").init();

    let bind_addr  = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into());
    let gfw_key    = std::env::var("GFW_API_KEY").expect("GFW_API_KEY must be set");
    let dark_mins: u64 = std::env::var("DARK_ZONE_THRESHOLD_MINS")
        .ok().and_then(|v| v.parse().ok()).unwrap_or(15);
    let qdb_url    = std::env::var("QUESTDB_HTTP_URL").unwrap_or_else(|_| "http://localhost:9000".into());

    let vessels:   VesselMap           = Arc::new(RwLock::new(HashMap::new()));
    let bus                            = FrameBus::new(8192);
    let dark_zone                      = Arc::new(DarkZoneDetector::new(Duration::from_secs(dark_mins * 60)));
    let gfw                            = Arc::new(GfwClient::new(gfw_key)?);
    let qdb                            = Arc::new(QuestDbReader::new(qdb_url));

    // Zenoh — subscribe to all frames published by ingest nodes on the mesh.
    let mut frame_rx = mesh::open_subscriber().await?;

    {
        let vessels_w  = vessels.clone();
        let bus_w      = bus.clone();
        tokio::spawn(async move {
            info!("zenoh frame subscriber running");
            while let Some(frame) = frame_rx.recv().await {
                let state = VesselState::from(&frame);
                vessels_w.write().unwrap().insert(frame.mmsi, state);
                bus_w.publish(frame);
            }
        });
    }

    // Cold-start: hydrate VesselMap from QuestDB (last 24h) before accepting traffic.
    match qdb.latest_vessel_states(24).await {
        Ok(states) => {
            let mut map = vessels.write().unwrap();
            let n = states.len();
            for s in states { map.insert(s.mmsi, s); }
            info!("cold-start: loaded {n} vessel states from QuestDB");
        }
        Err(e) => info!("cold-start: QuestDB unavailable ({e}), starting empty"),
    }

    // Dark-zone sweep every 30s against the live VesselMap.
    {
        let dz      = dark_zone.clone();
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

    let state = Arc::new(AppState { vessels, bus, dark_zone, gfw, qdb });

    let app = Router::new()
        .route("/v1/vessels",             get(routes::vessels::list))
        .route("/v1/vessels/:mmsi",       get(routes::vessels::get_vessel))
        .route("/v1/vessels/:mmsi/track", get(routes::vessels::track))
        .route("/v1/vessels/:mmsi/dark",  get(routes::dark::vessel_dark))
        .route("/v1/dark-zones",          get(routes::dark::dark_zones))
        .route("/v1/stream",              get(ws::stream_handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    info!("listening on {bind_addr} (dark threshold: {dark_mins}min)");
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
