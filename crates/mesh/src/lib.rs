pub mod publisher;
pub mod subscriber;

pub use publisher::{FramePublisher, open_publisher};
pub use subscriber::open_subscriber;

pub fn frame_key(node_id: u64) -> String {
    format!("maritime/frames/{node_id}")
}
