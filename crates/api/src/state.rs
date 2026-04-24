use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use dark_zone::DarkZoneDetector;
use ghost_log::FrameBus;
use schema::VesselState;

use crate::gfw::GfwClient;
use crate::questdb::QuestDbReader;

/// Live in-memory vessel state, keyed by MMSI.
/// Written by the Zenoh subscriber task; read by API routes and the dark-zone sweep.
pub type VesselMap = Arc<RwLock<HashMap<u32, VesselState>>>;

pub struct AppState {
    pub vessels:   VesselMap,
    pub bus:       FrameBus,
    pub dark_zone: Arc<DarkZoneDetector>,
    pub gfw:       Arc<GfwClient>,
    pub qdb:       Arc<QuestDbReader>,
}
