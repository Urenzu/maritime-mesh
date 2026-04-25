// ingest — NMEA TCP receiver for hardware AIS antennas and simulators.
// For live AISStream data use the `api` binary instead.
mod ais;
mod normalize;

use std::sync::Arc;

use anyhow::Result;
use ghost_log::{FjallStore, Store};
use schema::GhostFrame;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt().with_env_filter("info").init();

    let store_path = std::env::var("GHOST_LOG_PATH").unwrap_or_else(|_| "./data/ghost-log".into());
    let host       = std::env::var("SIM_HOST").unwrap_or_else(|_| "localhost".into());
    let port       = std::env::var("SIM_PORT").unwrap_or_else(|_| "10110".into());

    std::fs::create_dir_all(&store_path)?;
    let store = Arc::new(FjallStore::open(store_path.as_ref())?);

    info!("ingest: NMEA TCP connecting to {host}:{port}");
    let mut rx = nmea_tcp_source(&host, &port).await?;

    info!("ingest running");
    while let Some(frame) = rx.recv().await {
        if let Err(e) = store.write(&frame) {
            error!(mmsi = frame.mmsi, "ghost-log write failed: {e}");
        }
    }

    info!("NMEA source disconnected");
    Ok(())
}

async fn nmea_tcp_source(host: &str, port: &str) -> Result<mpsc::Receiver<GhostFrame>> {
    let addr   = format!("{host}:{port}");
    let stream = TcpStream::connect(&addr).await?;
    let reader = BufReader::new(stream);
    let mut lines = reader.lines();

    let (tx, rx) = mpsc::channel::<GhostFrame>(4096);
    tokio::spawn(async move {
        let mut counter = 0u64;
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    match normalize::sentence_to_frame(&line, counter) {
                        Ok(Some(frame)) => { counter += 1; let _ = tx.send(frame).await; }
                        Ok(None) => {}
                        Err(e)   => warn!("nmea parse: {e}"),
                    }
                }
                Ok(None) => { info!("NMEA TCP closed"); break; }
                Err(e)   => { warn!("nmea read: {e}"); break; }
            }
        }
    });
    Ok(rx)
}
