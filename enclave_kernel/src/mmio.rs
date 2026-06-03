//! Memory-mapped I/O register map for sensor ingress, uplink commit, and serial logging.
//!
//! On x86_64 this layer simulates flight registers with atomics. On ESP32-C3 it delegates
//! sensor reads to the BMP390 / ADS1115 drivers while preserving the logical MMIO contract.

use core::fmt::{self, Write};
use core::sync::atomic::{AtomicU32, Ordering};
use spin::Mutex;

// ---------------------------------------------------------------------------
// Flight / logical MMIO register addresses (stable across targets)
// ---------------------------------------------------------------------------

/// Simulated atomic-oxygen density sensor (raw ADC counts).
pub const REG_ATOMIC_O2_SENSOR: usize = 0xFEF0_0000;

/// Simulated kinetic joint strain gauge.
pub const REG_KINETIC_JOINT: usize = 0xFEF0_0004;

/// Barometric / atmospheric pressure sensor (IEEE-754 `f32` bit pattern in a 32-bit word).
pub const REG_ATMOSPHERIC_PRESSURE: usize = 0xFEF0_0008;

/// Cumulative radiation dosimeter (millirad-equivalent counts).
pub const REG_RADIATION_DOSIMETER: usize = 0xFEF0_000C;

/// Uplink commit: low word = proof digest, high word = sequence / status flags.
pub const REG_UPLINK_COMMIT_LO: usize = 0xFEF0_0010;
/// High word of uplink commit register pair.
pub const REG_UPLINK_COMMIT_HI: usize = 0xFEF0_0014;

/// Power-management unit command register.
pub const REG_PMU_COMMAND: usize = 0xFEF0_0020;
/// PMU: enter hard dormancy.
pub const PMU_CMD_DORMANT: u32 = 0x0000_0001;
/// PMU: request hard reset.
pub const PMU_CMD_HARD_RESET: u32 = 0xDEAD_0002;

/// Maximum guest telemetry blob accepted by [`commit_telemetry_vector`].
pub const TELEMETRY_VECTOR_CAP: usize = 64;

static LIVE_O2: AtomicU32 = AtomicU32::new(0x0000_4000);
static LIVE_KINETIC: AtomicU32 = AtomicU32::new(0);
static LIVE_ATM_PRESSURE_BITS: AtomicU32 = AtomicU32::new(f32::to_bits(0.21));
static LIVE_RADIATION: AtomicU32 = AtomicU32::new(450);
static LIVE_COMMIT_LO: AtomicU32 = AtomicU32::new(0);
static LIVE_COMMIT_HI: AtomicU32 = AtomicU32::new(0);
static LIVE_PMU: AtomicU32 = AtomicU32::new(0);
static LIVE_TELEMETRY_LEN: core::sync::atomic::AtomicUsize =
    core::sync::atomic::AtomicUsize::new(0);
static LIVE_TELEMETRY: Mutex<[u8; TELEMETRY_VECTOR_CAP]> = Mutex::new([0; TELEMETRY_VECTOR_CAP]);

/// Inject simulated bench sensor readings (x86 QEMU harness only).
#[cfg(target_arch = "x86_64")]
pub fn sim_inject_o2_drop() {
    LIVE_O2.store(0x0000_0100, Ordering::Release);
    LIVE_ATM_PRESSURE_BITS.store(f32::to_bits(0.12), Ordering::Release);
    LIVE_RADIATION.store(1250, Ordering::Release);
}

/// Inject simulated bench sensor readings (x86 QEMU harness only).
#[cfg(not(target_arch = "x86_64"))]
pub fn sim_inject_o2_drop() {}

/// Inject a simulated kinetic joint trigger (x86 bench).
#[cfg(target_arch = "x86_64")]
pub fn sim_inject_kinetic_pulse() {
    LIVE_KINETIC.store(0x0000_8000, Ordering::Release);
}

/// Inject a simulated kinetic joint trigger (x86 bench).
#[cfg(not(target_arch = "x86_64"))]
pub fn sim_inject_kinetic_pulse() {}

/// Read the atomic oxygen sensor via the logical MMIO map.
#[inline]
pub fn read_atomic_o2() -> u32 {
    read_reg_u32(REG_ATOMIC_O2_SENSOR)
}

/// Read kinetic joint sensor.
#[inline]
pub fn read_kinetic_joint() -> u32 {
    read_reg_u32(REG_KINETIC_JOINT)
}

/// Read atmospheric pressure in atm.
#[inline]
pub fn read_atmospheric_pressure() -> f32 {
    #[cfg(target_arch = "riscv32")]
    {
        let pressure = crate::platform::esp32c3::read_bmp390_pressure();
        LIVE_ATM_PRESSURE_BITS.store(pressure.to_bits(), Ordering::Release);
        return pressure;
    }
    #[cfg(not(target_arch = "riscv32"))]
    {
        f32::from_bits(read_reg_u32(REG_ATMOSPHERIC_PRESSURE))
    }
}

/// Read radiation dosimeter counts.
#[inline]
pub fn read_radiation_dosimeter() -> u32 {
    #[cfg(target_arch = "riscv32")]
    {
        let dose = crate::platform::esp32c3::read_ads1115_dose();
        LIVE_RADIATION.store(dose, Ordering::Release);
        return dose;
    }
    #[cfg(not(target_arch = "riscv32"))]
    {
        read_reg_u32(REG_RADIATION_DOSIMETER)
    }
}

/// Copy guest telemetry vector into the MMIO staging buffer.
pub fn commit_telemetry_vector(data: &[u8]) -> usize {
    let len = data.len().min(TELEMETRY_VECTOR_CAP);
    let mut buf = LIVE_TELEMETRY.lock();
    buf[..len].fill(0);
    buf[..len].copy_from_slice(&data[..len]);
    LIVE_TELEMETRY_LEN.store(len, Ordering::Release);
    len
}

/// Read back last committed telemetry.
pub fn last_telemetry_vector() -> ([u8; TELEMETRY_VECTOR_CAP], usize) {
    let len = LIVE_TELEMETRY_LEN.load(Ordering::Acquire);
    let buf = LIVE_TELEMETRY.lock();
    (*buf, len)
}

/// Commit proof words to uplink MMIO and return the combined 64-bit digest view.
pub fn commit_proof(proof_lo: u32, proof_hi: u32) -> u64 {
    write_reg_u32(REG_UPLINK_COMMIT_LO, proof_lo);
    write_reg_u32(REG_UPLINK_COMMIT_HI, proof_hi);
    (u64::from(proof_hi) << 32) | u64::from(proof_lo)
}

/// Last committed proof (readback).
pub fn last_committed_proof() -> u64 {
    let lo = LIVE_COMMIT_LO.load(Ordering::Acquire);
    let hi = LIVE_COMMIT_HI.load(Ordering::Acquire);
    (u64::from(hi) << 32) | u64::from(lo)
}

/// Issue PMU dormancy command (logical register latch).
pub fn request_dormancy() {
    write_reg_u32(REG_PMU_COMMAND, PMU_CMD_DORMANT);
}

/// Issue hard reset command (logical register latch).
pub fn request_hard_reset() {
    write_reg_u32(REG_PMU_COMMAND, PMU_CMD_HARD_RESET);
}

#[inline]
fn read_reg_u32(addr: usize) -> u32 {
    unsafe { read_volatile_u32(addr) }
}

#[inline]
unsafe fn read_volatile_u32(addr: usize) -> u32 {
    match addr {
        REG_ATOMIC_O2_SENSOR => LIVE_O2.load(Ordering::Acquire),
        REG_KINETIC_JOINT => LIVE_KINETIC.load(Ordering::Acquire),
        REG_ATMOSPHERIC_PRESSURE => LIVE_ATM_PRESSURE_BITS.load(Ordering::Acquire),
        REG_RADIATION_DOSIMETER => LIVE_RADIATION.load(Ordering::Acquire),
        REG_UPLINK_COMMIT_LO => LIVE_COMMIT_LO.load(Ordering::Acquire),
        REG_UPLINK_COMMIT_HI => LIVE_COMMIT_HI.load(Ordering::Acquire),
        REG_PMU_COMMAND => LIVE_PMU.load(Ordering::Acquire),
        _ => unsafe { core::ptr::read_volatile(addr as *const u32) },
    }
}

#[inline]
fn write_reg_u32(addr: usize, value: u32) {
    unsafe { write_volatile_u32(addr, value) }
}

#[inline]
unsafe fn write_volatile_u32(addr: usize, value: u32) {
    match addr {
        REG_ATOMIC_O2_SENSOR => LIVE_O2.store(value, Ordering::Release),
        REG_KINETIC_JOINT => LIVE_KINETIC.store(value, Ordering::Release),
        REG_ATMOSPHERIC_PRESSURE => LIVE_ATM_PRESSURE_BITS.store(value, Ordering::Release),
        REG_RADIATION_DOSIMETER => LIVE_RADIATION.store(value, Ordering::Release),
        REG_UPLINK_COMMIT_LO => LIVE_COMMIT_LO.store(value, Ordering::Release),
        REG_UPLINK_COMMIT_HI => LIVE_COMMIT_HI.store(value, Ordering::Release),
        REG_PMU_COMMAND => LIVE_PMU.store(value, Ordering::Release),
        _ => unsafe { core::ptr::write_volatile(addr as *mut u32, value) },
    }
}

// ---------------------------------------------------------------------------
// Serial logging
// ---------------------------------------------------------------------------

/// Initialize the platform console (COM1 on x86, UART0 on ESP32-C3).
pub fn serial_init() {
    #[cfg(target_arch = "x86_64")]
    x86_serial::serial_init();
}

/// Format and emit text on the platform console.
pub fn serial_write_fmt(args: fmt::Arguments<'_>) {
    #[cfg(target_arch = "x86_64")]
    {
        x86_serial::serial_write_fmt(args);
    }
    #[cfg(target_arch = "riscv32")]
    {
        crate::platform::esp32c3::serial_write_fmt(args);
    }
}

// ---------------------------------------------------------------------------
// x86 COM1 UART
// ---------------------------------------------------------------------------

#[cfg(target_arch = "x86_64")]
mod x86_serial {
    use super::*;
    use core::fmt;

    /// COM1 base I/O port (UART channel 0).
    pub const COM1_PORT: u16 = 0x3F8;

    const COM1_DATA: u16 = COM1_PORT;
    const COM1_INT_ENABLE: u16 = COM1_PORT + 1;
    const COM1_FIFO_CTRL: u16 = COM1_PORT + 2;
    const COM1_LINE_CTRL: u16 = COM1_PORT + 3;
    const COM1_MODEM_CTRL: u16 = COM1_PORT + 4;
    const COM1_LINE_STATUS: u16 = COM1_PORT + 5;

    const LSR_THR_EMPTY: u8 = 0x20;
    const BAUD_DIVISOR_LO: u8 = 0x01;
    const BAUD_DIVISOR_HI: u8 = 0x00;

    static SERIAL: Mutex<SerialPort> = Mutex::new(SerialPort::new());

    pub(super) fn serial_init() {
        SERIAL.lock().init();
    }

    pub(super) fn serial_write_fmt(args: fmt::Arguments<'_>) {
        let _ = SERIAL.lock().write_fmt(args);
    }

    /// Bare-metal UART backed by COM1.
    pub struct SerialPort {
        base: u16,
    }

    impl SerialPort {
        /// COM1 at the standard PC port base.
        pub const fn new() -> Self {
            Self { base: COM1_PORT }
        }

        /// Program 8N1 @ 115200 and enable FIFOs.
        pub fn init(&mut self) {
            self.outb(COM1_INT_ENABLE, 0x00);
            self.outb(COM1_LINE_CTRL, 0x80);
            self.outb(COM1_DATA, BAUD_DIVISOR_LO);
            self.outb(COM1_INT_ENABLE, BAUD_DIVISOR_HI);
            self.outb(COM1_LINE_CTRL, 0x03);
            self.outb(COM1_FIFO_CTRL, 0xC7);
            self.outb(COM1_MODEM_CTRL, 0x0B);
        }

        /// Write one raw byte, spinning until the THR is empty.
        pub fn write_byte(&mut self, byte: u8) {
            while (self.inb(COM1_LINE_STATUS) & LSR_THR_EMPTY) == 0 {
                core::hint::spin_loop();
            }
            self.outb(COM1_DATA, byte);
        }

        /// Write a byte slice to the UART.
        pub fn write_bytes(&mut self, bytes: &[u8]) {
            for &b in bytes {
                self.write_byte(b);
            }
        }

        #[inline]
        fn outb(&self, port: u16, value: u8) {
            unsafe {
                core::arch::asm!(
                    "out dx, al",
                    in("dx") port,
                    in("al") value,
                    options(nomem, nostack, preserves_flags)
                );
            }
        }

        #[inline]
        fn inb(&self, port: u16) -> u8 {
            let value: u8;
            unsafe {
                core::arch::asm!(
                    "in al, dx",
                    out("al") value,
                    in("dx") port,
                    options(nomem, nostack, preserves_flags)
                );
            }
            value
        }
    }

    impl Write for SerialPort {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            self.write_bytes(s.as_bytes());
            Ok(())
        }
    }
}

#[cfg(target_arch = "x86_64")]
pub use x86_serial::{SerialPort, COM1_PORT};
