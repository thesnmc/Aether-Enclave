//! # AETHER-ENCLAVE (Atmospheric-State Execution Module)
//!
//! Ring-0, `#![no_std]` unikernel that boots a bounded WebAssembly diagnostic payload
//! on hardware interrupt, commits a verifiable outcome via MMIO, and returns to
//! absolute zero-power dormancy.
//!
//! ## Safety model
//! - All dynamic allocation is satisfied from a single static bump arena ([`memory`]).
//! - Guest WASM memory is capped at [`memory::SANDBOX_MEMORY_SIZE`] and validated on every host call.
//! - ISRs run with interrupts masked; they delegate to [`runtime::sovereign_bootstrap`] on a
//!   dedicated ISR stack to avoid corrupting the dormant main stack.

#![no_std]
#![warn(missing_docs)]
#![cfg_attr(target_arch = "x86_64", feature(abi_x86_interrupt))]

extern crate alloc;

pub mod interrupts;
pub mod memory;
pub mod mmio;
pub mod runtime;
pub mod shutdown;
pub mod wasm_payload;

/// Re-exported lifecycle entry for integration tests / alternate boot paths.
pub use runtime::sovereign_bootstrap;

/// Re-export COM1 serial helpers.
pub use mmio::{serial_init, serial_write_fmt, SerialPort, COM1_PORT};

/// Write formatted text to COM1 without a trailing newline.
#[macro_export]
macro_rules! serial_print {
    ($($t:tt)*) => {
        $crate::mmio::serial_write_fmt(format_args!($($t)*));
    };
}

/// Write formatted text to COM1 followed by `\r\n`.
#[macro_export]
macro_rules! serial_println {
    () => {
        $crate::mmio::serial_write_fmt(format_args!("\r\n"));
    };
    ($($t:tt)*) => {
        $crate::mmio::serial_write_fmt(format_args!($($t)*));
        $crate::mmio::serial_write_fmt(format_args!("\r\n"));
    };
}
