use anyhow::{Context, Result};
use fjall::{Config, Keyspace, PartitionCreateOptions, PartitionHandle};
use schema::{GhostFrame, VesselState};
use tracing::instrument;

use crate::store::Store;

/// Phase 1 storage backend.
/// Key schema:
///   track partition: "{mmsi}/{timestamp_utc_ns:020}" → JSON GhostFrame
///   latest partition: "{mmsi}"                       → JSON VesselState
pub struct FjallStore {
    _keyspace: Keyspace,
    track: PartitionHandle,
    latest: PartitionHandle,
}

impl FjallStore {
    pub fn open(path: &std::path::Path) -> Result<Self> {
        let keyspace = Config::new(path).open().context("open fjall keyspace")?;

        let track = keyspace
            .open_partition("track", PartitionCreateOptions::default())
            .context("open track partition")?;

        let latest = keyspace
            .open_partition("latest", PartitionCreateOptions::default())
            .context("open latest partition")?;

        Ok(Self { _keyspace: keyspace, track, latest })
    }
}

impl Store for FjallStore {
    #[instrument(skip(self, frame), fields(mmsi = frame.mmsi))]
    fn write(&self, frame: &GhostFrame) -> Result<()> {
        let track_key = format!("{:010}/{:020}", frame.mmsi, frame.timestamp_utc_ns);
        let latest_key = format!("{:010}", frame.mmsi);
        let state = VesselState::from(frame);

        let frame_bytes = serde_json::to_vec(frame)?;
        let state_bytes = serde_json::to_vec(&state)?;

        self.track.insert(track_key, frame_bytes)?;
        self.latest.insert(latest_key, state_bytes)?;

        Ok(())
    }

    fn latest(&self, mmsi: u32) -> Result<Option<VesselState>> {
        let key = format!("{:010}", mmsi);
        match self.latest.get(key)? {
            Some(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
            None => Ok(None),
        }
    }

    fn track(&self, mmsi: u32, from_ns: i64, to_ns: i64) -> Result<Vec<GhostFrame>> {
        let start = format!("{:010}/{:020}", mmsi, from_ns);
        let end = format!("{:010}/{:020}", mmsi, to_ns);

        let mut frames = Vec::new();
        for item in self.track.range(start..=end) {
            let (_, value) = item?;
            frames.push(serde_json::from_slice(&value)?);
        }
        Ok(frames)
    }

    fn vessels(&self) -> Result<Vec<u32>> {
        let mut mmsis = Vec::new();
        for item in self.latest.iter() {
            let (key, _) = item?;
            let key_str = std::str::from_utf8(&key)?;
            if let Ok(mmsi) = key_str.trim().parse::<u32>() {
                mmsis.push(mmsi);
            }
        }
        Ok(mmsis)
    }
}
