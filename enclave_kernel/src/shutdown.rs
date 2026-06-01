//! Self-annihilation controller — zero memory, clear architectural state, hard sleep.

use crate::memory;
use crate::mmio;
use crate::serial_println;

/// Outcome bitfield written to MMIO status before power-down.
#[derive(Debug, Clone, Copy, Default)]
pub struct ShutdownReport {
    /// WASM diagnostic return value (guest `i32`).
    pub guest_result: i32,
    /// Fused 64-bit proof committed to uplink registers.
    pub proof: u64,
    /// Hardware vector that initiated the cycle.
    pub vector: u8,
}

/// Execute post-run annihilation: scrub sandbox/arena, flush registers, PMU dormancy.
pub fn self_annihilate(report: ShutdownReport) -> ! {
    serial_println!(
        "[AETHER] cycle success — guest={} proof=0x{:016X} vector=0x{:02X} — self-annihilation",
        report.guest_result,
        report.proof,
        report.vector
    );

    memory::annihilate_sandbox();
    memory::reset_arena();

    clear_architectural_state();

    mmio::request_dormancy();
    enter_absolute_halt();
}

/// Zero general-purpose registers that may have held transient secrets.
fn clear_architectural_state() {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        // SAFETY: No live Rust references depend on these registers across this boundary.
        core::arch::asm!(
            "xor rax, rax",
            "xor rbx, rbx",
            "xor rcx, rcx",
            "xor rdx, rdx",
            "xor rsi, rsi",
            "xor rdi, rdi",
            options(nomem, nostack)
        );
    }
}

/// QEMU `isa-debug-exit` I/O port (see `.cargo/config.toml` runner `-device`).
const QEMU_DEBUG_EXIT_PORT: u16 = 0xf4;
/// Exit code written to `isa-debug-exit` (QEMU terminates the process).
const QEMU_DEBUG_EXIT_SUCCESS: u32 = 0x10;

fn enter_absolute_halt() -> ! {
    #[cfg(target_arch = "x86_64")]
    {
        use x86_64::instructions::port::Port;

        // SAFETY: Port 0xf4 is the isa-debug-exit device when QEMU provides it.
        let mut debug_exit = Port::<u32>::new(QEMU_DEBUG_EXIT_PORT);
        unsafe {
            debug_exit.write(QEMU_DEBUG_EXIT_SUCCESS);
        }

        unsafe {
            core::arch::asm!(
                "cli",
                "hlt",
                options(nomem, nostack, noreturn)
            );
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    loop {
        core::hint::spin_loop();
    }
}
