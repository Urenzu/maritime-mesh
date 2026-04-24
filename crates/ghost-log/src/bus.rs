use schema::GhostFrame;
use tokio::sync::broadcast;

/// In-process pub/sub for real-time frame distribution.
/// Ingest writes here; the API WebSocket layer and dark-zone detector subscribe.
/// Phase 2: replaced by Zenoh for cross-node distribution.
#[derive(Clone)]
pub struct FrameBus {
    tx: broadcast::Sender<GhostFrame>,
}

impl FrameBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn publish(&self, frame: GhostFrame) {
        // Ignore send errors — no subscribers is fine (startup / shutdown race).
        let _ = self.tx.send(frame);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<GhostFrame> {
        self.tx.subscribe()
    }
}
