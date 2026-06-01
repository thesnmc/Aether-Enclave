//! Sovereign diagnostic WebAssembly payload (`wasm32-unknown-unknown`, `#![no_std]`).
//!
//! Exports `diagnostic` and imports host syscalls from the `aether` module.

#![no_std]

/// Placeholder constant referenced by the diagnostic path (flight software version tag).
pub const PAYLOAD_MAGIC: u32 = 0xA17E_0001;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[link(wasm_import_module = "aether")]
extern "C" {
    fn read_sensor(channel: i32) -> i32;
    fn commit_uplink(proof_lo: i32, proof_hi: i32);
}

/// Primary exported entry — read sensor, fuse magic, commit proof, return digest.
#[no_mangle]
pub unsafe extern "C" fn diagnostic() -> i32 {
    let mut acc = read_sensor(0);
    acc = acc.wrapping_add(0xA17E);
    commit_uplink(acc, 0);
    acc
}

/// Placeholder export for future routing tables / multi-entry payloads.
#[no_mangle]
pub extern "C" fn payload_version() -> u32 {
    PAYLOAD_MAGIC
}
