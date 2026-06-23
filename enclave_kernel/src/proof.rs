//! Tamper-evident proof chain — each cycle links to the previous proof.

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01B3;

fn mix(mut h: u64, word: u32) -> u64 {
    h ^= u64::from(word);
    h = h.wrapping_mul(FNV_PRIME);
    h
}

fn mix_u8(mut h: u64, byte: u8) -> u64 {
    h ^= u64::from(byte);
    h = h.wrapping_mul(FNV_PRIME);
    h
}

/// Chain-link proof: `hash(prev || guest || sensors || vector || cycle || mission)`.
pub fn chain_proof(
    prev_proof: u64,
    guest_result: i32,
    pressure_bits: u32,
    dose: u32,
    vector: u8,
    cycle: u32,
    mission_id: u32,
    payload_slot: u8,
) -> u64 {
    let mut h = FNV_OFFSET;
    h = mix(h, (prev_proof >> 32) as u32);
    h = mix(h, prev_proof as u32);
    h = mix(h, guest_result as u32);
    h = mix(h, pressure_bits);
    h = mix(h, dose);
    h = mix_u8(h, vector);
    h = mix(h, cycle);
    h = mix(h, mission_id);
    h = mix_u8(h, payload_slot);
    h
}
