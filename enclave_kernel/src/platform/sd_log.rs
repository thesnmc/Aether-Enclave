//! microSD SPI proof log — append-only sectors, no FAT.
//!
//! Wiring (common 6-pin SPI module):
//!   MOSI → GPIO3, MISO → GPIO4, SCK → GPIO5, CS → GPIO15, 3.3 V, GND
//!
//! Log lives at sector 2048+ so a FAT-formatted card's MBR/partition table is
//! usually untouched. Use a dedicated card for field trials.

use core::fmt::Write;

use esp_hal::delay::Delay;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::spi::master::{Config as SpiConfig, Spi};
use esp_hal::spi::Mode;
use esp_hal::time::Rate;
use esp_hal::Blocking;
use spin::Mutex;

use crate::platform::demo;
use crate::platform::esp32c6;

const CMD0: u8 = 0;
const CMD8: u8 = 8;
const CMD16: u8 = 16;
const CMD17: u8 = 17;
const CMD24: u8 = 24;
const CMD55: u8 = 55;
const CMD58: u8 = 58;
const ACMD41: u8 = 41;

const LOG_META_SECTOR: u32 = 2048;
const LOG_FIRST_SECTOR: u32 = 2049;
const LOG_SECTOR_COUNT: u32 = 512;

struct SdState {
    spi: Spi<'static, Blocking>,
    cs: Output<'static>,
    is_sdhc: bool,
}

static SD: Mutex<Option<SdState>> = Mutex::new(None);

/// Init SPI + probe card. Returns `true` when a card responds.
pub fn init(
    spi_periph: esp_hal::peripherals::SPI2<'static>,
    mosi: esp_hal::peripherals::GPIO3<'static>,
    miso: esp_hal::peripherals::GPIO4<'static>,
    sck: esp_hal::peripherals::GPIO5<'static>,
    cs_pin: esp_hal::peripherals::GPIO15<'static>,
) -> bool {
    let cs = Output::new(cs_pin, Level::High, OutputConfig::default());
    let spi = match Spi::new(
        spi_periph,
        SpiConfig::default()
            .with_frequency(Rate::from_khz(400))
            .with_mode(Mode::_0),
    ) {
        Ok(s) => s,
        Err(_) => return false,
    }
    .with_sck(sck)
    .with_mosi(mosi)
    .with_miso(miso);

    let mut state = SdState {
        spi,
        cs,
        is_sdhc: false,
    };

    if sd_init(&mut state).is_ok() {
        *SD.lock() = Some(state);
        true
    } else {
        false
    }
}

/// Append one cycle line to the on-card log (512-byte sector).
pub fn log_cycle(
    cycle: u32,
    guest: i32,
    proof: u64,
    vector: u8,
    pressure: f32,
    temp_c: f32,
    dose: u32,
    proof_changed: bool,
) -> bool {
    let mut guard = SD.lock();
    let Some(state) = guard.as_mut() else {
        return false;
    };

    let mut meta = [0u8; 512];
    if read_block(state, LOG_META_SECTOR, &mut meta).is_err() {
        return false;
    }

    let mut next = if meta[0..4] == *b"AETH" {
        u32::from_le_bytes([meta[4], meta[5], meta[6], meta[7]])
    } else {
        meta[0..4].copy_from_slice(b"AETH");
        meta[4..8].copy_from_slice(&LOG_FIRST_SECTOR.to_le_bytes());
        meta[8..12].copy_from_slice(&0u32.to_le_bytes());
        if write_block(state, LOG_META_SECTOR, &meta).is_err() {
            return false;
        }
        LOG_FIRST_SECTOR
    };

    if next < LOG_FIRST_SECTOR || next >= LOG_FIRST_SECTOR + LOG_SECTOR_COUNT {
        next = LOG_FIRST_SECTOR;
    }

    let mut sector = [0xFFu8; 512];
    sector[0..4].copy_from_slice(b"AEC1");
    let mut writer = ByteWriter::new(&mut sector[4..]);
    let _ = write!(
        writer,
        "cycle={} guest={} flags={} proof=0x{:016X} vector=0x{:02X} P={:.3} T={:.1} D={} changed={}\n",
        cycle,
        guest,
        demo::guest_flags_text(guest),
        proof,
        vector,
        pressure,
        temp_c,
        dose,
        if proof_changed { 1 } else { 0 },
    );

    if write_block(state, next, &sector).is_err() {
        return false;
    }

    next += 1;
    if next >= LOG_FIRST_SECTOR + LOG_SECTOR_COUNT {
        next = LOG_FIRST_SECTOR;
    }
    meta[4..8].copy_from_slice(&next.to_le_bytes());
    let count = u32::from_le_bytes([meta[8], meta[9], meta[10], meta[11]]).saturating_add(1);
    meta[8..12].copy_from_slice(&count.to_le_bytes());
    write_block(state, LOG_META_SECTOR, &meta).is_ok()
}

fn sd_init(state: &mut SdState) -> Result<(), ()> {
    cs_high(state);
    idle_clocks(state, 10);

    let r0 = send_cmd(state, CMD0, 0)?;
    if r0 != 0x01 {
        return Err(());
    }

    let r8 = send_cmd(state, CMD8, 0x0000_01AA)?;
    if r8 == 0x01 {
        let mut tail = [0u8; 4];
        read_bytes(state, &mut tail)?;
        if tail[2] != 0x01 || tail[3] != 0xAA {
            return Err(());
        }
    }

    let mut ready = false;
    for _ in 0..200 {
        send_cmd(state, CMD55, 0)?;
        let r = send_cmd(state, ACMD41, 0x4000_0000)?;
        if r & 0x01 == 0 {
            ready = true;
            break;
        }
        Delay::new().delay_millis(10);
        esp32c6::feed_watchdog();
    }
    if !ready {
        return Err(());
    }

    send_cmd(state, CMD58, 0)?;
    let ocr = read_r3(state)?;
    state.is_sdhc = (ocr & 0x4000_0000) != 0;

    if !state.is_sdhc {
        send_cmd(state, CMD16, 512)?;
    }

    Ok(())
}

fn send_cmd(state: &mut SdState, cmd: u8, arg: u32) -> Result<u8, ()> {
    cs_low(state);
    let crc = match cmd {
        CMD0 => 0x95,
        CMD8 => 0x87,
        _ => 0xFF,
    };
    let packet = [
        0x40 | cmd,
        (arg >> 24) as u8,
        (arg >> 16) as u8,
        (arg >> 8) as u8,
        arg as u8,
        crc,
    ];
    write_bytes(state, &packet)?;
    let resp = read_response(state, 8)?;
    cs_high(state);
    idle_clocks(state, 1);
    Ok(resp)
}

fn read_r3(state: &mut SdState) -> Result<u32, ()> {
    let mut ocr = [0u8; 4];
    read_bytes(state, &mut ocr)?;
    Ok(u32::from_be_bytes(ocr))
}

fn read_block(state: &mut SdState, sector: u32, buf: &mut [u8; 512]) -> Result<(), ()> {
    let addr = block_arg(state, sector);
    send_cmd(state, CMD17, addr)?;
    cs_low(state);
    if !wait_data_token(state)? {
        cs_high(state);
        return Err(());
    }
    read_bytes(state, buf)?;
    let mut crc = [0u8; 2];
    read_bytes(state, &mut crc)?;
    cs_high(state);
    idle_clocks(state, 1);
    Ok(())
}

fn write_block(state: &mut SdState, sector: u32, data: &[u8; 512]) -> Result<(), ()> {
    let addr = block_arg(state, sector);
    send_cmd(state, CMD24, addr)?;
    cs_low(state);
    write_bytes(state, &[0xFF, 0xFE])?;
    write_bytes(state, data)?;
    write_bytes(state, &[0xFF, 0xFF])?;
    let mut token = [0xFF];
    read_bytes(state, &mut token)?;
    cs_high(state);
    idle_clocks(state, 1);
    if (token[0] & 0x1F) != 0x05 {
        return Err(());
    }
    wait_not_busy(state)?;
    Ok(())
}

fn block_arg(state: &SdState, sector: u32) -> u32 {
    if state.is_sdhc {
        sector
    } else {
        sector.saturating_mul(512)
    }
}

fn wait_data_token(state: &mut SdState) -> Result<bool, ()> {
    for _ in 0..100_000 {
        let mut b = [0xFF];
        read_bytes(state, &mut b)?;
        if b[0] == 0xFE {
            return Ok(true);
        }
        if b[0] != 0xFF {
            return Ok(false);
        }
        esp32c6::feed_watchdog();
    }
    Err(())
}

fn wait_not_busy(state: &mut SdState) -> Result<(), ()> {
    for _ in 0..100_000 {
        let mut b = [0xFF];
        read_bytes(state, &mut b)?;
        if b[0] == 0xFF {
            return Ok(());
        }
        esp32c6::feed_watchdog();
    }
    Err(())
}

fn read_response(state: &mut SdState, max: u8) -> Result<u8, ()> {
    for _ in 0..max {
        let mut b = [0xFF];
        read_bytes(state, &mut b)?;
        if b[0] != 0xFF {
            return Ok(b[0]);
        }
    }
    Err(())
}

fn cs_low(state: &mut SdState) {
    state.cs.set_low();
}

fn cs_high(state: &mut SdState) {
    state.cs.set_high();
}

fn idle_clocks(state: &mut SdState, bytes: u8) {
    let pad = [0xFF; 16];
    cs_high(state);
    for _ in 0..bytes {
        let _ = write_bytes(state, &pad[..1]);
    }
}

fn write_bytes(state: &mut SdState, data: &[u8]) -> Result<(), ()> {
    state.spi.write(data).map_err(|_| ())
}

fn read_bytes(state: &mut SdState, data: &mut [u8]) -> Result<(), ()> {
    for byte in data.iter_mut() {
        *byte = 0xFF;
    }
    state.spi.transfer(data).map_err(|_| ())
}

struct ByteWriter<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> ByteWriter<'a> {
    fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }
}

impl Write for ByteWriter<'_> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for b in s.bytes() {
            if self.pos >= self.buf.len() {
                break;
            }
            self.buf[self.pos] = b;
            self.pos += 1;
        }
        Ok(())
    }
}
