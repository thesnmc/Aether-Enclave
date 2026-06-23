//! WebAssembly payload — pressure + dose limit check for the enclave host.
//! Limits are read from the host (mission profile / SD); defaults match strict mode.

#![no_std]

pub const HOST_IMPORT_MODULE: &str = "aether";

pub const STATUS_OK: i32 = 0;
pub const STATUS_PRESSURE_LOW: i32 = 0x1;
pub const STATUS_DOSE_HIGH: i32 = 0x2;
pub const STATUS_BOTH: i32 = STATUS_PRESSURE_LOW | STATUS_DOSE_HIGH;

#[repr(C)]
pub struct TelemetryRecord {
    pub flags: u8,
    pub _pad: [u8; 3],
    pub pressure_bits: u32,
    pub dose: u32,
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[link(wasm_import_module = "aether")]
extern "C" {
    fn read_atmospheric_pressure() -> f32;
    fn read_radiation_dosimeter() -> i32;
    fn read_pressure_limit() -> f32;
    fn read_dose_limit() -> i32;
    fn commit_telemetry_vector(ptr: i32, len: i32);
}

#[no_mangle]
pub extern "C" fn evaluate_limits() -> i32 {
    let pressure = unsafe { read_atmospheric_pressure() };
    let dose = unsafe { read_radiation_dosimeter() } as u32;
    let p_lim = unsafe { read_pressure_limit() };
    let d_lim = unsafe { read_dose_limit() } as u32;

    let mut flags = STATUS_OK;
    if pressure < p_lim {
        flags |= STATUS_PRESSURE_LOW;
    }
    if dose > d_lim {
        flags |= STATUS_DOSE_HIGH;
    }

    let record = TelemetryRecord {
        flags: flags as u8,
        _pad: [0; 3],
        pressure_bits: pressure.to_bits(),
        dose,
    };
    let ptr = (&record as *const TelemetryRecord) as i32;
    let len = core::mem::size_of::<TelemetryRecord>() as i32;
    unsafe {
        commit_telemetry_vector(ptr, len);
    }

    flags
}
