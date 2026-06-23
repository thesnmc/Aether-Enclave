//! RTC fast RAM state — survives deep sleep, wiped on full power loss.

use core::sync::atomic::{AtomicU32, Ordering};

const MAGIC: u32 = 0xA17E_C001;

/// Layout stored in [`RTC_WORDS`].
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Layout {
    magic: u32,
    cycle: u32,
    last_proof: u64,
    last_pressure_bits: u32,
    wake_secs: u32,
}

#[esp_hal::ram(unstable(rtc_fast, persistent))]
static mut RTC_WORDS: [u32; 6] = [0; 6];

static DOSE_SENSITIVITY: AtomicU32 = AtomicU32::new(1_000);

fn layout() -> Layout {
    unsafe {
        let w = &RTC_WORDS;
        Layout {
            magic: w[0],
            cycle: w[1],
            last_proof: u64::from(w[2]) | (u64::from(w[3]) << 32),
            last_pressure_bits: w[4],
            wake_secs: w[5],
        }
    }
}

fn store(l: Layout) {
    unsafe {
        RTC_WORDS[0] = l.magic;
        RTC_WORDS[1] = l.cycle;
        RTC_WORDS[2] = l.last_proof as u32;
        RTC_WORDS[3] = (l.last_proof >> 32) as u32;
        RTC_WORDS[4] = l.last_pressure_bits;
        RTC_WORDS[5] = l.wake_secs;
    }
}

fn ensure_valid() -> Layout {
    let mut l = layout();
    if l.magic != MAGIC {
        l = Layout {
            magic: MAGIC,
            cycle: 0,
            last_proof: 0,
            last_pressure_bits: 0,
            wake_secs: 10,
        };
        store(l);
    }
    if l.wake_secs == 0 || l.wake_secs > 120 {
        l.wake_secs = 10;
        store(l);
    }
    l
}

/// Mission cycle count (incremented after each completed WASM run).
pub fn cycle_count() -> u32 {
    ensure_valid().cycle
}

/// Seconds for the next RTC timer wake (5–60, set from pot at boot).
pub fn wake_timer_secs() -> u64 {
    u64::from(ensure_valid().wake_secs)
}

/// Previous proof hash from the last completed cycle.
pub fn last_proof() -> u64 {
    ensure_valid().last_proof
}

/// Barometric sample (atm bits) saved before the last sleep.
pub fn last_pressure_bits() -> u32 {
    ensure_valid().last_pressure_bits
}

/// Update pot-derived dose sensitivity (200–2000 maps to guest threshold behavior).
pub fn set_dose_sensitivity(raw_adc: u32) {
    let span = 1_800u32;
    let min = 200u32;
    let sens = min + (raw_adc.min(32_000) * span / 32_000);
    DOSE_SENSITIVITY.store(sens.max(200), Ordering::Release);
}

/// Scale raw ADC counts so the fixed guest limit (1000) tracks the pot.
pub fn scale_dose(raw: u32) -> u32 {
    let sens = DOSE_SENSITIVITY.load(Ordering::Acquire).max(200);
    raw.saturating_mul(1_000) / sens
}

/// Map pot ADC to wake timer seconds (5–60).
pub fn set_wake_timer_from_pot(raw_adc: u32) {
    let mut l = ensure_valid();
    let secs = 5 + (raw_adc.min(32_000) * 55 / 32_000);
    l.wake_secs = secs.max(5);
    store(l);
}

/// Record cycle outcome before sleep.
pub fn record_cycle(proof: u64, pressure_bits: u32) {
    let mut l = ensure_valid();
    l.cycle = l.cycle.saturating_add(1);
    l.last_proof = proof;
    l.last_pressure_bits = pressure_bits;
    store(l);
}
