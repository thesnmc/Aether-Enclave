//! AETHER-ENCLAVE bare-metal binary — x86 bootloader entry or ESP32-C6 `esp_hal::main`.

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
    serial_println!("[AETHER] bench: software IRQ 0x20 (pressure threshold)");
    interrupts::software_trigger(interrupts::HardwareInterrupt::AtmosphericPressureThreshold);

    dormancy_loop();
}

// ---------------------------------------------------------------------------
// ESP32-C6 — esp-hal entry, USB Serial/JTAG logging, RTC deep sleep wake
// ---------------------------------------------------------------------------

#[cfg(target_arch = "riscv32")]
use esp_println as _;

#[cfg(target_arch = "riscv32")]
use esp_hal::clock::CpuClock;
#[cfg(target_arch = "riscv32")]
use esp_hal::main;

/// Timestamp hook used by esp-println (milliseconds since boot).
#[cfg(target_arch = "riscv32")]
#[unsafe(no_mangle)]
pub extern "Rust" fn _esp_println_timestamp() -> u64 {
    esp_hal::time::Instant::now()
        .duration_since_epoch()
        .as_millis()
}

#[cfg(target_arch = "riscv32")]
fn log_sensor_health(health: enclave_kernel::platform::esp32c6::SensorHealth) {
    serial_println!(
        "[AETHER] sensors — BMP390: {} (0x{:02X})  ADS1115: {} (0x{:02X})",
        if health.bmp390 { "OK" } else { "MISSING" },
        health.bmp390_addr,
        if health.ads1115 { "OK" } else { "MISSING" },
        health.ads1115_addr,
    );
    if !health.bmp390 || !health.ads1115 {
        serial_println!("[AETHER] hint: check 3.3V, GND, SDA=GPIO8, SCL=GPIO9");
    }
}

#[cfg(target_arch = "riscv32")]
fn log_sensor_snapshot() {
    let pressure = mmio::read_atmospheric_pressure();
    let dose = mmio::read_radiation_dosimeter();
    serial_println!(
        "[AETHER] snapshot — pressure={:.3} atm  dose={} counts",
        pressure,
        dose,
    );
}

#[cfg(target_arch = "riscv32")]
#[main]
fn esp_main() -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    memory::reset_arena();
    mmio::serial_init();

    serial_println!("[AETHER] ESP32-C6 cold boot — USB Serial/JTAG ready");

    let health = enclave_kernel::platform::esp32c6::init(peripherals);
    interrupts::init();

    log_sensor_health(health);
    log_sensor_snapshot();

    if let Some(trigger) = interrupts::detect_wake_trigger() {
        serial_println!(
            "[AETHER] wake — vector 0x{:02X}, running WASM cycle",
            trigger as u8
        );
        runtime::sovereign_bootstrap(Some(trigger));
    } else {
        serial_println!("[AETHER] cold boot — running WASM self-test (vector 0x20)");
        interrupts::latch_vector(
            interrupts::HardwareInterrupt::AtmosphericPressureThreshold as u8,
        );
        runtime::sovereign_bootstrap(Some(
            interrupts::HardwareInterrupt::AtmosphericPressureThreshold,
        ));
    }

    // Unreachable on ESP32-C6: sovereign_bootstrap ends in deep sleep.
    serial_println!("[AETHER] entering deep sleep (10 s timer or GPIO2 high)");
    dormancy_loop();
}

/// Wait for the next wake trigger, then run the WASM cycle.
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
                    "[AETHER] wake — vector 0x{:02X}, running WASM cycle",
                    vector
                );
                runtime::sovereign_bootstrap(interrupts::HardwareInterrupt::from_vector(vector));
            }
        }
    }

    #[cfg(target_arch = "riscv32")]
    {
        enclave_kernel::platform::esp32c6::request_deep_sleep();
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
