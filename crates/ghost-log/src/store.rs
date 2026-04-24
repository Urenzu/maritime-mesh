use anyhow::Result;
use schema::{GhostFrame, VesselState};

pub trait Store: Send + Sync + 'static {
    fn write(&self, frame: &GhostFrame) -> Result<()>;

    /// Latest known state for a vessel.
    fn latest(&self, mmsi: u32) -> Result<Option<VesselState>>;

    /// Positional track for a vessel within a time range (Unix ns).
    fn track(&self, mmsi: u32, from_ns: i64, to_ns: i64) -> Result<Vec<GhostFrame>>;

    /// All MMSIs currently in the store.
    fn vessels(&self) -> Result<Vec<u32>>;
}
