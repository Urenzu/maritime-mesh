use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use schema::VesselState;
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DarkEvent {
    pub dark_event_id: u64,
    pub mmsi: u32,
    pub gap_start_ns: i64,
    pub gap_end_ns: Option<i64>,
    pub last_lat: f64,
    pub last_lon: f64,
    pub last_sog: f32,
    pub last_cog: f32,
    pub is_ongoing: bool,
}

struct VesselDarkState {
    event: DarkEvent,
}

pub struct DarkZoneDetector {
    threshold: Duration,
    active: Arc<Mutex<HashMap<u32, VesselDarkState>>>,
    next_id: Arc<Mutex<u64>>,
}

impl DarkZoneDetector {
    pub fn new(threshold: Duration) -> Self {
        Self {
            threshold,
            active: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
        }
    }

    /// Sweep the live in-memory vessel map.
    /// Called every 30s from the API's background task.
    pub fn sweep(&self, vessels: &HashMap<u32, VesselState>) {
        let now_ns = now_ns();
        let threshold_ns = self.threshold.as_nanos() as i64;
        let mut active = self.active.lock().unwrap();

        for (mmsi, state) in vessels {
            let gap = now_ns - state.last_seen_ns;

            match (active.get(mmsi), gap >= threshold_ns) {
                (None, true) => {
                    let mut id = self.next_id.lock().unwrap();
                    debug!(mmsi, gap_secs = gap / 1_000_000_000, "dark event opened");
                    active.insert(*mmsi, VesselDarkState {
                        event: DarkEvent {
                            dark_event_id: *id,
                            mmsi: *mmsi,
                            gap_start_ns: state.last_seen_ns,
                            gap_end_ns: None,
                            last_lat: state.lat,
                            last_lon: state.lon,
                            last_sog: state.sog,
                            last_cog: state.cog,
                            is_ongoing: true,
                        },
                    });
                    *id += 1;
                }
                (Some(_), false) => {
                    debug!(mmsi, "dark event closed — vessel reappeared");
                    active.remove(mmsi);
                }
                _ => {}
            }
        }
    }

    pub fn active_events(&self) -> Vec<DarkEvent> {
        self.active.lock().unwrap().values().map(|s| s.event.clone()).collect()
    }

    pub fn vessel_event(&self, mmsi: u32) -> Option<DarkEvent> {
        self.active.lock().unwrap().get(&mmsi).map(|s| s.event.clone())
    }
}

fn now_ns() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0)
}
