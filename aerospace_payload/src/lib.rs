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

/// Host-visible status flags (`i32` return from [`evaluate_limits`]).
pub const STATUS_OK: i32 = 0;
/// Bit `0x1`: pressure below [`PRESSURE_LIMIT_ATM`].
pub const STATUS_PRESSURE_LOW: i32 = 0x1;
/// Bit `0x2`: dose above [`DOSE_LIMIT`].
pub const STATUS_DOSE_HIGH: i32 = 0x2;
/// Both limits exceeded (e.g. bench `sim_inject_o2_drop`: 0.12 atm, 1250 dose).
pub const STATUS_BOTH: i32 = STATUS_PRESSURE_LOW | STATUS_DOSE_HIGH;

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
    fn read_atmospheric_pressure() -> f32;
    fn read_radiation_dosimeter() -> i32;
    fn commit_telemetry_vector(ptr: i32, len: i32);
    fn commit_uplink(proof_lo: i32, proof_hi: i32);
}

/// Evaluate pressure + radiation limits, commit telemetry, return combined status flags as `i32`.
#[no_mangle]
pub extern "C" fn evaluate_limits() -> i32 {
    let pressure = unsafe { read_atmospheric_pressure() };
    let dose = unsafe { read_radiation_dosimeter() } as u32;

    let mut status: i32 = STATUS_OK;
    if pressure < PRESSURE_LIMIT_ATM {
        status |= STATUS_PRESSURE_LOW;
    }
    if dose > DOSE_LIMIT {
        status |= STATUS_DOSE_HIGH;
    }

    static mut TELEMETRY: TelemetryRecord = TelemetryRecord {
        flags: 0,
        _pad: [0, 0, 0],
        pressure_bits: 0,
        dose: 0,
    };

    unsafe {
        *core::ptr::addr_of_mut!(TELEMETRY) = TelemetryRecord {
            flags: status as u8,
            _pad: [0, 0, 0],
            pressure_bits: pressure.to_bits(),
            dose,
        };
        let ptr = core::ptr::addr_of!(TELEMETRY) as i32;
        let len = core::mem::size_of::<TelemetryRecord>() as i32;
        commit_telemetry_vector(ptr, len);
    }

    let digest = status
        .wrapping_add(pressure.to_bits() as i32)
        .wrapping_add(dose as i32);
    unsafe {
        commit_uplink(digest, dose as i32);
    }

    // Explicit positive bitflags for the kernel shutdown report (3 = both limits).
    status
}

/// Exported alias — same symbol the host resolves first when both are present.
#[no_mangle]
pub extern "C" fn diagnostic() -> i32 {
    evaluate_limits()
}

/// Payload version tag for routing tables.
#[no_mangle]
pub extern "C" fn payload_version() -> u32 {
    PAYLOAD_MAGIC
}
