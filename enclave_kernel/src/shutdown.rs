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
        use crate::platform::{demo, esp32c6, rtc_state};

        let prev_proof = rtc_state::last_proof();
        let sample = esp32c6::read_env_sample();
        let cycle = rtc_state::cycle_count().saturating_add(1);
        let proof_changed = report.proof != prev_proof;

        serial_println!(
            "[AETHER] cycle #{} — guest={} ({}) proof=0x{:016X} vector=0x{:02X} ({}) proof_changed={}",
            cycle,
            report.guest_result,
            demo::guest_flags_text(report.guest_result),
            report.proof,
            report.vector,
            demo::vector_name(report.vector),
            proof_changed,
        );

        demo::log_json_cycle(
            cycle,
            report.guest_result,
            report.proof,
            report.vector,
            sample.pressure_atm,
            sample.temp_c,
            sample.dose_scaled,
            proof_changed,
        );

        rtc_state::record_cycle(report.proof, sample.pressure_atm.to_bits());
        crate::platform::oled::show_cycle(cycle, report.guest_result, report.proof, report.vector);
        if crate::platform::sd_log::log_cycle(
            cycle,
            report.guest_result,
            report.proof,
            report.vector,
            sample.pressure_atm,
            sample.temp_c,
            sample.dose_scaled,
            proof_changed,
        ) {
            serial_println!("[AETHER] SD — cycle #{} logged", cycle);
        }
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
