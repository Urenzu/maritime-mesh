use crate::vessel::SimVessel;

/// Encode a SimVessel into a minimal AIVDM sentence (Class A position report, msg type 1).
/// This is a best-effort encoder for testing — not ITU-R M.1371 certified.
pub fn to_aivdm(v: &SimVessel) -> String {
    let payload = encode_payload(v);
    let sentence = format!("!AIVDM,1,1,,A,{},0", payload);
    let checksum = nmea_checksum(&sentence[1..]); // skip leading '!'
    format!("{sentence}*{checksum:02X}\r\n")
}

fn encode_payload(v: &SimVessel) -> String {
    let mut bits = vec![0u8; 168];

    write_bits(&mut bits, 0,  6, 1);                              // msg type 1
    write_bits(&mut bits, 6,  2, 0);                              // repeat
    write_bits(&mut bits, 8, 30, v.mmsi as u64);
    write_bits(&mut bits, 38, 4, 0);                              // nav status: underway
    write_bits_i(&mut bits, 42, 8, -128);                         // ROT not available
    write_bits(&mut bits, 50, 10, (v.sog * 10.0) as u64);
    write_bits(&mut bits, 60, 1, 1);                              // position accuracy
    write_bits_i(&mut bits, 61, 28, (v.lon * 10_000.0 * 60.0) as i64);
    write_bits_i(&mut bits, 89, 27, (v.lat * 10_000.0 * 60.0) as i64);
    write_bits(&mut bits, 116, 12, (v.cog * 10.0) as u64);
    write_bits(&mut bits, 128, 9, v.heading as u64);

    bits_to_payload(&bits)
}

fn write_bits(bits: &mut [u8], offset: usize, len: usize, val: u64) {
    for i in 0..len {
        let bit = ((val >> (len - 1 - i)) & 1) as u8;
        bits[offset + i] = bit;
    }
}

fn write_bits_i(bits: &mut [u8], offset: usize, len: usize, val: i64) {
    let mask = if len < 64 { (1u64 << len) - 1 } else { u64::MAX };
    write_bits(bits, offset, len, (val as u64) & mask);
}

fn bits_to_payload(bits: &[u8]) -> String {
    bits.chunks(6)
        .map(|chunk| {
            let mut val = 0u8;
            for (i, &b) in chunk.iter().enumerate() {
                val |= b << (5 - i);
            }
            val += 48;
            if val > 87 { val += 8; }
            val as char
        })
        .collect()
}

fn nmea_checksum(s: &str) -> u8 {
    s.bytes()
        .take_while(|&b| b != b'*')
        .fold(0u8, |acc, b| acc ^ b)
}
