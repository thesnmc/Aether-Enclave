//! AETHER-ENCLAVE bare-metal binary entry (`_start`) and dormancy lifecycle.
//!
//! ```text
//! Deep Dormancy (HLT)
//!   -> Physical IRQ (0x20 / 0x21)
//!   -> ISR (cli, sovereign_bootstrap)
//!   -> WASM diagnostic + MMIO proof
//!   -> self_annihilate (zero, PMU sleep)
//! ```

#![no_std]
#![no_main]
use core::panic::PanicInfo;

use aether_enclave::{interrupts, memory, runtime, shutdown};

/// Bare-metal entry — no Rust `main`, no libc `_start`.
#[no_mangle]
pub extern "C" fn _start() -> ! {
    memory::reset_arena();
    interrupts::init();

    dormancy_loop();
}

/// Absolute zero-power wait — processor halted until IRQ.
fn dormancy_loop() -> ! {
    loop {
        interrupts::enable();
        interrupts::halt_until_interrupt();

        // If bootstrap did not consume the full cycle (spurious wake), observe latch.
        if interrupts::wake_pending() {
            interrupts::clear_wake();
            runtime::sovereign_bootstrap(interrupts::HardwareInterrupt::from_vector(
                interrupts::last_vector(),
            ));
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    shutdown::self_annihilate(shutdown::ShutdownReport {
        guest_result: -1,
        proof: 0,
        vector: interrupts::last_vector(),
    });
}

/// Required on some bare-metal targets when unwinding is disabled.
#[no_mangle]
extern "C" fn eh_personality() {}
