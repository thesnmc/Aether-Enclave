//! Logical MMIO map for sensor reads, proof commit, and serial logging.

use core::fmt;
use core::sync::atomic::{AtomicU32, Ordering};
use spin::Mutex;

pub const REG_ATMOSPHERIC_PRESSURE: usize = 0xFEF0_0008;
pub const REG_RADIATION_DOSIMETER: usize = 0xFEF0_000C;
pub const REG_UPLINK_COMMIT_LO: usize = 0xFEF0_0010;
pub const REG_UPLINK_COMMIT_HI: usize = 0xFEF0_0014;
pub const REG_PMU_COMMAND: usize = 0xFEF0_0020;
pub const PMU_CMD_DORMANT: u32 = 0x0000_0001;

pub const TELEMETRY_VECTOR_CAP: usize = 64;

static LIVE_ATM_PRESSURE_BITS: AtomicU32 = AtomicU32::new(f32::to_bits(0.21));
static LIVE_RADIATION: AtomicU32 = AtomicU32::new(450);
static LIVE_COMMIT_LO: AtomicU32 = AtomicU32::new(0);
static LIVE_COMMIT_HI: AtomicU32 = AtomicU32::new(0);
static LIVE_PMU: AtomicU32 = AtomicU32::new(0);
static LIVE_TELEMETRY_LEN: core::sync::atomic::AtomicUsize =
    core::sync::atomic::AtomicUsize::new(0);
static LIVE_TELEMETRY: Mutex<[u8; TELEMETRY_VECTOR_CAP]> = Mutex::new([0; TELEMETRY_VECTOR_CAP]);

/// Inject bench sensor readings (QEMU only).
#[cfg(target_arch = "x86_64")]
pub fn sim_inject_o2_drop() {
    LIVE_ATM_PRESSURE_BITS.store(f32::to_bits(0.12), Ordering::Release);
    LIVE_RADIATION.store(1250, Ordering::Release);
}

#[cfg(not(target_arch = "x86_64"))]
pub fn sim_inject_o2_drop() {}

pub fn read_atmospheric_pressure() -> f32 {
    #[cfg(target_arch = "riscv32")]
    {
        let pressure = crate::platform::esp32c6::read_bmp390_pressure();
        LIVE_ATM_PRESSURE_BITS.store(pressure.to_bits(), Ordering::Release);
        return pressure;
    }
    #[cfg(not(target_arch = "riscv32"))]
    {
        f32::from_bits(LIVE_ATM_PRESSURE_BITS.load(Ordering::Acquire))
    }
}

pub fn read_radiation_dosimeter() -> u32 {
    #[cfg(target_arch = "riscv32")]
    {
        let dose = crate::platform::esp32c6::read_ads1115_dose();
        LIVE_RADIATION.store(dose, Ordering::Release);
        return dose;
    }
    #[cfg(not(target_arch = "riscv32"))]
    {
        LIVE_RADIATION.load(Ordering::Acquire)
    }
}

pub fn commit_telemetry_vector(data: &[u8]) -> usize {
    let len = data.len().min(TELEMETRY_VECTOR_CAP);
    let mut buf = LIVE_TELEMETRY.lock();
    buf[..len].fill(0);
    buf[..len].copy_from_slice(&data[..len]);
    LIVE_TELEMETRY_LEN.store(len, Ordering::Release);
    len
}

pub fn commit_proof(proof_lo: u32, proof_hi: u32) -> u64 {
    LIVE_COMMIT_LO.store(proof_lo, Ordering::Release);
    LIVE_COMMIT_HI.store(proof_hi, Ordering::Release);
    (u64::from(proof_hi) << 32) | u64::from(proof_lo)
}

pub fn request_dormancy() {
    LIVE_PMU.store(PMU_CMD_DORMANT, Ordering::Release);
}

pub fn serial_init() {
    #[cfg(target_arch = "x86_64")]
    x86_serial::serial_init();
}

pub fn serial_write_fmt(args: fmt::Arguments<'_>) {
    #[cfg(target_arch = "x86_64")]
    {
        x86_serial::serial_write_fmt(args);
    }
    #[cfg(target_arch = "riscv32")]
    {
        use core::fmt::Write;
        let mut printer = esp_println::Printer;
        let _ = printer.write_fmt(args);
    }
}

#[cfg(target_arch = "x86_64")]
mod x86_serial {
    use core::fmt::{self, Write};
    use spin::Mutex;

    pub const COM1_PORT: u16 = 0x3F8;

    const COM1_DATA: u16 = COM1_PORT;
    const COM1_INT_ENABLE: u16 = COM1_PORT + 1;
    const COM1_FIFO_CTRL: u16 = COM1_PORT + 2;
    const COM1_LINE_CTRL: u16 = COM1_PORT + 3;
    const COM1_MODEM_CTRL: u16 = COM1_PORT + 4;
    const COM1_LINE_STATUS: u16 = COM1_PORT + 5;

    const LSR_THR_EMPTY: u8 = 0x20;

    static SERIAL: Mutex<SerialPort> = Mutex::new(SerialPort::new());

    pub(super) fn serial_init() {
        SERIAL.lock().init();
    }

    pub(super) fn serial_write_fmt(args: fmt::Arguments<'_>) {
        let _ = SERIAL.lock().write_fmt(args);
    }

    pub struct SerialPort {
        base: u16,
    }

    impl SerialPort {
        pub const fn new() -> Self {
            Self { base: COM1_PORT }
        }

        pub fn init(&mut self) {
            self.outb(COM1_INT_ENABLE, 0x00);
            self.outb(COM1_LINE_CTRL, 0x80);
            self.outb(COM1_DATA, 0x01);
            self.outb(COM1_INT_ENABLE, 0x00);
            self.outb(COM1_LINE_CTRL, 0x03);
            self.outb(COM1_FIFO_CTRL, 0xC7);
            self.outb(COM1_MODEM_CTRL, 0x0B);
        }

        pub fn write_byte(&mut self, byte: u8) {
            while (self.inb(COM1_LINE_STATUS) & LSR_THR_EMPTY) == 0 {
                core::hint::spin_loop();
            }
            self.outb(COM1_DATA, byte);
        }

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
            for &b in s.as_bytes() {
                self.write_byte(b);
            }
            Ok(())
        }
    }
}

#[cfg(target_arch = "x86_64")]
pub use x86_serial::{SerialPort, COM1_PORT};
