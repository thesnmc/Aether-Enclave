//! Platform-specific hardware backends.

#[cfg(target_arch = "riscv32")]
pub mod demo;
#[cfg(target_arch = "riscv32")]
pub mod event_browser;
#[cfg(target_arch = "riscv32")]
pub mod event_log;
#[cfg(target_arch = "riscv32")]
pub mod esp32c6;
#[cfg(target_arch = "riscv32")]
mod font5x7;
#[cfg(target_arch = "riscv32")]
pub mod mission_profile;
#[cfg(target_arch = "riscv32")]
mod radio;
#[cfg(target_arch = "riscv32")]
pub mod radio_emit;
#[cfg(target_arch = "riscv32")]
pub mod oled;
#[cfg(target_arch = "riscv32")]
pub mod uplink;
#[cfg(target_arch = "riscv32")]
pub mod power_log;
#[cfg(target_arch = "riscv32")]
pub mod rtc_state;
#[cfg(target_arch = "riscv32")]
pub mod sd_log;
