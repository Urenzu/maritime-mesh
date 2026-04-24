use anyhow::Result;
use schema::GhostFrame;
use zenoh::prelude::r#async::*;
use tracing::warn;

pub struct FramePublisher {
    session: zenoh::Session,
    node_id: u64,
}

impl FramePublisher {
    pub fn new(session: zenoh::Session, node_id: u64) -> Self {
        Self { session, node_id }
    }

    pub async fn publish(&self, frame: &GhostFrame) {
        let Ok(bytes) = serde_json::to_vec(frame) else { return };
        let key = crate::frame_key(self.node_id);
        if let Err(e) = self.session.put(key, bytes).res().await {
            warn!("zenoh put: {e:?}");
        }
    }
}

pub async fn open_publisher(node_id: u64) -> Result<FramePublisher> {
    let session = zenoh::open(zenoh::config::peer())
        .res()
        .await
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    Ok(FramePublisher::new(session, node_id))
}
