//! # AETHER-ENCLAVE kernel (`enclave_kernel`)
//!
//! `#![no_std]` bare-metal host that loads the `aerospace_payload` WebAssembly module,
//! writes a proof hash to MMIO, wipes memory, and returns to sleep.

#![no_std]

extern crate alloc;

pub mod interrupts;
pub mod memory;
pub mod mmio;
#[cfg(target_arch = "riscv32")]
pub mod platform;
pub mod proof;
pub mod runtime;
pub mod shutdown;
pub mod wasm_payload;

pub use runtime::{run_mission_cycle, sovereign_bootstrap};

#[cfg(target_arch = "x86_64")]
pub use mmio::{serial_init, serial_write_fmt, SerialPort, COM1_PORT};

#[macro_export]
macro_rules! serial_print {
    ($($t:tt)*) => {
        $crate::mmio::serial_write_fmt(format_args!($($t)*));
    };
}

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
