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
    last_cycle_ms: u32,
}

#[esp_hal::ram(unstable(rtc_fast, persistent))]
static mut RTC_WORDS: [u32; 7] = [0; 7];

static DOSE_SENSITIVITY: AtomicU32 = AtomicU32::new(1_000);

fn read_word(i: usize) -> u32 {
    unsafe { core::ptr::read_volatile(core::ptr::addr_of!(RTC_WORDS).cast::<u32>().add(i)) }
}

fn store(l: Layout) {
    let words = [
        l.magic,
        l.cycle,
        l.last_proof as u32,
        (l.last_proof >> 32) as u32,
        l.last_pressure_bits,
        l.wake_secs,
        l.last_cycle_ms,
    ];
    unsafe {
        let base = core::ptr::addr_of_mut!(RTC_WORDS).cast::<u32>();
        for (i, w) in words.iter().enumerate() {
            core::ptr::write_volatile(base.add(i), *w);
        }
    }
}

fn layout() -> Layout {
    Layout {
        magic: read_word(0),
        cycle: read_word(1),
        last_proof: u64::from(read_word(2)) | (u64::from(read_word(3)) << 32),
        last_pressure_bits: read_word(4),
        wake_secs: read_word(5),
        last_cycle_ms: read_word(6),
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
            last_cycle_ms: 0,
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

/// Seconds for the next RTC timer wake (profile-clamped, set from pot at boot).
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

/// Epoch milliseconds when the last cycle completed (for leak-rate detection).
pub fn last_cycle_ms() -> u32 {
    ensure_valid().last_cycle_ms
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

/// Map pot ADC to wake timer seconds, clamped to mission profile bounds.
pub fn set_wake_timer_from_pot(raw_adc: u32) {
    let mut l = ensure_valid();
    let secs = 5 + (raw_adc.min(32_000) * 55 / 32_000);
    l.wake_secs = super::mission_profile::clamp_wake_secs(secs.max(5));
    store(l);
}

/// Record cycle outcome before sleep.
pub fn record_cycle(proof: u64, pressure_bits: u32, cycle_ms: u32) {
    let mut l = ensure_valid();
    l.cycle = l.cycle.saturating_add(1);
    l.last_proof = proof;
    l.last_pressure_bits = pressure_bits;
    l.last_cycle_ms = cycle_ms;
    store(l);
}
