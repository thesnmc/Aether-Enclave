//! Wake vectors — x86 bench trigger or ESP32-C6 RTC wake decode.

use core::sync::atomic::{AtomicU8, Ordering};

#[cfg(target_arch = "x86_64")]
use crate::{runtime, serial_println};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareInterrupt {
    AtmosphericPressureThreshold = 0x20,
    KineticJointActuation = 0x21,
}

impl HardwareInterrupt {
    pub fn from_vector(v: u8) -> Option<Self> {
        match v {
            0x20 => Some(Self::AtmosphericPressureThreshold),
            0x21 => Some(Self::KineticJointActuation),
            _ => None,
        }
    }
}

static LAST_VECTOR: AtomicU8 = AtomicU8::new(0);

#[inline]
pub fn latch_vector(vector: u8) {
    LAST_VECTOR.store(vector, Ordering::Release);
}

#[inline]
pub fn last_vector() -> u8 {
    LAST_VECTOR.load(Ordering::Acquire)
}

pub fn init() {}

#[cfg(target_arch = "riscv32")]
pub fn detect_wake_trigger() -> Option<HardwareInterrupt> {
    crate::platform::esp32c6::detect_wake_trigger().inspect(|trigger| {
        LAST_VECTOR.store(*trigger as u8, Ordering::Release);
    })
}

/// QEMU bench: inject IRQ 0x20/0x21 and run one full cycle.
#[cfg(target_arch = "x86_64")]
pub fn software_trigger(vector: HardwareInterrupt) {
    LAST_VECTOR.store(vector as u8, Ordering::Release);
    serial_println!("[AETHER] IRQ 0x{:02X} — bench trigger", vector as u8);
    runtime::sovereign_bootstrap(Some(vector));
}
