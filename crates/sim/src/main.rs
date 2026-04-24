/// Lightweight AIS simulator.
/// Emits NMEA AIVDM sentences over TCP on port 10110, matching the real feed format.
/// The ingest binary connects to this the same way it connects to a live AIS aggregator.
mod vessel;
mod encode;

use std::time::Duration;
use anyhow::Result;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tracing::info;

const VESSEL_COUNT: usize = 50;
const TICK_MS: u64 = 2000;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let bind = std::env::var("SIM_ADDR").unwrap_or_else(|_| "0.0.0.0:10110".into());
    let listener = TcpListener::bind(&bind).await?;
    info!("sim listening on {bind} ({VESSEL_COUNT} vessels, {TICK_MS}ms tick)");

    loop {
        let (mut socket, addr) = listener.accept().await?;
        info!("ingest connected from {addr}");

        tokio::spawn(async move {
            let mut vessels: Vec<vessel::SimVessel> = (0..VESSEL_COUNT)
                .map(|i| vessel::SimVessel::new(200_000_000 + i as u32))
                .collect();

            let mut interval = tokio::time::interval(Duration::from_millis(TICK_MS));
            loop {
                interval.tick().await;
                for v in &mut vessels {
                    v.step(TICK_MS as f64 / 1000.0);
                    let sentence = encode::to_aivdm(v);
                    if socket.write_all(sentence.as_bytes()).await.is_err() {
                        return;
                    }
                }
            }
        });
    }
}
