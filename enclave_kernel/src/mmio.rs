//! Memory-mapped I/O register map for sensor ingress, uplink commit, and COM1 UART.
//!
//! Sensor/uplink addresses are board placeholders; the UART uses standard x86 I/O ports.

use core::fmt::{self, Write};
use core::sync::atomic::{AtomicU32, Ordering};
use spin::Mutex;

// ---------------------------------------------------------------------------
// Flight / simulation MMIO (memory-mapped peripherals)
// ---------------------------------------------------------------------------

/// Simulated atomic-oxygen density sensor (raw ADC counts).
pub const REG_ATOMIC_O2_SENSOR: usize = 0xFEF0_0000;

/// Simulated kinetic joint strain gauge.
pub const REG_KINETIC_JOINT: usize = 0xFEF0_0004;

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

static SIM_O2: AtomicU32 = AtomicU32::new(0x0000_4000);
static SIM_KINETIC: AtomicU32 = AtomicU32::new(0);
static SIM_COMMIT_LO: AtomicU32 = AtomicU32::new(0);
static SIM_COMMIT_HI: AtomicU32 = AtomicU32::new(0);
static SIM_PMU: AtomicU32 = AtomicU32::new(0);

/// Inject a simulated atmospheric trigger (test / pre-flight harness).
pub fn sim_inject_o2_drop() {
    SIM_O2.store(0x0000_0100, Ordering::Release);
}

/// Inject a simulated kinetic joint trigger.
pub fn sim_inject_kinetic_pulse() {
    SIM_KINETIC.store(0x0000_8000, Ordering::Release);
}

/// Read the atomic oxygen sensor via MMIO (simulation uses atomic backing).
#[inline]
pub fn read_atomic_o2() -> u32 {
    unsafe { read_volatile_u32(REG_ATOMIC_O2_SENSOR) }
}

/// Read kinetic joint sensor.
#[inline]
pub fn read_kinetic_joint() -> u32 {
    unsafe { read_volatile_u32(REG_KINETIC_JOINT) }
}

/// Commit proof words to uplink MMIO and return the combined 64-bit digest view.
pub fn commit_proof(proof_lo: u32, proof_hi: u32) -> u64 {
    unsafe {
        write_volatile_u32(REG_UPLINK_COMMIT_LO, proof_lo);
        write_volatile_u32(REG_UPLINK_COMMIT_HI, proof_hi);
    }
    (u64::from(proof_hi) << 32) | u64::from(proof_lo)
}

/// Last committed proof (simulation readback).
pub fn last_committed_proof() -> u64 {
    let lo = SIM_COMMIT_LO.load(Ordering::Acquire);
    let hi = SIM_COMMIT_HI.load(Ordering::Acquire);
    (u64::from(hi) << 32) | u64::from(lo)
}

/// Issue PMU dormancy command.
pub fn request_dormancy() {
    unsafe {
        write_volatile_u32(REG_PMU_COMMAND, PMU_CMD_DORMANT);
    }
}

/// Issue hard reset (self-annihilation path).
pub fn request_hard_reset() {
    unsafe {
        write_volatile_u32(REG_PMU_COMMAND, PMU_CMD_HARD_RESET);
    }
}

#[inline]
unsafe fn read_volatile_u32(addr: usize) -> u32 {
    match addr {
        REG_ATOMIC_O2_SENSOR => SIM_O2.load(Ordering::Acquire),
        REG_KINETIC_JOINT => SIM_KINETIC.load(Ordering::Acquire),
        REG_UPLINK_COMMIT_LO => SIM_COMMIT_LO.load(Ordering::Acquire),
        REG_UPLINK_COMMIT_HI => SIM_COMMIT_HI.load(Ordering::Acquire),
        REG_PMU_COMMAND => SIM_PMU.load(Ordering::Acquire),
        _ => unsafe { core::ptr::read_volatile(addr as *const u32) },
    }
}

#[inline]
unsafe fn write_volatile_u32(addr: usize, value: u32) {
    match addr {
        REG_ATOMIC_O2_SENSOR => SIM_O2.store(value, Ordering::Release),
        REG_KINETIC_JOINT => SIM_KINETIC.store(value, Ordering::Release),
        REG_UPLINK_COMMIT_LO => SIM_COMMIT_LO.store(value, Ordering::Release),
        REG_UPLINK_COMMIT_HI => SIM_COMMIT_HI.store(value, Ordering::Release),
        REG_PMU_COMMAND => SIM_PMU.store(value, Ordering::Release),
        _ => unsafe { core::ptr::write_volatile(addr as *mut u32, value) },
    }
}

// ---------------------------------------------------------------------------
// x86 COM1 UART (I/O port 0x3F8) — bare-metal serial logger
// ---------------------------------------------------------------------------

/// COM1 base I/O port (UART channel 0).
pub const COM1_PORT: u16 = 0x3F8;

/// Transmitter holding register / receive buffer (offset 0).
pub const COM1_DATA: u16 = COM1_PORT;
/// Interrupt-enable register (offset 1).
pub const COM1_INT_ENABLE: u16 = COM1_PORT + 1;
/// FIFO control register (offset 2).
pub const COM1_FIFO_CTRL: u16 = COM1_PORT + 2;
/// Line control register (offset 3).
pub const COM1_LINE_CTRL: u16 = COM1_PORT + 3;
/// Modem control register (offset 4).
pub const COM1_MODEM_CTRL: u16 = COM1_PORT + 4;
/// Line status register (offset 5).
pub const COM1_LINE_STATUS: u16 = COM1_PORT + 5;

/// LSR bit: transmitter holding register empty (ready for next byte).
const LSR_THR_EMPTY: u8 = 0x20;

/// Divisor for 115200 baud when UART clock is 1.8432 MHz (divisor = 1).
const BAUD_DIVISOR_LO: u8 = 0x01;
const BAUD_DIVISOR_HI: u8 = 0x00;

/// Thread-safe COM1 writer (single-core unikernel; `Mutex` suppresses reentrancy races).
static SERIAL: Mutex<SerialPort> = Mutex::new(SerialPort::new());

/// Bare-metal UART backed by COM1.
pub struct SerialPort {
    base: u16,
}

impl SerialPort {
    /// COM1 at the standard PC port base.
    pub const fn new() -> Self {
        Self { base: COM1_PORT }
    }

    /// Program 8N1 @ 115200 and enable FIFOs (safe before `sti` on x86 bring-up).
    pub fn init(&mut self) {
        #[cfg(target_arch = "x86_64")]
        {
            self.outb(COM1_INT_ENABLE, 0x00);
            self.outb(COM1_LINE_CTRL, 0x80);
            self.outb(COM1_DATA, BAUD_DIVISOR_LO);
            self.outb(COM1_INT_ENABLE, BAUD_DIVISOR_HI);
            self.outb(COM1_LINE_CTRL, 0x03);
            self.outb(COM1_FIFO_CTRL, 0xC7);
            self.outb(COM1_MODEM_CTRL, 0x0B);
        }
        let _ = self.base;
    }

    /// Write one raw byte, spinning until the THR is empty.
    pub fn write_byte(&mut self, byte: u8) {
        #[cfg(target_arch = "x86_64")]
        {
            while (self.inb(COM1_LINE_STATUS) & LSR_THR_EMPTY) == 0 {
                core::hint::spin_loop();
            }
            self.outb(COM1_DATA, byte);
        }
        #[cfg(not(target_arch = "x86_64"))]
        let _ = byte;
    }

    /// Write a byte slice to the UART.
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.write_byte(b);
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[inline]
    fn outb(&self, port: u16, value: u8) {
        // SAFETY: `port` is a valid x86 I/O address for COM1 register file.
        unsafe {
            core::arch::asm!(
                "out dx, al",
                in("dx") port,
                in("al") value,
                options(nomem, nostack, preserves_flags)
            );
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[inline]
    fn inb(&self, port: u16) -> u8 {
        let value: u8;
        // SAFETY: `port` is a valid x86 I/O read port (line status).
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

/// Initialize COM1 (call once from `_start` before logging).
pub fn serial_init() {
    SERIAL.lock().init();
}

/// Format and emit text on the serial port (used by `serial_print!` / `serial_println!`).
pub fn serial_write_fmt(args: fmt::Arguments<'_>) {
    let _ = SERIAL.lock().write_fmt(args);
}
