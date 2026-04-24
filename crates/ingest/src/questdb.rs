use anyhow::Result;
use schema::{FrameType, GhostFrame};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tracing::{error, warn};

/// Async client that writes GhostFrames to QuestDB via InfluxDB Line Protocol (TCP, port 9009).
pub struct QuestDbClient {
    addr: String,
    stream: Option<TcpStream>,
}

impl QuestDbClient {
    pub fn new(addr: String) -> Self {
        Self { addr, stream: None }
    }

    async fn stream(&mut self) -> Result<&mut TcpStream> {
        if self.stream.is_none() {
            let s = TcpStream::connect(&self.addr).await?;
            self.stream = Some(s);
        }
        Ok(self.stream.as_mut().unwrap())
    }

    /// Write a single GhostFrame as an ILP line.
    /// Reconnects once on failure (QuestDB restarts, network blip).
    pub async fn write(&mut self, frame: &GhostFrame) {
        let line = frame_to_ilp(frame);
        if let Err(e) = self.send(&line).await {
            warn!("questdb write failed ({e}), reconnecting");
            self.stream = None;
            if let Err(e2) = self.send(&line).await {
                error!("questdb reconnect failed: {e2}");
            }
        }
    }

    async fn send(&mut self, line: &str) -> Result<()> {
        let s = self.stream().await?;
        s.write_all(line.as_bytes()).await?;
        Ok(())
    }
}

/// Serialise a GhostFrame to an InfluxDB Line Protocol line.
///
/// Measurement: `ghost_frames`
/// Tags: mmsi, frame_type, source_node_id
/// Fields: lat, lon, sog, cog, heading, rot, confidence, vessel_type, frame_id
/// Timestamp: nanoseconds (QuestDB native)
fn frame_to_ilp(f: &GhostFrame) -> String {
    let frame_type = match &f.frame_type {
        FrameType::Real => "real",
        FrameType::Synthetic { .. } => "synthetic",
        FrameType::GapMarker(_) => "gap_marker",
    };

    // Escape tag values per ILP spec (no spaces, commas, or = in tag values)
    let name_tag = f
        .vessel_name
        .as_deref()
        .unwrap_or("unknown")
        .replace([' ', ',', '='], "_");

    // ILP: measurement,tag=val field=val timestamp\n
    format!(
        "ghost_frames,mmsi={mmsi},frame_type={ft},source_node={node},vessel_name={name} \
         lat={lat},lon={lon},sog={sog},cog={cog},heading={hdg},rot={rot},\
         confidence={conf},vessel_type={vtype}i,frame_id={fid}i {ts}\n",
        mmsi  = f.mmsi,
        ft    = frame_type,
        node  = f.source_node_id,
        name  = name_tag,
        lat   = f.lat,
        lon   = f.lon,
        sog   = f.sog,
        cog   = f.cog,
        hdg   = f.heading,
        rot   = f.rot,
        conf  = f.confidence,
        vtype = f.vessel_type,
        fid   = f.frame_id,
        ts    = f.timestamp_utc_ns,
    )
}
