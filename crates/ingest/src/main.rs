mod ais;
mod aisstream;
mod normalize;
mod questdb;

use std::sync::Arc;

use anyhow::Result;
use ghost_log::{FjallStore, Store};
use tracing::{error, info};

use questdb::QuestDbClient;

const NODE_ID: u64 = 0; // unique per vessel/node in production

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt().with_env_filter("info").init();

    let api_key    = std::env::var("AISSTREAM_API_KEY").expect("AISSTREAM_API_KEY must be set");
    let store_path = std::env::var("GHOST_LOG_PATH").unwrap_or_else(|_| "./data/ghost-log".into());
    let qdb_addr   = std::env::var("QUESTDB_ILP_ADDR").unwrap_or_else(|_| "localhost:9009".into());

    std::fs::create_dir_all(&store_path)?;
    let store = Arc::new(FjallStore::open(store_path.as_ref())?);
    let mut qdb = QuestDbClient::new(qdb_addr);

    let publisher = mesh::open_publisher(NODE_ID).await?;

    info!("connecting to AISStream");
    let mut rx = aisstream::connect(&api_key).await?;
    let mut frame_counter = 0u64;

    while let Some(msg) = rx.recv().await {
        match normalize::aisstream_to_frame(msg, frame_counter) {
            Ok(Some(frame)) => {
                frame_counter += 1;

                // 1. Persist to ghost-log (edge durability / forensic record)
                if let Err(e) = store.write(&frame) {
                    error!(mmsi = frame.mmsi, "ghost-log write failed: {e}");
                }

                // 2. Mirror to QuestDB (cloud analytics / track history)
                qdb.write(&frame).await;

                // 3. Publish to Zenoh mesh (live distribution)
                publisher.publish(&frame).await;
            }
            Ok(None) => {}
            Err(e) => tracing::warn!("normalize error: {e}"),
        }
    }

    info!("AISStream disconnected");
    Ok(())
}
