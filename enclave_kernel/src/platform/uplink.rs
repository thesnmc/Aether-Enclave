//! One-way uplink packet format and AES-256-GCM sealing.
//!
//! Radio TX is gated by [`super::mission_profile::radio_enabled`] and the
//! `radio-tx` Cargo feature. Default: no RF activity.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};

/// Plaintext alert frame (17 bytes) sealed for over-the-air burst.
#[derive(Clone, Copy, Debug)]
pub struct AlertPlaintext {
    pub mission_id: u32,
    pub cycle: u32,
    pub flags: u8,
    pub proof: u64,
}

/// Sealed uplink blob: nonce (12) + ciphertext (17) + tag (16) = 45 bytes.
pub const SEALED_LEN: usize = 12 + 17 + 16;

/// Development PSK — replace at manufacture; never commit production keys.
const DEV_PSK: [u8; 32] = [
    0xAE, 0x7E, 0x1C, 0x4A, 0xD0, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09,
    0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
    0x19, 0x1A,
];

impl AlertPlaintext {
    pub fn encode(self) -> [u8; 17] {
        let mut out = [0u8; 17];
        out[0..4].copy_from_slice(&self.mission_id.to_le_bytes());
        out[4..8].copy_from_slice(&self.cycle.to_le_bytes());
        out[8] = self.flags;
        out[9..17].copy_from_slice(&self.proof.to_le_bytes());
        out
    }
}

/// Build deterministic nonce from cycle + mission (unique per cycle for demo PSK).
pub fn nonce_for(cycle: u32, mission_id: u32) -> [u8; 12] {
    let mut nonce = [0u8; 12];
    nonce[0..4].copy_from_slice(&cycle.to_le_bytes());
    nonce[4..8].copy_from_slice(&mission_id.to_le_bytes());
    nonce[8..12].copy_from_slice(b"AETH");
    nonce
}

/// Seal plaintext with AES-256-GCM; returns nonce || ciphertext || tag.
pub fn seal(plain: &AlertPlaintext) -> Result<[u8; SEALED_LEN], ()> {
    let cipher = Aes256Gcm::new_from_slice(&DEV_PSK).map_err(|_| ())?;
    let nonce_bytes = nonce_for(plain.cycle, plain.mission_id);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plain.encode().as_ref())
        .map_err(|_| ())?;
    if ciphertext.len() != 17 + 16 {
        return Err(());
    }
    let mut out = [0u8; SEALED_LEN];
    out[0..12].copy_from_slice(&nonce_bytes);
    out[12..].copy_from_slice(&ciphertext);
    Ok(out)
}

/// Decrypt a sealed frame (receiver / dry-run verify).
pub fn open(frame: &[u8; SEALED_LEN]) -> Result<AlertPlaintext, ()> {
    let cipher = Aes256Gcm::new_from_slice(&DEV_PSK).map_err(|_| ())?;
    let nonce = Nonce::from_slice(&frame[0..12]);
    let plain = cipher.decrypt(nonce, &frame[12..]).map_err(|_| ())?;
    if plain.len() != 17 {
        return Err(());
    }
    Ok(AlertPlaintext {
        mission_id: u32::from_le_bytes([plain[0], plain[1], plain[2], plain[3]]),
        cycle: u32::from_le_bytes([plain[4], plain[5], plain[6], plain[7]]),
        flags: plain[8],
        proof: u64::from_le_bytes([
            plain[9], plain[10], plain[11], plain[12], plain[13], plain[14], plain[15], plain[16],
        ]),
    })
}

/// Hex format for serial dry-run when radio is enabled in profile but TX stubbed.
pub fn sealed_hex(frame: &[u8; SEALED_LEN]) -> alloc::string::String {
    use core::fmt::Write;
    let mut s = alloc::string::String::with_capacity(SEALED_LEN * 2 + 8);
    for b in frame {
        let _ = write!(s, "{b:02X}");
    }
    s
}
