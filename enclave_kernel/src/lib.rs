//! # AETHER-ENCLAVE kernel (`enclave_kernel`)
//!
//! Ring-0, `#![no_std]` unikernel that boots the `aerospace_payload` WebAssembly module,
//! commits a verifiable outcome via MMIO, and returns to zero-power dormancy.

#![no_std]
#![warn(missing_docs)]
#![cfg_attr(target_arch = "x86_64", feature(abi_x86_interrupt))]

extern crate alloc;

pub mod interrupts;
pub mod memory;
pub mod mmio;
#[cfg(target_arch = "riscv32")]
pub mod platform;
pub mod runtime;
pub mod shutdown;
pub mod wasm_payload;

pub use runtime::sovereign_bootstrap;

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
