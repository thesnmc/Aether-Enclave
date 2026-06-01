//! Sovereign diagnostic WebAssembly payload (`wasm32-unknown-unknown`, `#![no_std]`).
//!
//! Imports the `aether` host API (must match `enclave_kernel` linker definitions exactly).

#![no_std]

/// Wasm import module name — must match `linker.define("…", …)` in the kernel.
pub const HOST_IMPORT_MODULE: &str = "aether";

/// Flight-software payload identifier.
pub const PAYLOAD_MAGIC: u32 = 0xA17E_0001;

/// Minimum acceptable atmospheric partial pressure (atm) before alarm.
pub const PRESSURE_LIMIT_ATM: f32 = 0.15;

/// Maximum acceptable radiation dose (simulated millirad-equivalent counts).
pub const DOSE_LIMIT: u32 = 1_000;

/// Telemetry record committed via [`commit_telemetry_vector`].
#[repr(C)]
pub struct TelemetryRecord {
    /// Bit flags: `0x1` pressure low, `0x2` dose high.
    pub flags: u8,
    pub _pad: [u8; 3],
    /// Raw pressure sample (`f32::to_bits`).
    pub pressure_bits: u32,
    /// Dosimeter reading.
    pub dose: u32,
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

/// Host syscall imports — signatures must mirror Wasm valtypes (`f32` = `0x7D`, `i32` = `0x7F`).
#[link(wasm_import_module = "aether")]
extern "C" {
    /// `() -> f32` in the guest module type section.
    fn read_atmospheric_pressure() -> f32;
    /// `() -> i32` in the guest module (Rust `u32` would mismatch the linker).
    fn read_radiation_dosimeter() -> i32;
    /// `(i32, i32) -> ()` — pointer/length into guest linear memory.
    fn commit_telemetry_vector(ptr: i32, len: i32);
    /// `(i32, i32) -> ()` — legacy uplink proof commit.
    fn commit_uplink(proof_lo: i32, proof_hi: i32);
}

/// Evaluate pressure + radiation limits, commit telemetry vector, return status flags.
#[no_mangle]
pub extern "C" fn evaluate_limits() -> i32 {
    let pressure = unsafe { read_atmospheric_pressure() };
    let dose = unsafe { read_radiation_dosimeter() } as u32;

    let mut flags: u8 = 0;
    if pressure < PRESSURE_LIMIT_ATM {
        flags |= 0x1;
    }
    if dose > DOSE_LIMIT {
        flags |= 0x2;
    }

    // Static storage guarantees a stable address in wasm linear memory for the host copy.
    static mut TELEMETRY: TelemetryRecord = TelemetryRecord {
        flags: 0,
        _pad: [0, 0, 0],
        pressure_bits: 0,
        dose: 0,
    };

    unsafe {
        *core::ptr::addr_of_mut!(TELEMETRY) = TelemetryRecord {
            flags,
            _pad: [0, 0, 0],
            pressure_bits: pressure.to_bits(),
            dose,
        };
        let ptr = core::ptr::addr_of!(TELEMETRY) as i32;
        let len = core::mem::size_of::<TelemetryRecord>() as i32;
        commit_telemetry_vector(ptr, len);
    }

    let digest = (flags as i32)
        .wrapping_add(pressure.to_bits() as i32)
        .wrapping_add(dose as i32);
    unsafe {
        commit_uplink(digest, dose as i32);
    }

    flags as i32
}

/// Primary exported entry — delegates to [`evaluate_limits`].
#[no_mangle]
pub extern "C" fn diagnostic() -> i32 {
    evaluate_limits()
}

/// Payload version tag for routing tables.
#[no_mangle]
pub extern "C" fn payload_version() -> u32 {
    PAYLOAD_MAGIC
}
