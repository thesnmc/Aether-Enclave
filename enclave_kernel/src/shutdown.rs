//! Post-cycle memory wipe and sleep entry.

use crate::memory;
use crate::mmio;
use crate::serial_println;

/// Outcome written to serial and RTC before sleep.
#[derive(Debug, Clone, Copy, Default)]
pub struct ShutdownReport {
    /// WASM diagnostic return value (guest `i32`).
    pub guest_result: i32,
    /// Fused 64-bit proof committed to uplink registers.
    pub proof: u64,
    /// Hardware vector that initiated the cycle.
    pub vector: u8,
}

/// Wipe memory and log cycle outcome.
pub fn finish_cycle(report: ShutdownReport) {
    #[cfg(target_arch = "riscv32")]
    {
        use esp_hal::time::Instant;

        use crate::platform::{demo, esp32c6, mission_profile, power_log, rtc_state};

        let prev_proof = rtc_state::last_proof();
        let sample = esp32c6::read_env_sample();
        let cycle = rtc_state::cycle_count().saturating_add(1);
        let proof_changed = report.proof != prev_proof;
        let active_ms = power_log::cycle_active_ms();
        let profile = mission_profile::current();
        let cycle_ms = Instant::now().duration_since_epoch().as_millis() as u32;

        serial_println!(
            "[AETHER] cycle #{} — guest={} ({}) proof=0x{:016X} prev=0x{:016X} chain={} payload={} mission={} vector=0x{:02X} ({}) active={}ms",
            cycle,
            report.guest_result,
            demo::guest_flags_text(report.guest_result),
            report.proof,
            prev_proof,
            if proof_changed { "LINKED" } else { "REPEAT" },
            mission_profile::payload_name(profile.payload_slot),
            profile.mission_id,
            report.vector,
            demo::vector_name(report.vector),
            active_ms,
        );

        demo::log_json_cycle(
            cycle,
            report.guest_result,
            report.proof,
            prev_proof,
            report.vector,
            sample.pressure_atm,
            sample.temp_c,
            sample.dose_scaled,
            proof_changed,
            profile.mission_id,
            profile.payload_slot,
            active_ms,
        );

        rtc_state::record_cycle(report.proof, sample.pressure_atm.to_bits(), cycle_ms);
        crate::platform::oled::show_cycle(
            cycle,
            report.guest_result,
            report.proof,
            report.vector,
            proof_changed,
        );
        if crate::platform::sd_log::log_cycle(
            cycle,
            report.guest_result,
            report.proof,
            prev_proof,
            report.vector,
            sample.pressure_atm,
            sample.temp_c,
            sample.dose_scaled,
            proof_changed,
            profile.mission_id,
            profile.payload_slot,
            active_ms,
        ) {
            serial_println!("[AETHER] SD — cycle #{} logged", cycle);
        }
        power_log::log_power_budget(rtc_state::wake_timer_secs());
        esp32c6::status_led_off();
    }

    #[cfg(not(target_arch = "riscv32"))]
    {
        serial_println!(
            "[AETHER] cycle done — guest={} proof=0x{:016X} vector=0x{:02X} — wiping memory",
            report.guest_result,
            report.proof,
            report.vector
        );
    }

    memory::wipe_host_memory();
    mmio::request_dormancy();
}

/// Wipe memory, log, and enter platform sleep.
pub fn self_annihilate(report: ShutdownReport) -> ! {
    finish_cycle(report);
    enter_absolute_halt();
}

/// Deep sleep (C6) or QEMU exit (x86).
pub fn enter_absolute_halt() -> ! {
    #[cfg(target_arch = "x86_64")]
    {
        use x86_64::instructions::port::Port;

        let mut debug_exit = Port::<u32>::new(0xf4);
        unsafe {
            debug_exit.write(0x10);
        }
        unsafe {
            core::arch::asm!("cli", "hlt", options(nomem, nostack, noreturn));
        }
    }

    #[cfg(target_arch = "riscv32")]
    {
        crate::platform::esp32c6::request_deep_sleep();
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "riscv32")))]
    loop {
        core::hint::spin_loop();
    }
}
