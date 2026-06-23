//! Platform-specific hardware backends.

#[cfg(target_arch = "riscv32")]
pub mod demo;
#[cfg(target_arch = "riscv32")]
pub mod esp32c6;
#[cfg(target_arch = "riscv32")]
mod font5x7;
#[cfg(target_arch = "riscv32")]
pub mod oled;
#[cfg(target_arch = "riscv32")]
pub mod rtc_state;
