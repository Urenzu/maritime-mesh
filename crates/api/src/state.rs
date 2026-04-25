use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use dark_zone::DarkZoneDetector;
use ghost_log::{FrameBus, FjallStore};
use schema::VesselState;

use crate::gfw::GfwClient;

pub type VesselMap = Arc<RwLock<HashMap<u32, VesselState>>>;

pub struct AppState {
    pub vessels:    VesselMap,
    pub bus:        FrameBus,
    pub dark_zone:  Arc<DarkZoneDetector>,
    pub gfw:        Arc<GfwClient>,
    pub ghost_log:  Arc<FjallStore>,
}
