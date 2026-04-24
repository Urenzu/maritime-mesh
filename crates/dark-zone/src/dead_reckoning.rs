/// Constant-velocity dead-reckoning.
/// Projects a vessel's last known position forward by `elapsed_secs`
/// using its last known SOG and COG.
///
/// Returns (lat, lon, confidence) where confidence decays linearly with time.
/// Phase 2: replace with Kalman filter (constant velocity / constant acceleration).
pub fn project(
    lat: f64,
    lon: f64,
    sog_knots: f32,
    cog_deg: f32,
    elapsed_secs: f64,
    confidence_decay_per_hour: f32,
) -> (f64, f64, f32) {
    const KNOTS_TO_M_PER_S: f64 = 0.514444;
    const EARTH_RADIUS_M: f64 = 6_371_000.0;

    let distance_m = sog_knots as f64 * KNOTS_TO_M_PER_S * elapsed_secs;
    let bearing_rad = (cog_deg as f64).to_radians();

    let lat_rad = lat.to_radians();
    let lon_rad = lon.to_radians();
    let angular = distance_m / EARTH_RADIUS_M;

    let new_lat_rad = (lat_rad.sin() * angular.cos()
        + lat_rad.cos() * angular.sin() * bearing_rad.cos())
    .asin();

    let new_lon_rad = lon_rad
        + (bearing_rad.sin() * angular.sin() * lat_rad.cos())
            .atan2(angular.cos() - lat_rad.sin() * new_lat_rad.sin());

    let hours_elapsed = elapsed_secs / 3600.0;
    let confidence = (1.0 - confidence_decay_per_hour * hours_elapsed as f32).max(0.0);

    (new_lat_rad.to_degrees(), new_lon_rad.to_degrees(), confidence)
}
