use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::GfwClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SatDetection {
    pub source: DetectionSource,
    pub lat: f64,
    pub lon: f64,
    pub timestamp: String,
    pub length_m: Option<f64>,
    pub score: Option<f64>,
    /// True if GFW matched this detection to an AIS-transmitting vessel.
    pub ais_matched: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DetectionSource {
    Sar,
    Viirs,
}

/// Bounding box for a spatial query.
pub struct Bbox {
    pub min_lat: f64,
    pub min_lon: f64,
    pub max_lat: f64,
    pub max_lon: f64,
}

impl Bbox {
    pub fn around(lat: f64, lon: f64, radius_km: f64) -> Self {
        let deg = radius_km / 111.0;
        let lon_deg = deg / lat.to_radians().cos();
        Self {
            min_lat: lat - deg,
            max_lat: lat + deg,
            min_lon: lon - lon_deg,
            max_lon: lon + lon_deg,
        }
    }

    fn as_geojson_polygon(&self) -> String {
        format!(
            r#"{{"type":"Polygon","coordinates":[[
              [{min_lon},{min_lat}],
              [{max_lon},{min_lat}],
              [{max_lon},{max_lat}],
              [{min_lon},{max_lat}],
              [{min_lon},{min_lat}]
            ]]}}"#,
            min_lon = self.min_lon,
            min_lat = self.min_lat,
            max_lon = self.max_lon,
            max_lat = self.max_lat,
        )
    }
}

impl GfwClient {
    /// Query SAR vessel detections within a bounding box and time window.
    /// Unmatched detections (ais_matched=false) are dark vessels — physically present but no AIS.
    pub async fn sar_detections(
        &self,
        bbox: &Bbox,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<SatDetection>> {
        self.sat_detections("sar-detections:latest", DetectionSource::Sar, bbox, start_date, end_date).await
    }

    /// Query VIIRS nighttime light detections within a bounding box and time window.
    /// Vessels detected by VIIRS with no AIS match are dark but still lit.
    pub async fn viirs_detections(
        &self,
        bbox: &Bbox,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<SatDetection>> {
        self.sat_detections("viirs-band-detections:latest", DetectionSource::Viirs, bbox, start_date, end_date).await
    }

    async fn sat_detections(
        &self,
        dataset: &str,
        source: DetectionSource,
        bbox: &Bbox,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<SatDetection>> {
        let geometry = urlencoding::encode(&bbox.as_geojson_polygon()).into_owned();
        let path = format!(
            "/v3/vessel-detections\
             ?datasets[0]={dataset}\
             &startDate={start_date}\
             &endDate={end_date}\
             &geometry={geometry}\
             &limit=200"
        );

        let Some(body) = self.get(&path).await? else {
            return Ok(vec![]);
        };

        let entries = match body["entries"].as_array() {
            Some(e) => e,
            None => return Ok(vec![]),
        };

        let detections = entries
            .iter()
            .map(|e| SatDetection {
                source: source.clone(),
                lat: e["lat"].as_f64().unwrap_or(0.0),
                lon: e["lon"].as_f64().unwrap_or(0.0),
                timestamp: e["timestamp"].as_str().unwrap_or("").to_string(),
                length_m: e["lengthM"].as_f64(),
                score: e["score"].as_f64(),
                ais_matched: e["vesselId"].is_string(),
            })
            .collect();

        Ok(detections)
    }
}
