//! AETHER-ENCLAVE bare-metal binary — x86 bootloader entry or ESP32-C6 `esp_hal::main`.

#![no_std]
#![no_main]

use core::panic::PanicInfo;

use enclave_kernel::{interrupts, memory, mmio, runtime, shutdown, serial_println};

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

#[cfg(target_arch = "riscv32")]
use esp_println as _;

#[cfg(target_arch = "riscv32")]
use esp_hal::clock::CpuClock;
#[cfg(target_arch = "riscv32")]
use esp_hal::delay::Delay;
#[cfg(target_arch = "riscv32")]
use esp_hal::main;

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
        "[AETHER] sensors — BMP390: {} (0x{:02X})  ADS1115: {} (0x{:02X})  OLED: {}",
        if health.bmp390 { "OK" } else { "MISSING" },
        health.bmp390_addr,
        if health.ads1115 { "OK" } else { "MISSING" },
        health.ads1115_addr,
        if health.oled { "OK" } else { "MISSING" },
    );
    if !health.bmp390 || !health.ads1115 {
        serial_println!("[AETHER] hint: check 3.3V, GND, SDA=GPIO6, SCL=GPIO7");
    }
}

#[cfg(target_arch = "riscv32")]
fn log_mission_banner(health: enclave_kernel::platform::esp32c6::SensorHealth) {
    if health.bmp390 && health.ads1115 {
        serial_println!("[AETHER] === MISSION READY ===");
    } else {
        serial_println!("[AETHER] === MISSION DEGRADED (sensor fault) ===");
    }
}

#[cfg(target_arch = "riscv32")]
fn log_sensor_snapshot() {
    use enclave_kernel::platform::{demo, esp32c6, rtc_state};

    let sample = esp32c6::read_env_sample();
    serial_println!(
        "[AETHER] snapshot — cycle={} pressure={:.3} atm alt={:.0} m temp={:.1} C dose={} (raw {}) wake_timer={}s",
        rtc_state::cycle_count(),
        sample.pressure_atm,
        demo::altitude_m(sample.pressure_atm),
        sample.temp_c,
        sample.dose_scaled,
        sample.dose_raw,
        rtc_state::wake_timer_secs(),
    );
}

#[cfg(target_arch = "riscv32")]
fn resolve_trigger() -> interrupts::HardwareInterrupt {
    if enclave_kernel::platform::esp32c6::pressure_drop_wake() {
        serial_println!("[AETHER] pressure drop detected — forcing vector 0x20");
        return interrupts::HardwareInterrupt::AtmosphericPressureThreshold;
    }
    if let Some(trigger) = interrupts::detect_wake_trigger() {
        return trigger;
    }
    interrupts::HardwareInterrupt::AtmosphericPressureThreshold
}

#[cfg(target_arch = "riscv32")]
fn run_one_cycle(trigger: interrupts::HardwareInterrupt) {
    serial_println!(
        "[AETHER] run — vector 0x{:02X} ({})",
        trigger as u8,
        enclave_kernel::platform::demo::trigger_label(Some(trigger)),
    );
    interrupts::latch_vector(trigger as u8);
    runtime::run_mission_cycle(Some(trigger));
}

#[cfg(target_arch = "riscv32")]
fn demo_loop() -> ! {
    serial_println!("[AETHER] DEMO MODE — hold GPIO2 at boot; cycles every 2 s");
    loop {
        let trigger = resolve_trigger();
        run_one_cycle(trigger);
        Delay::new().delay_millis(2000);
    }
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

    enclave_kernel::platform::esp32c6::log_wake_cause();
    log_sensor_health(health);
    enclave_kernel::platform::esp32c6::apply_pot_mission_profile();
    log_mission_banner(health);
    log_sensor_snapshot();
    enclave_kernel::platform::oled::show_boot(health.bmp390 && health.ads1115);

    if enclave_kernel::platform::esp32c6::detect_demo_mode_hold() {
        demo_loop();
    }

    let trigger = resolve_trigger();
    if interrupts::detect_wake_trigger().is_some() {
        serial_println!(
            "[AETHER] wake — vector 0x{:02X}, running WASM cycle",
            trigger as u8
        );
    } else {
        serial_println!("[AETHER] cold boot — WASM self-test (vector 0x20)");
    }

    run_one_cycle(trigger);
    serial_println!(
        "[AETHER] entering deep sleep (GPIO2 high or {} s timer)",
        enclave_kernel::platform::rtc_state::wake_timer_secs()
    );
    shutdown::enter_absolute_halt();
}

fn dormancy_loop() -> ! {
    #[cfg(target_arch = "x86_64")]
    {
        loop {
            interrupts::enable();
            interrupts::halt_until_interrupt();

            if interrupts::wake_pending() {
                interrupts::clear_wake();
                let vector = interrupts::last_vector();
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
