/// Raw AIS fields decoded from an AIVDM/AIVDO payload.
/// We decode only the fields present in Class A position reports (msg types 1/2/3)
/// and Class B (msg type 18) for now.
#[derive(Debug)]
pub struct AisPositionReport {
    pub mmsi: u32,
    #[allow(dead_code)]
    pub msg_type: u8,
    #[allow(dead_code)]
    pub nav_status: u8,
    pub rot: f32,       // rate of turn, deg/min
    pub sog: f32,       // speed over ground, knots (× 0.1 raw)
    pub lon: f64,       // degrees
    pub lat: f64,       // degrees
    pub cog: f32,       // course over ground, degrees (× 0.1 raw)
    pub heading: f32,   // true heading, degrees (511 = not available)
    pub position_acc: bool,
}

/// Decode an AIVDM payload bitstring into a position report.
/// Returns None if the message type is not a position report.
pub fn decode_position(payload: &str, fill_bits: u8) -> Option<AisPositionReport> {
    let bits = payload_to_bits(payload, fill_bits);
    if bits.len() < 168 {
        return None;
    }

    let msg_type = read_u8(&bits, 0, 6);
    if !matches!(msg_type, 1 | 2 | 3 | 18) {
        return None;
    }

    let mmsi = read_u32(&bits, 8, 30);

    if msg_type == 18 {
        // Class B — shorter report
        let sog = read_u16(&bits, 46, 10) as f32 * 0.1;
        let position_acc = bits[56];
        let lon = read_i32(&bits, 57, 28) as f64 / 10_000.0 / 60.0;
        let lat = read_i32(&bits, 85, 27) as f64 / 10_000.0 / 60.0;
        let cog = read_u16(&bits, 112, 12) as f32 * 0.1;
        let heading = read_u16(&bits, 124, 9) as f32;

        return Some(AisPositionReport {
            mmsi,
            msg_type,
            nav_status: 0,
            rot: 0.0,
            sog,
            lon,
            lat,
            cog,
            heading: if heading == 511.0 { f32::NAN } else { heading },
            position_acc,
        });
    }

    // Class A (1/2/3)
    let nav_status = read_u8(&bits, 38, 4);
    let rot_raw = read_i8(&bits, 42, 8);
    let rot = if rot_raw == -128 { f32::NAN } else { rot_raw as f32 };
    let sog = read_u16(&bits, 50, 10) as f32 * 0.1;
    let position_acc = bits[60];
    let lon = read_i32(&bits, 61, 28) as f64 / 10_000.0 / 60.0;
    let lat = read_i32(&bits, 89, 27) as f64 / 10_000.0 / 60.0;
    let cog = read_u16(&bits, 116, 12) as f32 * 0.1;
    let heading = read_u16(&bits, 128, 9) as f32;

    Some(AisPositionReport {
        mmsi,
        msg_type,
        nav_status,
        rot,
        sog,
        lon,
        lat,
        cog,
        heading: if heading == 511.0 { f32::NAN } else { heading },
        position_acc,
    })
}

// ── bit-level helpers ──────────────────────────────────────────────────────

fn payload_to_bits(payload: &str, fill_bits: u8) -> Vec<bool> {
    let mut bits = Vec::with_capacity(payload.len() * 6);
    for ch in payload.chars() {
        let mut val = ch as u8;
        val = val.wrapping_sub(48);
        if val > 40 {
            val = val.wrapping_sub(8);
        }
        for i in (0..6).rev() {
            bits.push((val >> i) & 1 == 1);
        }
    }
    let trim = fill_bits as usize;
    bits.truncate(bits.len().saturating_sub(trim));
    bits
}

fn read_u8(bits: &[bool], offset: usize, len: usize) -> u8 {
    read_u32(bits, offset, len) as u8
}

fn read_u16(bits: &[bool], offset: usize, len: usize) -> u16 {
    read_u32(bits, offset, len) as u16
}

fn read_u32(bits: &[bool], offset: usize, len: usize) -> u32 {
    let mut val = 0u32;
    for i in 0..len {
        if offset + i < bits.len() && bits[offset + i] {
            val |= 1 << (len - 1 - i);
        }
    }
    val
}

fn read_i8(bits: &[bool], offset: usize, len: usize) -> i8 {
    read_i32(bits, offset, len) as i8
}

fn read_i32(bits: &[bool], offset: usize, len: usize) -> i32 {
    let u = read_u32(bits, offset, len);
    // Sign-extend
    if len < 32 && (u >> (len - 1)) & 1 == 1 {
        (u | (!0u32 << len)) as i32
    } else {
        u as i32
    }
}
