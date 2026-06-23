//! 802.15.4 TX stub — enable with Cargo feature `radio-tx`.
//!
//! Full MAC/PHY bring-up lands when a second ESP32-C6 receiver board is available.

/// Transmit sealed uplink frame. Returns true on success.
#[cfg(feature = "radio-tx")]
pub fn transmit(_frame: &[u8; super::uplink::SEALED_LEN]) -> bool {
    // TODO: esp-hal IEEE 802.15.4 burst when receiver hardware is on the bench.
    false
}
