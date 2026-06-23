//! SSD1306 128×64 I2C display (address 0x3C / 0x3D). Shares the sensor I2C bus.

use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use esp_hal::delay::Delay;

use super::esp32c6::with_i2c_bus;
use super::font5x7;

const ADDR_PRIMARY: u8 = 0x3C;
const ADDR_SECONDARY: u8 = 0x3D;

static READY: AtomicBool = AtomicBool::new(false);
static ADDR: AtomicU8 = AtomicU8::new(0);

fn cmd(bytes: &[u8]) -> Result<(), ()> {
    let addr = ADDR.load(Ordering::Acquire);
    if addr == 0 {
        return Err(());
    }
    let mut buf = [0u8; 16];
    for chunk in bytes.chunks(15) {
        buf[0] = 0x00;
        buf[1..1 + chunk.len()].copy_from_slice(chunk);
        with_i2c_bus(|i2c| i2c.write(addr, &buf[..chunk.len() + 1]).map_err(|_| ())).ok_or(())??;
    }
    Ok(())
}

fn data(bytes: &[u8]) -> Result<(), ()> {
    let addr = ADDR.load(Ordering::Acquire);
    if addr == 0 {
        return Err(());
    }
    let mut buf = [0u8; 17];
    for chunk in bytes.chunks(16) {
        buf[0] = 0x40;
        buf[1..1 + chunk.len()].copy_from_slice(chunk);
        with_i2c_bus(|i2c| i2c.write(addr, &buf[..chunk.len() + 1]).map_err(|_| ())).ok_or(())??;
    }
    Ok(())
}

fn init_panel() -> Result<(), ()> {
    cmd(&[
        0xAE, 0xD5, 0x80, 0xA8, 0x3F, 0xD3, 0x00, 0x40, 0x8D, 0x14, 0x20, 0xA1, 0xC8, 0xDA,
        0x12, 0x81, 0xCF, 0xD9, 0xF1, 0xDB, 0x40, 0xA4, 0xA6, 0xAF,
    ])?;
    Ok(())
}

fn clear() -> Result<(), ()> {
    for page in 0u8..8 {
        cmd(&[0xB0 | page, 0x00, 0x10])?;
        data(&[0x00; 128])?;
    }
    Ok(())
}

fn draw_char(col: u8, page: u8, ch: u8) -> Result<(), ()> {
    let Some(g) = font5x7::glyph(ch) else {
        return Ok(());
    };
    cmd(&[0xB0 | page, col & 0x0F, 0x10 | (col >> 4)])?;
    data(&g)?;
    Ok(())
}

fn draw_str(mut col: u8, page: u8, s: &str) -> Result<(), ()> {
    for b in s.bytes() {
        draw_char(col, page, b)?;
        col = col.saturating_add(6);
        if col > 122 {
            break;
        }
    }
    Ok(())
}

fn format_hex8(out: &mut [u8; 8], v: u32) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for i in 0..8 {
        let n = ((v >> ((7 - i) * 4)) & 0xF) as usize;
        out[i] = HEX[n];
    }
}

fn format_dec(out: &mut [u8; 6], mut v: u32) {
    let mut tmp = [0u8; 6];
    let mut len = 0usize;
    if v == 0 {
        tmp[0] = b'0';
        len = 1;
    } else {
        while v > 0 && len < 6 {
            tmp[len] = b'0' + (v % 10) as u8;
            v /= 10;
            len += 1;
        }
    }
    for i in 0..len {
        out[i] = tmp[len - 1 - i];
    }
}

pub fn init() -> bool {
    for addr in [ADDR_PRIMARY, ADDR_SECONDARY] {
        if with_i2c_bus(|i2c| i2c.write(addr, &[0x00, 0xAE])).is_none() {
            continue;
        }
        ADDR.store(addr, Ordering::Release);
        if init_panel().is_ok() {
            let _ = clear();
            READY.store(true, Ordering::Release);
            return true;
        }
        ADDR.store(0, Ordering::Release);
    }
    false
}

pub fn is_ready() -> bool {
    READY.load(Ordering::Acquire)
}

pub fn show_boot(sensors_ok: bool) {
    if !is_ready() {
        return;
    }
    let _ = clear();
    let _ = draw_str(0, 0, "AETHER ENCLAVE");
    let _ = draw_str(0, 2, if sensors_ok { "SENSORS OK" } else { "SENSOR FAULT" });
    let _ = draw_str(0, 4, "DEFEXPO DEMO");
    Delay::new().delay_millis(5);
}

pub fn show_cycle(cycle: u32, guest: i32, proof: u64, vector: u8) {
    if !is_ready() {
        return;
    }
    let _ = clear();

    let mut num = [b' '; 6];
    format_dec(&mut num, cycle);
    let mut line1 = [0u8; 12];
    line1[..6].copy_from_slice(b"CYCLE ");
    line1[6..12].copy_from_slice(&num);
    let _ = draw_str(0, 0, core::str::from_utf8(&line1).unwrap_or("CYCLE"));

    let _ = draw_str(0, 2, super::demo::guest_flags_oled(guest));

    let mut lo = [0u8; 8];
    format_hex8(&mut lo, proof as u32);
    let mut line3 = [0u8; 13];
    line3[..5].copy_from_slice(b"PRF L");
    line3[5..13].copy_from_slice(&lo);
    let _ = draw_str(0, 4, core::str::from_utf8(&line3).unwrap_or("PRF"));

    let mut hi = [0u8; 8];
    format_hex8(&mut hi, (proof >> 32) as u32);
    let mut line4 = [0u8; 16];
    line4[..5].copy_from_slice(b"PRF H");
    line4[5..13].copy_from_slice(&hi);
    let mut vec = [0u8; 8];
    format_hex8(&mut vec, u32::from(vector));
    line4[13] = b'V';
    line4[14] = vec[6];
    line4[15] = vec[7];
    let _ = draw_str(0, 6, core::str::from_utf8(&line4).unwrap_or("VEC"));
    Delay::new().delay_millis(5);
}
