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
        "[AETHER] sensors — BMP390: {} (0x{:02X}) INT:{}  ADS1115: {} (0x{:02X})  OLED: {}  SD: {}",
        if health.bmp390 { "OK" } else { "MISSING" },
        health.bmp390_addr,
        if health.bmp390_int { "GPIO1" } else { "off" },
        if health.ads1115 { "OK" } else { "MISSING" },
        health.ads1115_addr,
        if health.oled { "OK" } else { "MISSING" },
        if health.sd { "OK" } else { "MISSING" },
    );
    if !health.bmp390 || !health.ads1115 {
        serial_println!("[AETHER] hint: check 3.3V, GND, SDA=GPIO6, SCL=GPIO7, BMP390 INT=GPIO1");
    }
    if !health.sd {
        serial_println!("[AETHER] hint: SD optional — MOSI=GPIO3 MISO=GPIO4 SCK=GPIO5 CS=GPIO15");
    }
}

#[cfg(target_arch = "riscv32")]
fn log_mission_banner(health: enclave_kernel::platform::esp32c6::SensorHealth) {
    if health.bmp390 && health.ads1115 {
        serial_println!("[AETHER] === SEALED COMPARTMENT WITNESS READY ===");
        serial_println!("[AETHER] mode=EVENT_ONLY — log on pressure/dose change or button");
    } else {
        serial_println!("[AETHER] === WITNESS DEGRADED (check sensors) ===");
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
fn log_mission_profile() {
    use enclave_kernel::platform::mission_profile;

    let p = mission_profile::current();
    serial_println!(
        "[AETHER] mission — id={} payload={} mode={} interval_wake={} radio={} P_lim={:.3}atm D_lim={}{}",
        p.mission_id,
        mission_profile::payload_name(p.payload_slot),
        if p.interval_wake { "INTERVAL+EVENT" } else { "EVENT_ONLY" },
        if p.interval_wake { "on" } else { "off" },
        if p.radio_enable { "ON" } else { "OFF" },
        p.pressure_limit_atm,
        p.dose_limit,
        if p.from_sd {
            " (SD profile)"
        } else {
            " (pot<10%=interval, >75%=RELAXED, >90%=radio dry-run)"
        },
    );
    if p.interval_wake {
        serial_println!(
            "[AETHER] interval — wake every {}-{} s (pot tunes within range)",
            p.wake_min_secs,
            p.wake_max_secs,
        );
    }
}

#[cfg(target_arch = "riscv32")]
fn resolve_trigger() -> interrupts::HardwareInterrupt {
    if let Some(reason) = enclave_kernel::platform::esp32c6::pressure_wake_label() {
        serial_println!("[AETHER] {reason} — forcing vector 0x20");
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
    let _profile = enclave_kernel::platform::mission_profile::load_from_sd();
    enclave_kernel::platform::esp32c6::apply_pot_mission_profile();
    log_mission_profile();
    log_mission_banner(health);
    log_sensor_snapshot();
    enclave_kernel::platform::oled::play_boot_splash(health.bmp390 && health.ads1115);

    if enclave_kernel::platform::rtc_state::breach_latched() {
        let guest = enclave_kernel::platform::rtc_state::breach_guest();
        enclave_kernel::platform::esp32c6::sync_breach_led();
        serial_println!(
            "[AETHER] BREACH latched — {} (GPIO10 ON; press GPIO2 to ACK)",
            enclave_kernel::platform::demo::guest_flags_text(guest),
        );
        enclave_kernel::platform::oled::show_breach_reminder(guest);
        if enclave_kernel::platform::esp32c6::try_acknowledge_breach() {
            // LED off + serial printed in try_acknowledge_breach
        }
    }

    if enclave_kernel::platform::esp32c6::detect_demo_mode_hold() {
        demo_loop();
    }

    enclave_kernel::platform::esp32c6::establish_baseline_if_needed();

    if enclave_kernel::platform::esp32c6::should_resleep_after_bmp_int() {
        serial_println!("[AETHER] sensors stable — back to sleep (no log)");
        enclave_kernel::platform::esp32c6::request_deep_sleep();
    }

    if let Some(trigger) = enclave_kernel::platform::esp32c6::mission_cycle_trigger() {
        if let Some(event) = enclave_kernel::platform::esp32c6::sensor_change_detected() {
            serial_println!("[AETHER] event — {event}");
        }
        serial_println!(
            "[AETHER] wake — vector 0x{:02X}, running WASM cycle",
            trigger as u8,
        );
        run_one_cycle(trigger);
        enclave_kernel::platform::esp32c6::run_review_browser_if_requested();
    } else {
        serial_println!("[AETHER] event-only — no change; sleeping without log");
        enclave_kernel::platform::esp32c6::run_review_browser_if_requested();
        enclave_kernel::platform::esp32c6::request_deep_sleep();
    }

    if enclave_kernel::platform::mission_profile::interval_wake_enabled() {
        serial_println!(
            "[AETHER] entering deep sleep (button, BMP390 INT, or {} s timer)",
            enclave_kernel::platform::rtc_state::wake_timer_secs()
        );
    } else {
        serial_println!("[AETHER] entering deep sleep (button or BMP390 INT on change)");
    }
    shutdown::enter_absolute_halt();
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
