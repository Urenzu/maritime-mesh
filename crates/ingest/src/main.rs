mod ais;
mod aisstream;
mod normalize;

use std::sync::Arc;

use anyhow::Result;
use ghost_log::{FjallStore, Store};
use schema::GhostFrame;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt().with_env_filter("info").init();

    let mode       = std::env::var("AISSTREAM_MODE").unwrap_or_else(|_| "live".into());
    let store_path = std::env::var("GHOST_LOG_PATH").unwrap_or_else(|_| "./data/ghost-log".into());

    std::fs::create_dir_all(&store_path)?;
    let store = Arc::new(FjallStore::open(store_path.as_ref())?);

    let mut rx: mpsc::Receiver<GhostFrame> = match mode.as_str() {
        "sim" => {
            let host = std::env::var("SIM_HOST").unwrap_or_else(|_| "localhost".into());
            let port = std::env::var("SIM_PORT").unwrap_or_else(|_| "10110".into());
            info!("sim mode: connecting to {host}:{port}");
            nmea_tcp_source(&host, &port).await?
        }
        _ => {
            let api_key = std::env::var("AISSTREAM_API_KEY").expect("AISSTREAM_API_KEY must be set");
            info!("live mode: connecting to AISStream");
            aisstream_source(&api_key).await?
        }
    };

    info!("ingest running");
    while let Some(frame) = rx.recv().await {
        if let Err(e) = store.write(&frame) {
            error!(mmsi = frame.mmsi, "ghost-log write failed: {e}");
        }
    }

    info!("source disconnected");
    Ok(())
}

async fn aisstream_source(api_key: &str) -> Result<mpsc::Receiver<GhostFrame>> {
    let api_key = api_key.to_string();
    let (tx, rx) = mpsc::channel::<GhostFrame>(4096);
    tokio::spawn(async move {
        let mut counter  = 0u64;
        let mut retry_ms = 3_000u64;
        loop {
            match aisstream::connect(&api_key).await {
                Err(e) => {
                    warn!("AISStream connect failed: {e}, retrying in {retry_ms}ms");
                    tokio::time::sleep(tokio::time::Duration::from_millis(retry_ms)).await;
                    retry_ms = (retry_ms * 2).min(30_000);
                }
                Ok(mut raw_rx) => {
                    info!("AISStream connected");
                    retry_ms = 3_000;
                    while let Some(msg) = raw_rx.recv().await {
                        match normalize::aisstream_to_frame(msg, counter) {
                            Ok(Some(frame)) => { counter += 1; let _ = tx.send(frame).await; }
                            Ok(None) => {}
                            Err(e) => warn!("normalize: {e}"),
                        }
                    }
                    warn!("AISStream disconnected, reconnecting in {retry_ms}ms");
                    tokio::time::sleep(tokio::time::Duration::from_millis(retry_ms)).await;
                    retry_ms = (retry_ms * 2).min(30_000);
                }
            }
        }
    });
    Ok(rx)
}

async fn nmea_tcp_source(host: &str, port: &str) -> Result<mpsc::Receiver<GhostFrame>> {
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::net::TcpStream;

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
                        Err(e) => warn!("nmea parse: {e}"),
                    }
                }
                Ok(None) => { info!("sim TCP closed"); break; }
                Err(e)   => { warn!("nmea read: {e}"); break; }
            }
        }
    });
    Ok(rx)
}
