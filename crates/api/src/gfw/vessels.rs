use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::GfwClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VesselInfo {
    pub gfw_id: String,
    pub mmsi: Option<String>,
    pub imo: Option<String>,
    pub ship_name: Option<String>,
    pub flag: Option<String>,
    pub vessel_type: Option<String>,
    pub length_m: Option<f32>,
    pub tonnage_gt: Option<f32>,
}

impl GfwClient {
    /// Look up a vessel by MMSI and return GFW vessel info including their internal ID.
    pub async fn vessel_by_mmsi(&self, mmsi: u32) -> Result<Option<VesselInfo>> {
        let path = format!(
            "/v3/vessels/search?query={mmsi}&datasets[0]=public-global-vessel-identity:latest&includes[0]=OWNERSHIP&includes[1]=AUTHORIZATIONS"
        );

        let Some(body) = self.get(&path).await? else {
            return Ok(None);
        };

        let entries = body["entries"].as_array();
        let Some(entries) = entries else { return Ok(None) };

        // Take the first match with a matching MMSI.
        for entry in entries {
            let entry_mmsi = entry["ssvid"].as_str().unwrap_or("");
            if entry_mmsi == mmsi.to_string().as_str() {
                return Ok(Some(VesselInfo {
                    gfw_id: entry["id"].as_str().unwrap_or("").to_string(),
                    mmsi: Some(entry_mmsi.to_string()),
                    imo: entry["imo"].as_str().map(str::to_string),
                    ship_name: entry["shipname"].as_str().map(str::to_string),
                    flag: entry["flag"].as_str().map(str::to_string),
                    vessel_type: entry["vesselType"].as_str().map(str::to_string),
                    length_m: entry["lengthM"].as_f64().map(|v| v as f32),
                    tonnage_gt: entry["tonnageGt"].as_f64().map(|v| v as f32),
                }));
            }
        }

        Ok(None)
    }
}
