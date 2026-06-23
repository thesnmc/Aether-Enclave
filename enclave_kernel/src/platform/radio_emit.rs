//! Optional one-way RF uplink — disabled unless mission profile enables it.

use crate::serial_println;

use super::{mission_profile, uplink};

/// After a successful cycle, emit sealed alert if mission profile requests radio.
pub fn emit_after_cycle(cycle: u32, guest_flags: i32, proof: u64) {
    if !mission_profile::radio_enabled() {
        return;
    }

    let plain = uplink::AlertPlaintext {
        mission_id: mission_profile::mission_id(),
        cycle,
        flags: guest_flags.max(0).min(255) as u8,
        proof,
    };

    let Ok(frame) = uplink::seal(&plain) else {
        serial_println!("[AETHER] uplink — seal failed");
        return;
    };

    #[cfg(feature = "radio-tx")]
    {
        if super::radio::transmit(&frame) {
            serial_println!("[AETHER] uplink — TX {} B", frame.len());
        } else {
            serial_println!("[AETHER] uplink — TX failed");
        }
    }

    #[cfg(not(feature = "radio-tx"))]
    {
        serial_println!(
            "[AETHER] uplink — dry-run sealed {} B hex={}",
            frame.len(),
            uplink::sealed_hex(&frame)
        );
        serial_println!("[AETHER] uplink — RF stub (build with --features radio-tx + receiver C6)");
    }
}
