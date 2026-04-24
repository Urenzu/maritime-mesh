/// A simulated vessel that moves on a constant heading with small random drift.
pub struct SimVessel {
    pub mmsi: u32,
    pub lat: f64,
    pub lon: f64,
    pub sog: f32,   // knots
    pub cog: f32,   // degrees true
    pub heading: f32,
}

impl SimVessel {
    pub fn new(mmsi: u32) -> Self {
        // Scatter vessels around the North Sea / English Channel as a default.
        let seed = mmsi as f64;
        Self {
            mmsi,
            lat: 51.0 + (seed * 0.00137).sin() * 3.0,
            lon:  2.0 + (seed * 0.00241).cos() * 4.0,
            sog: 8.0 + ((seed * 0.0031).sin() as f32).abs() * 10.0,
            cog: (seed as f32 * 7.3) % 360.0,
            heading: (seed as f32 * 7.3) % 360.0,
        }
    }

    pub fn step(&mut self, elapsed_secs: f64) {
        const KNOTS_TO_DEG_PER_SEC_LAT: f64 = 0.514444 / 111_320.0;
        let bearing = self.cog.to_radians() as f64;
        let dist = self.sog as f64 * KNOTS_TO_DEG_PER_SEC_LAT * elapsed_secs;

        self.lat += dist * bearing.cos();
        self.lon += dist * bearing.sin() / self.lat.to_radians().cos();

        // Gentle heading drift.
        self.cog = (self.cog + 0.05) % 360.0;
        self.heading = self.cog;
    }
}
