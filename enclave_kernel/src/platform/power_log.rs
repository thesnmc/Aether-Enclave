//! Cycle timing for power-budget logging.

use core::sync::atomic::{AtomicU32, Ordering};

use esp_hal::time::Instant;

static CYCLE_START_MS: AtomicU32 = AtomicU32::new(0);

fn now_ms() -> u32 {
    Instant::now().duration_since_epoch().as_millis() as u32
}

/// Mark the start of an active WASM cycle.
pub fn mark_cycle_start() {
    CYCLE_START_MS.store(now_ms(), Ordering::Release);
}

/// Milliseconds since [`mark_cycle_start`], or 0 if not started.
pub fn cycle_active_ms() -> u32 {
    let start = CYCLE_START_MS.load(Ordering::Acquire);
    if start == 0 {
        return 0;
    }
    now_ms().wrapping_sub(start)
}

/// Log sleep budget and last active window (deep-sleep µA measured on PCB).
pub fn log_power_budget(wake_secs: u64) {
    crate::serial_println!(
        "[AETHER] power — active_last={}ms sleep_next={}s (C6 deep-sleep ~10–30µA; measure on your PCB)",
        cycle_active_ms(),
        wake_secs,
    );
}
