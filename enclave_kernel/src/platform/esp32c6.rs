//! ESP32-C6 board support: I2C sensors (BMP390 + ADS1115), watchdog, RTC deep sleep.
//!
//! I2C on GPIO6/7. BMP390 INT on GPIO1 (top-row INT pin on breakout).
//! Button wake GPIO2 high; GPIO10 optional status LED.

use core::sync::atomic::{AtomicBool, Ordering};
use core::time::Duration as CoreDuration;

use esp_hal::delay::Delay;
use esp_hal::gpio::{Input, InputConfig, Output, OutputConfig, Pull, RtcPinWithResistors, Level};
use esp_hal::i2c::master::{Config as I2cConfig, I2c};
use esp_hal::peripherals::{GPIO1, GPIO2, TIMG0};
use esp_hal::rtc_cntl::Rtc;
use esp_hal::rtc_cntl::sleep::{
    Ext1WakeupSource, RtcSleepConfig, TimerWakeupSource, WakeSource, WakeupLevel,
};
use esp_hal::system::{self, SleepSource};
use esp_hal::time::{Duration, Rate};
use esp_hal::timer::timg::{MwdtStage, TimerGroup, Wdt};
use esp_hal::Blocking;
use spin::Mutex;

use crate::interrupts::HardwareInterrupt;
use crate::platform::rtc_state;

const WDT_TIMEOUT_SECS: u64 = 30;

const BMP390_ADDR_PRIMARY: u8 = 0x76;
const BMP390_ADDR_SECONDARY: u8 = 0x77;
const ADS1115_ADDR_PRIMARY: u8 = 0x48;
const ADS1115_ADDR_SECONDARY: u8 = 0x49;

const BMP390_REG_CHIP_ID: u8 = 0x00;
const BMP390_REG_INT_STATUS: u8 = 0x11;
const BMP390_REG_DATA: u8 = 0x04;
const BMP390_REG_CALIB: u8 = 0x31;
const BMP390_REG_INT_CTRL: u8 = 0x19;
const BMP390_REG_PWR_CTRL: u8 = 0x1B;
const BMP390_REG_ODR: u8 = 0x1D;
const BMP390_REG_CMD: u8 = 0x7E;
const BMP390_CHIP_ID: u8 = 0x60;
const BMP390_SOFT_RESET: u8 = 0xB6;
/// Normal mode + pressure + temperature enabled.
const BMP390_PWR_NORMAL: u8 = 0x33;
/// ~1.5 Hz ODR while sleeping (640 ms between samples).
const BMP390_ODR_SLEEP: u8 = 0x07;
/// Open-drain, active-low INT + data-ready to INT pin.
const BMP390_INT_DRDY: u8 = 0x41;

const ADS1115_REG_CONVERSION: u8 = 0x00;
const ADS1115_REG_CONFIG: u8 = 0x01;
const ADS1115_CFG_AIN0: u16 = 0xC583;

/// Pressure delta between sleeps that counts as an event (atm).
const PRESSURE_DROP_ATM: f32 = 0.015;

/// Scaled dose delta that counts as an event.
const DOSE_CHANGE_MIN: u32 = 80;

/// Boot-time button hold for continuous demo mode (ms).
const DEMO_HOLD_MS: u32 = 400;

/// Samples averaged for each sensor read.
const SENSOR_SAMPLES: usize = 3;

#[derive(Debug, Clone, Copy, Default)]
pub struct SensorHealth {
    pub bmp390: bool,
    pub ads1115: bool,
    pub oled: bool,
    pub sd: bool,
    pub bmp390_addr: u8,
    pub bmp390_int: bool,
    pub ads1115_addr: u8,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct EnvSample {
    pub pressure_atm: f32,
    pub temp_c: f32,
    pub dose_raw: u32,
    pub dose_scaled: u32,
}

#[derive(Clone, Copy, Default)]
struct Bmp390Calib {
    par_t1: u16,
    par_t2: u16,
    par_t3: i8,
    par_p1: i16,
    par_p2: i16,
    par_p3: i8,
    par_p4: i8,
    par_p5: u16,
    par_p6: u16,
    par_p7: i8,
    par_p8: i8,
    par_p9: i16,
    par_p10: i8,
    par_p11: i8,
}

pub(crate) struct PlatformState {
    i2c: I2c<'static, Blocking>,
    rtc: Rtc<'static>,
    wdt: Wdt<TIMG0<'static>>,
    wake_gpio: GPIO2<'static>,
    bmp_int_gpio: GPIO1<'static>,
    pub(crate) review_gpio: esp_hal::peripherals::GPIO9<'static>,
    status_led: Output<'static>,
    bmp_calib: Bmp390Calib,
    bmp_addr: u8,
    ads_addr: u8,
    health: SensorHealth,
}

static PLATFORM: Mutex<Option<PlatformState>> = Mutex::new(None);
static READY: AtomicBool = AtomicBool::new(false);
static DEMO_MODE: AtomicBool = AtomicBool::new(false);

fn with_platform<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut PlatformState) -> R,
{
    if !READY.load(Ordering::Acquire) {
        return None;
    }
    PLATFORM.lock().as_mut().map(f)
}

/// Borrow the shared I2C bus (sensors + optional OLED).
pub fn with_i2c_bus<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut I2c<'static, Blocking>) -> R,
{
    with_platform(|state| f(&mut state.i2c))
}

/// Boot: WDT, I2C @ GPIO6/7, sensors, GPIO2 button + GPIO1 BMP390 INT, GPIO10 LED.
pub fn init(peripherals: esp_hal::peripherals::Peripherals) -> SensorHealth {
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let mut wdt = timg0.wdt;
    wdt.set_timeout(MwdtStage::Stage0, Duration::from_secs(WDT_TIMEOUT_SECS));
    wdt.enable();
    wdt.feed();

    let i2c = I2c::new(
        peripherals.I2C0,
        I2cConfig::default().with_frequency(Rate::from_khz(400)),
    )
    .expect("I2C init")
    .with_sda(peripherals.GPIO6)
    .with_scl(peripherals.GPIO7);

    let mut gpio2 = peripherals.GPIO2;
    let demo_active = detect_demo_hold_on_pin(&mut gpio2);
    DEMO_MODE.store(demo_active, Ordering::Release);
    let wake_gpio = gpio2;
    let bmp_int_gpio = peripherals.GPIO1;
    let review_gpio = peripherals.GPIO9;

    let status_led = Output::new(
        peripherals.GPIO10,
        Level::Low,
        OutputConfig::default(),
    );

    let mut health = SensorHealth::default();
    let mut bmp_calib = Bmp390Calib::default();
    let mut bmp_addr = 0u8;
    let mut ads_addr = 0u8;

    let mut probe = PlatformState {
        i2c,
        rtc: Rtc::new(peripherals.LPWR),
        wdt,
        wake_gpio,
        bmp_int_gpio,
        review_gpio,
        status_led,
        bmp_calib: Bmp390Calib::default(),
        bmp_addr: 0,
        ads_addr: 0,
        health: SensorHealth::default(),
    };

    for addr in [BMP390_ADDR_PRIMARY, BMP390_ADDR_SECONDARY] {
        if init_bmp390(&mut probe, addr).is_ok() {
            health.bmp390 = true;
            health.bmp390_addr = addr;
            health.bmp390_int = configure_bmp390_interrupt(&mut probe).is_ok();
            bmp_addr = addr;
            bmp_calib = probe.bmp_calib;
            break;
        }
    }

    for addr in [ADS1115_ADDR_PRIMARY, ADS1115_ADDR_SECONDARY] {
        if init_ads1115(&mut probe, addr).is_ok() {
            health.ads1115 = true;
            health.ads1115_addr = addr;
            ads_addr = addr;
            break;
        }
    }

    probe.bmp_calib = bmp_calib;
    probe.bmp_addr = bmp_addr;
    probe.ads_addr = ads_addr;
    probe.health = health;

    *PLATFORM.lock() = Some(probe);
    READY.store(true, Ordering::Release);
    feed_watchdog();

    health.oled = crate::platform::oled::init();
    if let Some(state) = PLATFORM.lock().as_mut() {
        state.health.oled = health.oled;
    }

    health.sd = crate::platform::sd_log::init(
        peripherals.SPI2,
        peripherals.GPIO3,
        peripherals.GPIO4,
        peripherals.GPIO5,
        peripherals.GPIO15,
    );

    health
}

/// True when GPIO2 was held at boot — runs WASM cycles in a loop (no deep sleep).
pub fn detect_demo_mode_hold() -> bool {
    DEMO_MODE.load(Ordering::Acquire)
}

fn detect_demo_hold_on_pin(gpio2: &mut GPIO2<'_>) -> bool {
    let button = Input::new(
        gpio2.reborrow(),
        InputConfig::default().with_pull(Pull::Down),
    );
    let mut seen_high = 0u32;
    for _ in 0..40 {
        if button.is_high() {
            seen_high += 1;
        }
        feed_watchdog();
        Delay::new().delay_millis(10);
    }
    seen_high >= (DEMO_HOLD_MS / 10)
}

/// Sample pot / dose channel and configure RTC wake timer + dose sensitivity.
pub fn apply_pot_mission_profile() {
    let raw = read_ads1115_raw_avg();
    rtc_state::set_wake_timer_from_pot(raw);
    rtc_state::set_dose_sensitivity(raw);
    crate::platform::mission_profile::apply_pot_payload_override(raw);
}

pub fn feed_watchdog() {
    with_platform(|state| state.wdt.feed());
}

pub fn sync_breach_led() {
    if crate::platform::rtc_state::breach_latched() {
        status_led_on();
    } else {
        status_led_off();
    }
}

/// GPIO2 pressed at wake while alert latched → operator acknowledge.
pub fn try_acknowledge_breach() -> bool {
    if !crate::platform::rtc_state::breach_latched() {
        return false;
    }
    if !button_pressed_now() {
        return false;
    }
    crate::platform::rtc_state::clear_breach();
    status_led_off();
    crate::serial_println!("[AETHER] BREACH ACK — GPIO2 operator clear");
    true
}

pub fn status_led_on() {
    with_platform(|state| state.status_led.set_high());
}

pub fn status_led_off() {
    with_platform(|state| state.status_led.set_low());
}

pub fn log_wake_cause() {
    let cause = system::wakeup_cause();
    crate::serial_println!(
        "[AETHER] wake cause — {} ({})",
        crate::platform::demo::wake_cause_text(cause),
        wake_source_label(),
    );
}

/// Human-readable wake source after classifying GPIO vs timer.
pub fn wake_source_label() -> &'static str {
    match system::wakeup_cause() {
        SleepSource::Timer => "RTC_TIMER",
        SleepSource::Gpio | SleepSource::Ext0 | SleepSource::Ext1 => {
            if button_pressed_now() {
                "GPIO2_BUTTON"
            } else if bmp390_interrupt_pending() {
                "BMP390_INT"
            } else {
                "GPIO"
            }
        }
        _ => "POWER_ON_RESET",
    }
}

pub fn detect_wake_trigger() -> Option<HardwareInterrupt> {
    match system::wakeup_cause() {
        SleepSource::Timer => Some(HardwareInterrupt::KineticJointActuation),
        SleepSource::Gpio | SleepSource::Ext0 | SleepSource::Ext1 => {
            Some(HardwareInterrupt::AtmosphericPressureThreshold)
        }
        _ => None,
    }
}

/// BMP390 INT woke the chip but sensors are stable — skip WASM cycle.
pub fn should_resleep_after_bmp_int() -> bool {
    match system::wakeup_cause() {
        SleepSource::Gpio | SleepSource::Ext0 | SleepSource::Ext1 => {}
        _ => return false,
    }
    if button_pressed_now() {
        return false;
    }
    if crate::platform::mission_profile::interval_wake_enabled() {
        match system::wakeup_cause() {
            SleepSource::Timer => return false,
            _ => {}
        }
    }
    if !bmp390_interrupt_pending() && !crate::platform::mission_profile::interval_wake_enabled() {
        return sensor_change_detected().is_none();
    }
    if bmp390_interrupt_pending() {
        clear_bmp390_interrupt();
    }
    sensor_change_detected().is_none()
}

/// True when pressure or dose moved enough vs last baseline/cycle.
pub fn sensor_change_detected() -> Option<&'static str> {
    if let Some(reason) = pressure_wake_label() {
        return Some(reason);
    }

    let sample = read_env_sample();
    let last_p = f32::from_bits(rtc_state::last_pressure_bits());
    if last_p <= 0.0 || sample.pressure_atm <= 0.0 {
        return None;
    }

    let delta_p = (last_p - sample.pressure_atm).abs();
    if delta_p >= PRESSURE_DROP_ATM {
        return Some(if last_p > sample.pressure_atm {
            "pressure drop"
        } else {
            "pressure rise"
        });
    }

    let last_d = rtc_state::last_dose();
    if sample.dose_scaled.abs_diff(last_d) >= DOSE_CHANGE_MIN {
        return Some("dose change");
    }

    None
}

/// Save first sensor reading as reference, then sleep (event-only mode).
pub fn establish_baseline_if_needed() {
    if rtc_state::has_sensor_baseline() {
        return;
    }
    let sample = read_env_sample();
    if sample.pressure_atm <= 0.0 {
        return;
    }
    rtc_state::record_baseline(sample.pressure_atm.to_bits(), sample.dose_scaled);
    crate::serial_println!(
        "[AETHER] baseline — P={:.3} atm dose={} (event-only; waiting for change)",
        sample.pressure_atm,
        sample.dose_scaled,
    );
}

/// Whether a full WASM log cycle should run this wake.
pub fn mission_cycle_trigger() -> Option<HardwareInterrupt> {
    use crate::platform::mission_profile;

    if button_pressed_now() {
        return Some(HardwareInterrupt::AtmosphericPressureThreshold);
    }

    if mission_profile::interval_wake_enabled() {
        if matches!(system::wakeup_cause(), SleepSource::Timer) {
            return Some(HardwareInterrupt::KineticJointActuation);
        }
    }

    if sensor_change_detected().is_some() {
        return Some(HardwareInterrupt::AtmosphericPressureThreshold);
    }

    None
}

/// Pressure-based wake reason (one sensor sample per check).
pub fn pressure_wake_label() -> Option<&'static str> {
    use esp_hal::time::Instant;

    let sample = read_env_sample();
    if sample.pressure_atm <= 0.0 {
        return None;
    }
    let last = f32::from_bits(rtc_state::last_pressure_bits());
    if last <= 0.0 {
        return None;
    }

    let last_ms = rtc_state::last_cycle_ms();
    if last_ms != 0 {
        let now_ms = Instant::now().duration_since_epoch().as_millis() as u32;
        let dt_ms = now_ms.wrapping_sub(last_ms);
        if dt_ms >= 1_000 {
            let rate = (last - sample.pressure_atm) / (dt_ms as f32 / 1_000.0);
            if rate >= crate::platform::mission_profile::leak_rate_atm_s() {
                return Some("rapid leak");
            }
        }
    }

    if last - sample.pressure_atm >= PRESSURE_DROP_ATM {
        return Some("pressure drop");
    }

    None
}

pub fn read_bmp390_pressure() -> f32 {
    read_env_sample().pressure_atm
}

pub fn read_ads1115_dose() -> u32 {
    read_env_sample().dose_scaled
}

pub fn read_env_sample() -> EnvSample {
    let mut pressure = 0.0f32;
    let mut temp = 0.0f32;
    let mut raw_sum = 0u32;
    let mut n = 0u32;

    for _ in 0..SENSOR_SAMPLES {
        if let Some((p, t)) = read_bmp390_env_once() {
            pressure += p;
            temp += t;
            n += 1;
        }
        raw_sum = raw_sum.saturating_add(read_ads1115_raw_once());
        feed_watchdog();
        Delay::new().delay_millis(5);
    }

    let count = n.max(1) as f32;
    let raw_avg = raw_sum / SENSOR_SAMPLES as u32;
    EnvSample {
        pressure_atm: pressure / count,
        temp_c: temp / count,
        dose_raw: raw_avg,
        dose_scaled: rtc_state::scale_dose(raw_avg),
    }
}

pub fn run_review_browser_if_requested() {
    with_platform(|state| super::event_browser::run_if_requested(state));
}

pub fn request_deep_sleep() -> ! {
    feed_watchdog();
    if !rtc_state::breach_latched() {
        status_led_off();
    }

    let mut guard = PLATFORM.lock();
    if let Some(mut state) = guard.take() {
        state.wdt.feed();
        arm_bmp390_sleep_interrupt(&mut state);
        let delay = Delay::new();
        let secs = rtc_state::wake_timer_secs();
        let wakeup_pins: &mut [(&mut dyn RtcPinWithResistors, WakeupLevel)] = &mut [
            (&mut state.wake_gpio, WakeupLevel::High),
            (&mut state.bmp_int_gpio, WakeupLevel::Low),
        ];
        let ext1_wake = Ext1WakeupSource::new(wakeup_pins);
        let config = RtcSleepConfig::deep();
        delay.delay_millis(100);
        if crate::platform::mission_profile::interval_wake_enabled() {
            let timer = TimerWakeupSource::new(CoreDuration::from_secs(secs));
            state.rtc.sleep(
                &config,
                &[&timer as &dyn WakeSource, &ext1_wake as &dyn WakeSource],
            );
        } else {
            state.rtc.sleep(&config, &[&ext1_wake as &dyn WakeSource]);
        }
    }
    loop {}
}

fn read_ads1115_raw_avg() -> u32 {
    let mut sum = 0u32;
    for _ in 0..SENSOR_SAMPLES {
        sum = sum.saturating_add(read_ads1115_raw_once());
        Delay::new().delay_millis(5);
    }
    sum / SENSOR_SAMPLES as u32
}

fn read_ads1115_raw_once() -> u32 {
    with_platform(|state| {
        if !state.health.ads1115 || state.ads_addr == 0 {
            return 0;
        }
        state.wdt.feed();
        read_ads1115_inner(state).unwrap_or(0)
    })
    .unwrap_or(0)
}

fn read_bmp390_env_once() -> Option<(f32, f32)> {
    with_platform(|state| {
        if !state.health.bmp390 || state.bmp_addr == 0 {
            return None;
        }
        state.wdt.feed();
        read_bmp390_env_inner(state).ok()
    })
    .flatten()
}

fn i2c_write(state: &mut PlatformState, addr: u8, bytes: &[u8]) -> Result<(), ()> {
    state.i2c.write(addr, bytes).map_err(|_| ())
}

fn i2c_write_read(
    state: &mut PlatformState,
    addr: u8,
    write: &[u8],
    read: &mut [u8],
) -> Result<(), ()> {
    state.i2c.write_read(addr, write, read).map_err(|_| ())
}

fn init_bmp390(state: &mut PlatformState, addr: u8) -> Result<(), ()> {
    let mut id = [0u8];
    i2c_write_read(state, addr, &[BMP390_REG_CHIP_ID], &mut id)?;
    if id[0] != BMP390_CHIP_ID {
        return Err(());
    }
    i2c_write(state, addr, &[BMP390_REG_CMD, BMP390_SOFT_RESET])?;
    Delay::new().delay_millis(100);
    i2c_write(state, addr, &[BMP390_REG_PWR_CTRL, BMP390_PWR_NORMAL])?;
    i2c_write(state, addr, &[BMP390_REG_ODR, BMP390_ODR_SLEEP])?;
    Delay::new().delay_millis(20);
    let mut nvm = [0u8; 21];
    i2c_write_read(state, addr, &[BMP390_REG_CALIB], &mut nvm)?;
    state.bmp_calib = parse_bmp390_calib(&nvm);
    state.bmp_addr = addr;
    Ok(())
}

fn configure_bmp390_interrupt(state: &mut PlatformState) -> Result<(), ()> {
    let addr = state.bmp_addr;
    i2c_write(state, addr, &[BMP390_REG_INT_CTRL, BMP390_INT_DRDY])?;
    clear_bmp390_interrupt_on(state);
    Ok(())
}

fn arm_bmp390_sleep_interrupt(state: &mut PlatformState) {
    if !state.health.bmp390 || state.bmp_addr == 0 {
        return;
    }
    let _ = configure_bmp390_interrupt(state);
    let _ = i2c_write(state, state.bmp_addr, &[BMP390_REG_ODR, BMP390_ODR_SLEEP]);
    clear_bmp390_interrupt_on(state);
}

fn clear_bmp390_interrupt_on(state: &mut PlatformState) {
    if state.bmp_addr == 0 {
        return;
    }
    let mut status = [0u8];
    let _ = i2c_write_read(state, state.bmp_addr, &[BMP390_REG_INT_STATUS], &mut status);
}

fn clear_bmp390_interrupt() {
    with_platform(clear_bmp390_interrupt_on);
}

fn bmp390_interrupt_pending() -> bool {
    with_platform(|state| {
        if !state.health.bmp390_int || state.bmp_addr == 0 {
            return false;
        }
        let mut status = [0u8];
        if i2c_write_read(state, state.bmp_addr, &[BMP390_REG_INT_STATUS], &mut status).is_err() {
            return false;
        }
        status[0] & 0x08 != 0
    })
    .unwrap_or(false)
}

fn button_pressed_now() -> bool {
    with_platform(|state| {
        let button = Input::new(
            state.wake_gpio.reborrow(),
            InputConfig::default().with_pull(Pull::Down),
        );
        button.is_high()
    })
    .unwrap_or(false)
}

fn init_ads1115(state: &mut PlatformState, addr: u8) -> Result<(), ()> {
    let cfg = ADS1115_CFG_AIN0;
    let buf = [ADS1115_REG_CONFIG, (cfg >> 8) as u8, (cfg & 0xFF) as u8];
    i2c_write(state, addr, &buf)?;
    state.ads_addr = addr;
    Ok(())
}

fn read_bmp390_env_inner(state: &mut PlatformState) -> Result<(f32, f32), ()> {
    let addr = state.bmp_addr;
    let mut raw = [0u8; 6];
    i2c_write_read(state, addr, &[BMP390_REG_DATA], &mut raw)?;
    let press_raw = u32::from(raw[0]) | (u32::from(raw[1]) << 8) | (u32::from(raw[2]) << 16);
    let temp_raw = u32::from(raw[3]) | (u32::from(raw[4]) << 8) | (u32::from(raw[5]) << 16);
    let press_uncomp = (press_raw & 0x00FF_FFFF) as u64;
    let temp_uncomp = (temp_raw & 0x00FF_FFFF) as u64;
    let t_lin = compensate_bmp390_temp(temp_uncomp, &state.bmp_calib);
    let pa = compensate_bmp390_press(press_uncomp, t_lin, &state.bmp_calib);
    let t_c = t_lin as f32;
    Ok(((pa / 101_325.0) as f32, t_c))
}

fn read_ads1115_inner(state: &mut PlatformState) -> Result<u32, ()> {
    let addr = state.ads_addr;
    let cfg = ADS1115_CFG_AIN0 | 0x8000;
    i2c_write(
        state,
        addr,
        &[
            ADS1115_REG_CONFIG,
            (cfg >> 8) as u8,
            (cfg & 0xFF) as u8,
        ],
    )?;
    Delay::new().delay_millis(10);
    let mut conv = [0u8; 2];
    i2c_write_read(state, addr, &[ADS1115_REG_CONVERSION], &mut conv)?;
    Ok(i16::from_be_bytes(conv).unsigned_abs() as u32)
}

fn parse_bmp390_calib(nvm: &[u8; 21]) -> Bmp390Calib {
    Bmp390Calib {
        par_t1: u16::from_le_bytes([nvm[0], nvm[1]]),
        par_t2: u16::from_le_bytes([nvm[2], nvm[3]]),
        par_t3: nvm[4] as i8,
        par_p1: i16::from_le_bytes([nvm[5], nvm[6]]),
        par_p2: i16::from_le_bytes([nvm[7], nvm[8]]),
        par_p3: nvm[9] as i8,
        par_p4: nvm[10] as i8,
        par_p5: u16::from_le_bytes([nvm[11], nvm[12]]),
        par_p6: u16::from_le_bytes([nvm[13], nvm[14]]),
        par_p7: nvm[15] as i8,
        par_p8: nvm[16] as i8,
        par_p9: i16::from_le_bytes([nvm[17], nvm[18]]),
        par_p10: nvm[19] as i8,
        par_p11: nvm[20] as i8,
    }
}

fn compensate_bmp390_temp(uncomp: u64, cal: &Bmp390Calib) -> f64 {
    let partial1 = uncomp as f64 - (256.0 * f64::from(cal.par_t1));
    let partial2 = f64::from(cal.par_t2) * partial1;
    let partial3 = partial1 * partial1;
    let partial4 = partial3 * f64::from(cal.par_t3);
    let partial5 = partial2 * 262_144.0 + partial4;
    let partial6 = partial5 / 4_294_967_296.0;
    partial6 / 1_048_576.0
}

fn compensate_bmp390_press(uncomp: u64, t_lin: f64, cal: &Bmp390Calib) -> f64 {
    let partial1 = cal.par_p6 as f64 * t_lin;
    let partial2 = cal.par_p7 as f64 * t_lin * t_lin;
    let partial3 = cal.par_p8 as f64 * t_lin * t_lin * t_lin;
    let partial4 = partial1 + partial2 + partial3;
    let partial5 = cal.par_p5 as f64 + partial4;
    let partial6 = cal.par_p4 as f64 * t_lin;
    let partial7 = cal.par_p3 as f64 * t_lin * t_lin;
    let partial8 = cal.par_p2 as f64 * t_lin * t_lin * t_lin;
    let partial9 = cal.par_p1 as f64 + partial6 + partial7 + partial8;
    let partial10 = uncomp as f64 - partial9;
    let partial11 = partial10 * (partial5 / 4_194_304.0);
    let partial12 = partial11 * partial11;
    let partial13 = partial12 * partial11;
    let partial14 = partial13 * (cal.par_p11 as f64 / 281_474_976_710_656.0);
    let partial15 = partial12 * (cal.par_p10 as f64 / 281_474_976_710_656.0);
    let partial16 = partial11 * partial11 * (cal.par_p9 as f64 / 137_438_953_472.0);
    let partial17 = partial13 * (cal.par_p8 as f64 / 34_359_738_368.0);
    let partial18 = partial12 * (cal.par_p7 as f64 / 1_073_741_824.0);
    let partial19 = partial11 * (cal.par_p6 as f64 / 33_554_432.0);
    let partial20 = partial14 + partial15 + partial16 + partial17 + partial18 + partial19;
    partial20 * 100.0
}
