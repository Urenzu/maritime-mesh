use std::sync::Arc;

use axum::{
    extract::{State, WebSocketUpgrade},
    response::Response,
};
use axum::extract::ws::{Message, WebSocket};
use serde::Deserialize;
use tracing::{info, warn};

use crate::state::AppState;

#[derive(Deserialize, Clone, Default)]
struct Viewport {
    min_lat: f64,
    min_lon: f64,
    max_lat: f64,
    max_lon: f64,
}

impl Viewport {
    fn is_set(&self) -> bool {
        self.max_lat != 0.0 || self.max_lon != 0.0
    }

    fn contains(&self, lat: f64, lon: f64) -> bool {
        if !self.is_set() {
            return true; // no filter yet — send everything
        }
        lat >= self.min_lat && lat <= self.max_lat && lon >= self.min_lon && lon <= self.max_lon
    }
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMsg {
    Viewport(Viewport),
}

pub async fn stream_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.bus.subscribe();
    let mut viewport = Viewport::default();
    info!("ws client connected");

    loop {
        tokio::select! {
            // Incoming client messages (viewport updates).
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<ClientMsg>(&text) {
                            Ok(ClientMsg::Viewport(vp)) => { viewport = vp; }
                            Err(e) => warn!("unknown client msg: {e}"),
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }

            // Outgoing frames — filtered by current viewport.
            result = rx.recv() => {
                match result {
                    Ok(frame) => {
                        if !viewport.contains(frame.lat, frame.lon) {
                            continue;
                        }
                        let Ok(json) = serde_json::to_string(&frame) else { continue };
                        if socket.send(Message::Text(json)).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!("ws client lagged by {n} frames");
                    }
                    Err(_) => break,
                }
            }
        }
    }

    info!("ws client disconnected");
}
