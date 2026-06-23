//! ESP32-C6 board support: I2C sensors (BMP390 + ADS1115), watchdog, RTC deep sleep.
//!
//! Log output goes through the on-chip USB Serial/JTAG port (see esp-println in mmio.rs).

use core::sync::atomic::{AtomicBool, Ordering};
use core::time::Duration as CoreDuration;

use esp_hal::delay::Delay;
use esp_hal::gpio::{InputConfig, Pull, RtcPinWithResistors};
use esp_hal::i2c::master::{Config as I2cConfig, I2c};
use esp_hal::peripherals::{GPIO2, TIMG0};
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

/// Seconds until the RTC timer fires wake vector 0x21 (adjust for demo pacing).
const WAKE_TIMER_SECS: u64 = 10;

/// Watchdog timeout — must exceed worst-case wasmi compile + instantiate on device.
const WDT_TIMEOUT_SECS: u64 = 30;

const BMP390_ADDR_PRIMARY: u8 = 0x76;
const BMP390_ADDR_SECONDARY: u8 = 0x77;
const ADS1115_ADDR_PRIMARY: u8 = 0x48;
const ADS1115_ADDR_SECONDARY: u8 = 0x49;

const BMP390_REG_CHIP_ID: u8 = 0x00;
const BMP390_REG_DATA: u8 = 0x04;
const BMP390_REG_CALIB: u8 = 0x31;
const BMP390_REG_PWR_CTRL: u8 = 0x1D;
const BMP390_REG_CMD: u8 = 0x7E;
const BMP390_CHIP_ID: u8 = 0x60;
const BMP390_SOFT_RESET: u8 = 0xB6;
const BMP390_PWR_NORMAL: u8 = 0x30;

const ADS1115_REG_CONVERSION: u8 = 0x00;
const ADS1115_REG_CONFIG: u8 = 0x01;
/// AIN0 vs GND, ±4.096 V, single-shot, 128 SPS.
const ADS1115_CFG_AIN0: u16 = 0xC583;

/// Which I2C devices responded during boot.
#[derive(Debug, Clone, Copy, Default)]
pub struct SensorHealth {
    /// BMP390 barometer probed and calibrated.
    pub bmp390: bool,
    /// ADS1115 ADC probed and configured.
    pub ads1115: bool,
    /// BMP390 7-bit I2C address in use (0 if absent).
    pub bmp390_addr: u8,
    /// ADS1115 7-bit I2C address in use (0 if absent).
    pub ads1115_addr: u8,
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

struct PlatformState {
    i2c: I2c<'static, Blocking>,
    rtc: Rtc<'static>,
    wdt: Wdt<TIMG0<'static>>,
    wake_gpio: GPIO2<'static>,
    bmp_calib: Bmp390Calib,
    bmp_addr: u8,
    ads_addr: u8,
    health: SensorHealth,
}

static PLATFORM: Mutex<Option<PlatformState>> = Mutex::new(None);
static READY: AtomicBool = AtomicBool::new(false);

fn with_platform<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut PlatformState) -> R,
{
    if !READY.load(Ordering::Acquire) {
        return None;
    }
    if let Some(state) = PLATFORM.lock().as_mut() {
        Some(f(state))
    } else {
        None
    }
}

/// Boot: watchdog, I2C @ 400 kHz, probe BMP390 + ADS1115, arm GPIO2 wake.
///
/// Missing sensors do not panic — reads fall back to 0 and boot continues.
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
    .with_sda(peripherals.GPIO8)
    .with_scl(peripherals.GPIO9);

    let mut wake_gpio = peripherals.GPIO2;
    let _ = esp_hal::gpio::Input::new(
        wake_gpio.reborrow(),
        InputConfig::default().with_pull(Pull::Down),
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
        bmp_calib: Bmp390Calib::default(),
        bmp_addr: 0,
        ads_addr: 0,
        health: SensorHealth::default(),
    };

    for addr in [BMP390_ADDR_PRIMARY, BMP390_ADDR_SECONDARY] {
        if init_bmp390(&mut probe, addr).is_ok() {
            health.bmp390 = true;
            health.bmp390_addr = addr;
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

    health
}

/// Last boot-time sensor probe result.
pub fn sensor_health() -> SensorHealth {
    with_platform(|state| state.health)
        .unwrap_or_default()
}

pub fn feed_watchdog() {
    with_platform(|state| {
        state.wdt.feed();
    });
}

/// Map RTC wake cause to the same vector IDs used on x86 (0x20 / 0x21).
pub fn detect_wake_trigger() -> Option<HardwareInterrupt> {
    match system::wakeup_cause() {
        SleepSource::Timer => Some(HardwareInterrupt::KineticJointActuation),
        SleepSource::Gpio | SleepSource::Ext0 | SleepSource::Ext1 => {
            Some(HardwareInterrupt::AtmosphericPressureThreshold)
        }
        _ => None,
    }
}

pub fn read_bmp390_pressure() -> f32 {
    with_platform(|state| {
        if !state.health.bmp390 || state.bmp_addr == 0 {
            return 0.0;
        }
        state.wdt.feed();
        match read_bmp390_pressure_inner(state) {
            Ok(p) => p,
            Err(_) => 0.0,
        }
    })
    .unwrap_or(0.0)
}

pub fn read_ads1115_dose() -> u32 {
    with_platform(|state| {
        if !state.health.ads1115 || state.ads_addr == 0 {
            return 0;
        }
        state.wdt.feed();
        match read_ads1115_inner(state) {
            Ok(v) => v,
            Err(_) => 0,
        }
    })
    .unwrap_or(0)
}

/// Deep sleep until GPIO2 goes high or the timer fires, then CPU resets and boots again.
pub fn request_deep_sleep() -> ! {
    feed_watchdog();

    let mut guard = PLATFORM.lock();
    if let Some(mut state) = guard.take() {
        state.wdt.feed();

        let delay = Delay::new();
        let timer = TimerWakeupSource::new(CoreDuration::from_secs(WAKE_TIMER_SECS));
        let wakeup_pins: &mut [(&mut dyn RtcPinWithResistors, WakeupLevel)] =
            &mut [(&mut state.wake_gpio, WakeupLevel::High)];

        let ext1_wake = Ext1WakeupSource::new(wakeup_pins);
        let config = RtcSleepConfig::deep();

        delay.delay_millis(100);
        state.rtc.sleep(&config, &[&timer as &dyn WakeSource, &ext1_wake as &dyn WakeSource]);
    }

    loop {}
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
    state
        .i2c
        .write_read(addr, write, read)
        .map_err(|_| ())
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
    Delay::new().delay_millis(20);

    let mut nvm = [0u8; 21];
    i2c_write_read(state, addr, &[BMP390_REG_CALIB], &mut nvm)?;
    state.bmp_calib = parse_bmp390_calib(&nvm);
    state.bmp_addr = addr;
    Ok(())
}

fn init_ads1115(state: &mut PlatformState, addr: u8) -> Result<(), ()> {
    let cfg = ADS1115_CFG_AIN0;
    let buf = [ADS1115_REG_CONFIG, (cfg >> 8) as u8, (cfg & 0xFF) as u8];
    i2c_write(state, addr, &buf)?;
    state.ads_addr = addr;
    Ok(())
}

fn read_bmp390_pressure_inner(state: &mut PlatformState) -> Result<f32, ()> {
    let addr = state.bmp_addr;
    let mut raw = [0u8; 6];
    i2c_write_read(state, addr, &[BMP390_REG_DATA], &mut raw)?;

    let press_raw = u32::from(raw[0]) | (u32::from(raw[1]) << 8) | (u32::from(raw[2]) << 16);
    let temp_raw = u32::from(raw[3]) | (u32::from(raw[4]) << 8) | (u32::from(raw[5]) << 16);
    let press_uncomp = (press_raw & 0x00FF_FFFF) as u64;
    let temp_uncomp = (temp_raw & 0x00FF_FFFF) as u64;

    let t_lin = compensate_bmp390_temp(temp_uncomp, &state.bmp_calib);
    let pa = compensate_bmp390_press(press_uncomp, t_lin, &state.bmp_calib);
    Ok((pa / 101_325.0) as f32)
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
    let raw = i16::from_be_bytes(conv);
    Ok(raw.unsigned_abs() as u32)
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
