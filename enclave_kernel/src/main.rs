//! AETHER-ENCLAVE bare-metal binary — bootloader / ESP entry and dormancy lifecycle.

#![no_std]
#![no_main]

use core::panic::PanicInfo;

use enclave_kernel::{interrupts, memory, mmio, runtime, shutdown, serial_println};

// ---------------------------------------------------------------------------
// x86_64 — bootloader entry + QEMU bench harness
// ---------------------------------------------------------------------------

#[cfg(target_arch = "x86_64")]
use bootloader::{entry_point, BootInfo};

#[cfg(target_arch = "x86_64")]
entry_point!(kernel_main);

#[cfg(target_arch = "x86_64")]
fn kernel_main(_boot_info: &'static BootInfo) -> ! {
    memory::reset_arena();
    mmio::serial_init();
    interrupts::init();

    serial_println!("[AETHER] cold boot — COM1 ready, entering dormancy");

    mmio::sim_inject_o2_drop();
    serial_println!("[AETHER] bench: software IRQ 0x20 (atmospheric threshold)");
    interrupts::software_trigger(interrupts::HardwareInterrupt::AtmosphericPressureThreshold);

    dormancy_loop();
}

// ---------------------------------------------------------------------------
// ESP32-C3 — esp-hal entry + hybrid RTC deep sleep wake cycle
// ---------------------------------------------------------------------------

#[cfg(target_arch = "riscv32")]
use esp_hal::clock::CpuClock;
#[cfg(target_arch = "riscv32")]
use esp_hal::main;

#[cfg(target_arch = "riscv32")]
#[main]
fn esp_main() -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    enclave_kernel::platform::esp32c3::init(peripherals);

    memory::reset_arena();
    mmio::serial_init();
    interrupts::init();

    serial_println!("[AETHER] ESP32-C3 cold boot — UART0 ready");

    if let Some(trigger) = interrupts::detect_wake_trigger() {
        serial_println!(
            "[AETHER] wake event — vector 0x{:02X}, sovereign bootstrap",
            trigger as u8
        );
        runtime::sovereign_bootstrap(Some(trigger));
    }

    serial_println!("[AETHER] entering hybrid RTC deep sleep (10s + GPIO2 high)");
    dormancy_loop();
}

/// Absolute zero-power wait until the next physical trigger.
fn dormancy_loop() -> ! {
    #[cfg(target_arch = "x86_64")]
    {
        loop {
            interrupts::enable();
            interrupts::halt_until_interrupt();

            if interrupts::wake_pending() {
                interrupts::clear_wake();
                let vector = interrupts::last_vector();
                serial_println!(
                    "[AETHER] spurious wake — vector 0x{:02X}, handoff to bootstrap",
                    vector
                );
                runtime::sovereign_bootstrap(interrupts::HardwareInterrupt::from_vector(vector));
            }
        }
    }

    #[cfg(target_arch = "riscv32")]
    {
        enclave_kernel::platform::esp32c3::request_deep_sleep();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("[AETHER] KERNEL PANIC: {}", info);
    shutdown::self_annihilate(shutdown::ShutdownReport {
        guest_result: -1,
        proof: 0,
        vector: interrupts::last_vector(),
    });
}
