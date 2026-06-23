//! Event demo helpers — status text, JSON line, flag decode.

use crate::interrupts::HardwareInterrupt;

/// Decode guest status bitmask to plain text.
pub fn guest_flags_text(flags: i32) -> &'static str {
    match flags {
        0 => "OK",
        1 => "PRESSURE_LOW",
        2 => "DOSE_HIGH",
        3 => "PRESSURE_LOW|DOSE_HIGH",
        _ => "UNKNOWN",
    }
}

/// Human-readable wake source for serial logs.
pub fn wake_cause_text(cause: esp_hal::system::SleepSource) -> &'static str {
    use esp_hal::system::SleepSource;
    match cause {
        SleepSource::Timer => "RTC_TIMER",
        SleepSource::Gpio | SleepSource::Ext0 | SleepSource::Ext1 => "GPIO",
        _ => "POWER_ON_RESET",
    }
}

/// Vector name for logs.
pub fn vector_name(v: u8) -> &'static str {
    match v {
        0x20 => "PRESSURE",
        0x21 => "KINETIC_TIMER",
        _ => "UNKNOWN",
    }
}

pub fn trigger_label(t: Option<HardwareInterrupt>) -> &'static str {
    match t {
        Some(HardwareInterrupt::AtmosphericPressureThreshold) => "PRESSURE",
        Some(HardwareInterrupt::KineticJointActuation) => "KINETIC_TIMER",
        None => "SELF_TEST",
    }
}

/// One-line JSON for laptop capture / projection.
pub fn log_json_cycle(
    cycle: u32,
    guest: i32,
    proof: u64,
    vector: u8,
    pressure: f32,
    temp_c: f32,
    dose: u32,
    proof_changed: bool,
) {
    crate::serial_println!(
        "{{\"cycle\":{},\"guest\":{},\"flags\":\"{}\",\"proof\":\"0x{:016X}\",\"vector\":\"0x{:02X}\",\"pressure\":{:.3},\"temp_c\":{:.1},\"dose\":{},\"proof_changed\":{}}}",
        cycle,
        guest,
        guest_flags_text(guest),
        proof,
        vector,
        pressure,
        temp_c,
        dose,
        proof_changed,
    );
}

/// Barometric altitude estimate from pressure (atm).
pub fn altitude_m(pressure_atm: f32) -> f32 {
    if pressure_atm <= 0.0 {
        return 0.0;
    }
    // ISA barometric formula (approximate, good enough for demo).
    let p = pressure_atm as f64;
    (44330.0 * (1.0 - libm::pow(p, 0.1903))) as f32
}
