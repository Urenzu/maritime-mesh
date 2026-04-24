use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use tracing::{info, warn};

const WS_URL: &str = "wss://stream.aisstream.io/v0/stream";

#[derive(Serialize)]
struct Subscription {
    #[serde(rename = "APIKey")]
    api_key: String,
    #[serde(rename = "BoundingBoxes")]
    bounding_boxes: Vec<[[f64; 2]; 2]>,
    #[serde(rename = "FilterMessageTypes")]
    filter_message_types: Vec<String>,
}

/// Top-level AISStream envelope.
#[derive(Debug, Deserialize)]
pub struct AisStreamMsg {
    #[serde(rename = "MessageType")]
    #[allow(dead_code)]
    pub message_type: String,
    #[serde(rename = "MetaData")]
    pub meta: MetaData,
    #[serde(rename = "Message")]
    pub message: MessagePayload,
}

#[derive(Debug, Deserialize)]
pub struct MetaData {
    #[serde(rename = "MMSI")]
    pub mmsi: u32,
    #[serde(rename = "ShipName")]
    pub ship_name: Option<String>,
    #[allow(dead_code)]
    pub latitude: f64,
    #[allow(dead_code)]
    pub longitude: f64,
    #[allow(dead_code)]
    pub time_utc: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MessagePayload {
    #[serde(rename = "PositionReport")]
    pub position_report: Option<PositionReport>,
    #[serde(rename = "StandardClassBPositionReport")]
    pub class_b: Option<ClassBReport>,
}

#[derive(Debug, Deserialize)]
pub struct PositionReport {
    #[serde(rename = "Sog")]
    pub sog: f32,
    #[serde(rename = "Cog")]
    pub cog: f32,
    #[serde(rename = "TrueHeading")]
    pub true_heading: u16,
    #[serde(rename = "RateOfTurn")]
    pub rate_of_turn: i8,
    #[serde(rename = "NavigationalStatus")]
    #[allow(dead_code)]
    pub nav_status: u8,
    #[serde(rename = "Latitude")]
    pub lat: f64,
    #[serde(rename = "Longitude")]
    pub lon: f64,
}

#[derive(Debug, Deserialize)]
pub struct ClassBReport {
    #[serde(rename = "Sog")]
    pub sog: f32,
    #[serde(rename = "Cog")]
    pub cog: f32,
    #[serde(rename = "TrueHeading")]
    pub true_heading: u16,
    #[serde(rename = "Latitude")]
    pub lat: f64,
    #[serde(rename = "Longitude")]
    pub lon: f64,
}

/// Connect to AISStream and return a channel of decoded messages.
/// Subscribes to position reports globally (full bounding box).
pub async fn connect(api_key: &str) -> Result<mpsc::Receiver<AisStreamMsg>> {
    let (ws_stream, _) = connect_async(WS_URL).await?;
    info!("AISStream WebSocket connected");

    let (mut sink, mut stream) = ws_stream.split();

    // Subscribe: global coverage, position reports only.
    let sub = Subscription {
        api_key: api_key.to_string(),
        bounding_boxes: vec![[[-90.0, -180.0], [90.0, 180.0]]],
        filter_message_types: vec![
            "PositionReport".into(),
            "StandardClassBPositionReport".into(),
        ],
    };
    sink.send(Message::Text(serde_json::to_string(&sub)?)).await?;
    info!("AISStream subscription sent");

    let (tx, rx) = mpsc::channel::<AisStreamMsg>(4096);

    tokio::spawn(async move {
        while let Some(msg) = stream.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    match serde_json::from_str::<AisStreamMsg>(&text) {
                        Ok(m) => { let _ = tx.send(m).await; }
                        Err(e) => warn!("deserialize error: {e}: {text}"),
                    }
                }
                Ok(Message::Close(_)) => break,
                Err(e) => { warn!("ws error: {e}"); break; }
                _ => {}
            }
        }
    });

    Ok(rx)
}
