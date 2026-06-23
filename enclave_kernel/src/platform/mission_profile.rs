//! Mission profile stored on microSD sector 2047 (512 bytes, no FAT).
//!
//! Pot at boot still tunes wake timer and dose scale within profile bounds.
//! Payload slot 0 = strict WASM, 1 = relaxed WASM (both embedded in flash).

use core::sync::atomic::{AtomicU8, AtomicU32, Ordering};

const MAGIC: [u8; 4] = *b"AEPR";
const VERSION: u8 = 2;
const VERSION_V1: u8 = 1;
const FLAG_RADIO_ENABLE: u8 = 0x01;
const FLAG_INTERVAL_WAKE: u8 = 0x02;

pub const DEFAULT_PRESSURE_LIMIT_ATM: f32 = 0.15;
pub const DEFAULT_DOSE_LIMIT: u32 = 1_000;
pub const DEFAULT_LEAK_RATE_ATM_S: f32 = 0.003;
pub const DEFAULT_WAKE_MIN_SECS: u8 = 5;
pub const DEFAULT_WAKE_MAX_SECS: u8 = 60;

static MISSION_ID: AtomicU32 = AtomicU32::new(0);
static PAYLOAD_SLOT: AtomicU8 = AtomicU8::new(0);
static PRESSURE_LIMIT_BITS: AtomicU32 = AtomicU32::new(f32::to_bits(DEFAULT_PRESSURE_LIMIT_ATM));
static DOSE_LIMIT: AtomicU32 = AtomicU32::new(DEFAULT_DOSE_LIMIT);
static LEAK_RATE_BITS: AtomicU32 = AtomicU32::new(f32::to_bits(DEFAULT_LEAK_RATE_ATM_S));
static WAKE_MIN: AtomicU8 = AtomicU8::new(DEFAULT_WAKE_MIN_SECS);
static WAKE_MAX: AtomicU8 = AtomicU8::new(DEFAULT_WAKE_MAX_SECS);
static LOADED_FROM_SD: AtomicU8 = AtomicU8::new(0);
static RADIO_ENABLE: AtomicU8 = AtomicU8::new(0);
static INTERVAL_WAKE: AtomicU8 = AtomicU8::new(0);

#[derive(Clone, Copy, Debug, Default)]
pub struct Profile {
    pub mission_id: u32,
    pub payload_slot: u8,
    pub wake_min_secs: u8,
    pub wake_max_secs: u8,
    pub pressure_limit_atm: f32,
    pub dose_limit: u32,
    pub leak_rate_atm_s: f32,
    pub radio_enable: bool,
    pub interval_wake: bool,
    pub from_sd: bool,
}

impl Profile {
    pub fn defaults() -> Self {
        Self {
            mission_id: 0,
            payload_slot: 0,
            wake_min_secs: DEFAULT_WAKE_MIN_SECS,
            wake_max_secs: DEFAULT_WAKE_MAX_SECS,
            pressure_limit_atm: DEFAULT_PRESSURE_LIMIT_ATM,
            dose_limit: DEFAULT_DOSE_LIMIT,
            leak_rate_atm_s: DEFAULT_LEAK_RATE_ATM_S,
            radio_enable: false,
            interval_wake: false,
            from_sd: false,
        }
    }
}

fn apply(p: Profile) {
    MISSION_ID.store(p.mission_id, Ordering::Release);
    PAYLOAD_SLOT.store(p.payload_slot.min(1), Ordering::Release);
    PRESSURE_LIMIT_BITS.store(p.pressure_limit_atm.to_bits(), Ordering::Release);
    DOSE_LIMIT.store(p.dose_limit.max(100), Ordering::Release);
    LEAK_RATE_BITS.store(p.leak_rate_atm_s.to_bits(), Ordering::Release);
    WAKE_MIN.store(p.wake_min_secs.max(1), Ordering::Release);
    WAKE_MAX.store(
        p.wake_max_secs.max(p.wake_min_secs.max(1)).min(120),
        Ordering::Release,
    );
    LOADED_FROM_SD.store(if p.from_sd { 1 } else { 0 }, Ordering::Release);
    RADIO_ENABLE.store(if p.radio_enable { 1 } else { 0 }, Ordering::Release);
    INTERVAL_WAKE.store(if p.interval_wake { 1 } else { 0 }, Ordering::Release);
}

fn parse_sector(buf: &[u8; 512]) -> Option<Profile> {
    if buf[0..4] != MAGIC {
        return None;
    }
    let version = buf[4];
    if version != VERSION && version != VERSION_V1 {
        return None;
    }
    let payload_slot = buf[5].min(1);
    let wake_min = buf[6].max(1);
    let wake_max = buf[7].max(wake_min).min(120);
    let mission_id = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]);
    let pressure_limit_atm = f32::from_bits(u32::from_le_bytes([
        buf[12], buf[13], buf[14], buf[15],
    ]));
    let dose_limit = u32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]).max(100);
    let leak_rate_atm_s = f32::from_bits(u32::from_le_bytes([
        buf[20], buf[21], buf[22], buf[23],
    ]));
    if !pressure_limit_atm.is_finite() || pressure_limit_atm <= 0.0 {
        return None;
    }
    if !leak_rate_atm_s.is_finite() || leak_rate_atm_s <= 0.0 {
        return None;
    }
    let radio_enable = version >= VERSION && buf[24] & FLAG_RADIO_ENABLE != 0;
    let interval_wake = version >= VERSION && buf[24] & FLAG_INTERVAL_WAKE != 0;
    Some(Profile {
        mission_id,
        payload_slot,
        wake_min_secs: wake_min,
        wake_max_secs: wake_max,
        pressure_limit_atm,
        dose_limit,
        leak_rate_atm_s,
        radio_enable,
        interval_wake,
        from_sd: true,
    })
}

/// Load profile from SD sector 2047 when present; otherwise keep defaults.
pub fn load_from_sd() -> Profile {
    apply(Profile::defaults());
    if let Some(buf) = super::sd_log::read_profile_sector() {
        if let Some(p) = parse_sector(&buf) {
            apply(p);
            return p;
        }
    }
    Profile::defaults()
}

/// Pot high selects relaxed payload + limits when SD did not supply a profile.
pub fn apply_pot_payload_override(raw_adc: u32) {
    if LOADED_FROM_SD.load(Ordering::Acquire) != 0 {
        return;
    }
    if raw_adc > 24_000 {
        PAYLOAD_SLOT.store(1, Ordering::Release);
        PRESSURE_LIMIT_BITS.store(f32::to_bits(0.10), Ordering::Release);
        DOSE_LIMIT.store(2_000, Ordering::Release);
    }
    // Pot >90% also enables uplink dry-run (serial hex only; RF still off unless radio-tx).
    if raw_adc > 28_000 {
        RADIO_ENABLE.store(1, Ordering::Release);
    }
    // Pot <10% enables periodic interval wake (optional scheduled checks).
    if raw_adc < 3_200 {
        INTERVAL_WAKE.store(1, Ordering::Release);
    }
}

pub fn current() -> Profile {
    Profile {
        mission_id: MISSION_ID.load(Ordering::Acquire),
        payload_slot: PAYLOAD_SLOT.load(Ordering::Acquire),
        wake_min_secs: WAKE_MIN.load(Ordering::Acquire),
        wake_max_secs: WAKE_MAX.load(Ordering::Acquire),
        pressure_limit_atm: f32::from_bits(PRESSURE_LIMIT_BITS.load(Ordering::Acquire)),
        dose_limit: DOSE_LIMIT.load(Ordering::Acquire),
        leak_rate_atm_s: f32::from_bits(LEAK_RATE_BITS.load(Ordering::Acquire)),
        radio_enable: RADIO_ENABLE.load(Ordering::Acquire) != 0,
        interval_wake: INTERVAL_WAKE.load(Ordering::Acquire) != 0,
        from_sd: LOADED_FROM_SD.load(Ordering::Acquire) != 0,
    }
}

/// Periodic RTC timer wake + scheduled logging (off by default).
pub fn interval_wake_enabled() -> bool {
    INTERVAL_WAKE.load(Ordering::Acquire) != 0
}

/// One-way RF uplink after each cycle (off by default; set via SD profile v2).
pub fn radio_enabled() -> bool {
    RADIO_ENABLE.load(Ordering::Acquire) != 0
}

pub fn mission_id() -> u32 {
    MISSION_ID.load(Ordering::Acquire)
}

pub fn payload_slot() -> u8 {
    PAYLOAD_SLOT.load(Ordering::Acquire)
}

pub fn payload_name(slot: u8) -> &'static str {
    match slot {
        1 => "RELAXED",
        _ => "STRICT",
    }
}

pub fn pressure_limit_atm() -> f32 {
    f32::from_bits(PRESSURE_LIMIT_BITS.load(Ordering::Acquire))
}

pub fn dose_limit() -> u32 {
    DOSE_LIMIT.load(Ordering::Acquire)
}

pub fn leak_rate_atm_s() -> f32 {
    f32::from_bits(LEAK_RATE_BITS.load(Ordering::Acquire))
}

/// Clamp pot-derived wake seconds to profile bounds.
pub fn clamp_wake_secs(secs: u32) -> u32 {
    let min = u32::from(WAKE_MIN.load(Ordering::Acquire));
    let max = u32::from(WAKE_MAX.load(Ordering::Acquire));
    secs.max(min).min(max)
}
