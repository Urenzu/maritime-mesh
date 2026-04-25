use anyhow::Result;
use serde::Serialize;
use serde_json::Value;
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

#[derive(Debug)]
pub struct AisStreamMsg {
    pub mmsi:         u32,
    pub ship_name:    Option<String>,
    pub lat:          f64,
    pub lon:          f64,
    pub sog:          f32,
    pub cog:          f32,
    pub true_heading: Option<u16>,
    pub rate_of_turn: Option<i16>,
}

fn parse_msg(v: &Value) -> Option<AisStreamMsg> {
    let meta = &v["MetaData"];
    let msg  = &v["Message"];

    let mmsi: u32 = meta["MMSI"].as_u64()?.try_into().ok()?;

    let pos = if !msg["PositionReport"].is_null() {
        &msg["PositionReport"]
    } else if !msg["StandardClassBPositionReport"].is_null() {
        &msg["StandardClassBPositionReport"]
    } else {
        return None;
    };

    let lat = pos["Latitude"].as_f64().unwrap_or(0.0);
    let lon = pos["Longitude"].as_f64().unwrap_or(0.0);
    if lat == 0.0 && lon == 0.0 { return None; }

    let sog = pos["Sog"].as_f64().unwrap_or(0.0) as f32;
    let cog = pos["Cog"].as_f64().unwrap_or(0.0) as f32;

    let true_heading = pos["TrueHeading"].as_u64().map(|v| v as u16);
    let rate_of_turn = pos["RateOfTurn"].as_i64().map(|v| v as i16);

    let ship_name = meta["ShipName"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string);

    Some(AisStreamMsg { mmsi, ship_name, lat, lon, sog, cog, true_heading, rate_of_turn })
}

pub async fn connect(api_key: &str) -> Result<mpsc::Receiver<AisStreamMsg>> {
    let (ws_stream, _) = connect_async(WS_URL).await?;
    info!("AISStream WebSocket connected");

    let (mut sink, mut stream) = ws_stream.split();

    let sub = Subscription {
        api_key: api_key.to_string(),
        bounding_boxes: vec![[[-90.0, -180.0], [90.0, 180.0]]],
        filter_message_types: vec![
            "PositionReport".into(),
            "StandardClassBPositionReport".into(),
        ],
    };
    sink.send(Message::Text(serde_json::to_string(&sub)?)).await?;
    info!("AISStream subscription sent (global coverage)");

    let (tx, rx) = mpsc::channel::<AisStreamMsg>(8192);

    tokio::spawn(async move {
        let mut total    = 0u64;
        let mut accepted = 0u64;
        while let Some(msg) = stream.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    total += 1;
                    match serde_json::from_str::<Value>(&text) {
                        Ok(v) => {
                            if let Some(parsed) = parse_msg(&v) {
                                accepted += 1;
                                if accepted % 1000 == 0 {
                                    info!("AISStream: {accepted}/{total} frames accepted");
                                }
                                let _ = tx.send(parsed).await;
                            }
                        }
                        Err(e) => warn!("json parse: {e}"),
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
