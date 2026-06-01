//! Hardware interrupt vector table (IVT) and minimal-jitter ISRs.
//!
//! ISRs mask nested interrupts (`cli` on x86), record the firing vector, and invoke
//! [`crate::runtime::sovereign_bootstrap`] without scheduler mediation.

use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use crate::runtime;

/// Hardware interrupt vectors (IVT indices / IRQ offsets per platform BSP).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareInterrupt {
    /// Atomic oxygen density below mission threshold (vector `0x20`).
    AtmosphericPressureThreshold = 0x20,
    /// Mechanical deployment joint kinetic pulse (vector `0x21`).
    KineticJointActuation = 0x21,
}

impl HardwareInterrupt {
    /// Decode raw vector byte from IDT entry offset.
    pub fn from_vector(v: u8) -> Option<Self> {
        match v {
            0x20 => Some(Self::AtmosphericPressureThreshold),
            0x21 => Some(Self::KineticJointActuation),
            _ => None,
        }
    }
}

static LAST_VECTOR: AtomicU8 = AtomicU8::new(0);
static WAKE_PENDING: AtomicBool = AtomicBool::new(false);
static BOOTSTRAP_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Last hardware vector latched by the ISR (observability / proof chaining).
#[inline]
pub fn last_vector() -> u8 {
    LAST_VECTOR.load(Ordering::Acquire)
}

/// Returns `true` if an ISR requested a wake but the dormant loop has not yet observed it.
#[inline]
pub fn wake_pending() -> bool {
    WAKE_PENDING.load(Ordering::Acquire)
}

/// Clear wake latch after the dormant core services the event.
#[inline]
pub fn clear_wake() {
    WAKE_PENDING.store(false, Ordering::Release);
}

/// Platform interrupt subsystem initialization (IVT / IDT install).
pub fn init() {
    #[cfg(target_arch = "x86_64")]
    x86_init_idt();
    #[cfg(not(target_arch = "x86_64"))]
    cortex_m_stub_init();
}

/// Mask device interrupts (nested interrupt suppression).
#[inline]
pub fn disable_nested() {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("cli", options(nomem, nostack));
    }
}

/// Unmask interrupts for dormancy wait.
#[inline]
pub fn enable() {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack));
    }
}

/// Enter C1-equivalent halt until the next physical trigger (x86 `hlt`).
#[inline]
pub fn halt_until_interrupt() {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("hlt", options(nomem, nostack));
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        core::hint::spin_loop();
    }
}

/// Common ISR tail — vector dispatch and sovereign handoff.
fn dispatch_isr(vector: u8) {
    disable_nested();
    LAST_VECTOR.store(vector, Ordering::Release);
    WAKE_PENDING.store(true, Ordering::Release);

    if BOOTSTRAP_ACTIVE.swap(true, Ordering::AcqRel) {
        // Re-entrant interrupt during bootstrap: drop to avoid stack blowout.
        return;
    }

    // Hand control directly to WASM bootstrap (spec: no OS scheduler).
    runtime::sovereign_bootstrap(HardwareInterrupt::from_vector(vector));

    BOOTSTRAP_ACTIVE.store(false, Ordering::Release);
}

// ---------------------------------------------------------------------------
// x86_64 IDT
// ---------------------------------------------------------------------------

#[cfg(target_arch = "x86_64")]
mod x86 {
    use super::*;
    use core::sync::atomic::AtomicBool;
    use spin::Once;
    use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

    static IDT_READY: AtomicBool = AtomicBool::new(false);

    /// Platform IDT — `Once` yields a `&'static` table required by [`InterruptDescriptorTable::load`].
    /// A [`spin::Mutex`] guard cannot satisfy that lifetime contract without leaking.
    static IDT: Once<InterruptDescriptorTable> = Once::new();

    pub fn init_idt() {
        let idt = IDT.call_once(|| {
            let mut table = InterruptDescriptorTable::new();
            table[HardwareInterrupt::AtmosphericPressureThreshold as u8]
                .set_handler_fn(isr_atmospheric);
            table[HardwareInterrupt::KineticJointActuation as u8].set_handler_fn(isr_kinetic);
            table
        });
        idt.load();
        IDT_READY.store(true, Ordering::Release);
    }

    extern "x86-interrupt" fn isr_atmospheric(_stack: InterruptStackFrame) {
        dispatch_isr(HardwareInterrupt::AtmosphericPressureThreshold as u8);
    }

    extern "x86-interrupt" fn isr_kinetic(_stack: InterruptStackFrame) {
        dispatch_isr(HardwareInterrupt::KineticJointActuation as u8);
    }

    /// Software-trigger for bench tests (writes vector into latch and runs ISR path).
    pub fn software_trigger(vector: HardwareInterrupt) {
        dispatch_isr(vector as u8);
    }
}

#[cfg(target_arch = "x86_64")]
fn x86_init_idt() {
    x86::init_idt();
}

/// Software IRQ injection for bench / HIL (hardware-in-the-loop) validation.
#[cfg(target_arch = "x86_64")]
pub fn software_trigger(vector: HardwareInterrupt) {
    x86::software_trigger(vector);
}

#[cfg(not(target_arch = "x86_64"))]
fn cortex_m_stub_init() {
    // NVIC vector table would be installed here for Cortex-M flight targets.
}

#[cfg(not(target_arch = "x86_64"))]
pub fn software_trigger(vector: HardwareInterrupt) {
    dispatch_isr(vector as u8);
}
