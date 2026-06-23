//! SSD1306 128×64 I2C display — Aether Enclave boot / cycle / shutdown animations.

use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use esp_hal::delay::Delay;

use super::esp32c6::{feed_watchdog, with_i2c_bus};
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

fn display_on(on: bool) -> Result<(), ()> {
    cmd(&[if on { 0xAF } else { 0xAE }])
}

fn invert(on: bool) -> Result<(), ()> {
    cmd(&[if on { 0xA7 } else { 0xA6 }])
}

fn draw_char(col: u8, page: u8, ch: u8) -> Result<(), ()> {
    let Some(g) = font5x7::glyph(ch) else {
        return Ok(());
    };
    cmd(&[0xB0 | page, col & 0x0F, 0x10 | (col >> 4)])?;
    data(&g)?;
    Ok(())
}

fn str_pixel_width(s: &str) -> u8 {
    s.bytes().fold(0u8, |w, b| w.saturating_add(if font5x7::glyph(b).is_some() { 6 } else { 0 }))
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

fn draw_str_centered(page: u8, s: &str) -> Result<(), ()> {
    let w = str_pixel_width(s);
    let col = 128u8.saturating_sub(w) / 2;
    draw_str(col, page, s)
}

fn draw_hbar(page: u8, x: u8, total_w: u8, filled: u8) -> Result<(), ()> {
    let filled = filled.min(total_w);
    cmd(&[0xB0 | page, x & 0x0F, 0x10 | (x >> 4)])?;
    let mut row = [0u8; 128];
    for i in 0..total_w as usize {
        row[i] = if (i as u8) < filled { 0xFF } else { 0x00 };
    }
    data(&row[..total_w as usize])?;
    Ok(())
}

fn draw_frame(page_top: u8, page_bot: u8) -> Result<(), ()> {
    for page in page_top..=page_bot {
        cmd(&[0xB0 | page, 0x00, 0x10])?;
        let mut row = [0u8; 128];
        row[0] = 0xFF;
        row[127] = 0xFF;
        if page == page_top || page == page_bot {
            row.fill(0xFF);
        }
        data(&row)?;
    }
    Ok(())
}

fn pause(ms: u32) {
    feed_watchdog();
    Delay::new().delay_millis(ms);
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

fn is_ready() -> bool {
    READY.load(Ordering::Acquire)
}

/// True when SSD1306 responded on I2C.
pub fn panel_ready() -> bool {
    is_ready()
}

fn play_boot_intro() {
    let _ = clear();
    for _ in 0..2 {
        let _ = draw_frame(1, 5);
        let _ = draw_str_centered(2, "AETHER ENCLAVE");
        let _ = draw_str_centered(4, "WAKE WITNESS NODE");
        pause(110);
        let _ = invert(true);
        pause(65);
        let _ = invert(false);
        pause(65);
    }

    for step in 0..=22u8 {
        let _ = clear();
        let _ = draw_str_centered(1, "AETHER ENCLAVE");
        let _ = draw_str_centered(3, "BOOT SEQUENCE");
        let filled = step.saturating_mul(4).min(100);
        let _ = draw_hbar(5, 14, 100, filled);
        let _ = draw_str(40, 6, "BOOTING");
        pause(32);
    }

    for frame in 0..=12u8 {
        let _ = clear();
        let offset = 128u8.saturating_sub(frame.saturating_mul(10));
        let _ = draw_str(offset.min(76), 2, "AETHER ENCLAVE");
        if frame >= 5 {
            let _ = draw_str_centered(5, "THE SNMC");
        }
        pause(38);
    }
}

fn draw_boot_status(sensors_ok: bool) {
    let _ = clear();
    let _ = draw_str_centered(0, "AETHER ENCLAVE");
    let _ = draw_str_centered(2, if sensors_ok { "SENSORS THEEK" } else { "SENSOR FAULT" });
    let _ = draw_str_centered(4, "iDEX OPEN DEMO");
    let _ = draw_str_centered(6, if sensors_ok { "READY / TAIYAR" } else { "DEGRADED MODE" });
}

/// Boot splash — logo pulse, progress bar, slide-in.
pub fn play_boot_splash(sensors_ok: bool) {
    if !is_ready() {
        return;
    }
    play_boot_intro();
    draw_boot_status(sensors_ok);
    pause(5);
}

pub fn show_boot(sensors_ok: bool) {
    play_boot_splash(sensors_ok);
}

/// Shutdown splash before deep sleep — drain bar, wipe message, fade off.
pub fn play_shutdown_splash(sleep_secs: u64) {
    if !is_ready() {
        return;
    }

    let _ = clear();
    let _ = draw_str_centered(1, "AETHER ENCLAVE");
    let _ = draw_str_centered(3, "CYCLE COMPLETE");
    pause(120);

    for step in (0..=18u8).rev() {
        let _ = clear();
        let _ = draw_str_centered(0, "AETHER ENCLAVE");
        let _ = draw_str_centered(2, "WIPE RAM");
        let filled = step.saturating_mul(5);
        let _ = draw_hbar(4, 14, 100, filled);
        let _ = draw_str_centered(6, "SHUTTING DOWN");
        pause(28);
    }

    let _ = clear();
    let _ = draw_str_centered(2, "AETHER ENCLAVE");
    let secs = sleep_secs.min(99) as u8;
    let mut line = [0u8; 14];
    line[..10].copy_from_slice(b"SLEEP IN S");
    line[10] = b'0' + secs / 10;
    line[11] = b'0' + secs % 10;
    let _ = draw_str_centered(4, core::str::from_utf8(&line[..12]).unwrap_or("SLEEP"));
    let _ = draw_str_centered(6, "NIDRALIN");
    pause(200);

    for _ in 0..3 {
        let _ = invert(true);
        pause(45);
        let _ = invert(false);
        pause(45);
    }

    let _ = display_on(false);
}

pub fn show_cycle(cycle: u32, guest: i32, proof: u64, vector: u8, chain_linked: bool) {
    if !is_ready() {
        return;
    }

    if chain_linked {
        for _ in 0..2 {
            let _ = clear();
            let _ = draw_str_centered(2, "AETHER ENCLAVE");
            let _ = draw_str_centered(4, "PROOF LINKED");
            pause(85);
            let _ = invert(true);
            pause(45);
            let _ = invert(false);
            pause(45);
        }
        let _ = display_on(true);
    }

    let _ = clear();

    let mut num = [b' '; 6];
    format_dec(&mut num, cycle);
    let mut line1 = [0u8; 14];
    line1[..6].copy_from_slice(b"CYCLE ");
    line1[6..12].copy_from_slice(&num);
    let _ = draw_str(0, 0, core::str::from_utf8(&line1).unwrap_or("CYCLE"));

    let _ = draw_str(0, 2, super::demo::guest_flags_oled(guest));

    let mut lo = [0u8; 8];
    format_hex8(&mut lo, proof as u32);
    let mut line3 = [0u8; 16];
    line3[..5].copy_from_slice(b"PRF L");
    line3[5..13].copy_from_slice(&lo);
    let chain = if chain_linked { "LNK" } else { "RPT" };
    line3[13..16].copy_from_slice(chain.as_bytes());
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
    pause(5);
}

/// Full-screen breach alert — stays visible; GPIO10 latched ON until ACK.
pub fn show_breach_alert(cycle: u32, guest: i32) {
    if !is_ready() {
        return;
    }
    let _ = display_on(true);
    for _ in 0..6 {
        let _ = clear();
        let _ = invert(true);
        let _ = draw_str_centered(0, "** ALERT **");
        let _ = draw_str_centered(2, super::demo::guest_flags_oled(guest));
        let mut num = [b' '; 6];
        format_dec(&mut num, cycle);
        let mut line = [0u8; 14];
        line[..6].copy_from_slice(b"CYCLE ");
        line[6..12].copy_from_slice(&num);
        let _ = draw_str_centered(4, core::str::from_utf8(&line).unwrap_or("CYCLE"));
        let _ = draw_str_centered(6, "GPIO2=ACK");
        pause(120);
        let _ = invert(false);
        pause(80);
    }
}

/// Remind operator on wake that breach is still latched.
pub fn show_breach_reminder(guest: i32) {
    if !is_ready() {
        return;
    }
    let _ = clear();
    let _ = draw_str_centered(0, "ALERT ACTIVE");
    let _ = draw_str_centered(2, super::demo::guest_flags_oled(guest));
    let _ = draw_str_centered(5, "GPIO2 ACK");
    pause(250);
}

/// Operator UI — one stored event (GPIO9 scrolls).
pub fn show_event_record(index: usize, total: usize, ev: &super::event_log::EventRecord) {
    if !is_ready() {
        return;
    }
    let _ = clear();
    let _ = draw_str_centered(0, "EVENT LOG");

    let mut idx = [b' '; 6];
    format_dec(&mut idx, (index + 1) as u32);
    let mut tot = [b' '; 6];
    format_dec(&mut tot, total as u32);
    let mut line0 = [0u8; 16];
    line0[..4].copy_from_slice(b"PG ");
    line0[4] = idx[0];
    line0[5] = b'/';
    line0[6] = tot[0];
    let _ = draw_str(0, 2, core::str::from_utf8(&line0[..7]).unwrap_or("PG"));

    let mut num = [b' '; 6];
    format_dec(&mut num, ev.cycle);
    let mut line1 = [0u8; 14];
    line1[..6].copy_from_slice(b"CYC ");
    line1[6..12].copy_from_slice(&num);
    let _ = draw_str(0, 3, core::str::from_utf8(&line1).unwrap_or("CYC"));

    let _ = draw_str(0, 4, super::demo::guest_flags_oled(ev.guest));

    let mut lo = [0u8; 8];
    format_hex8(&mut lo, ev.proof as u32);
    let mut line3 = [0u8; 14];
    line3[..4].copy_from_slice(b"PRF ");
    line3[4..12].copy_from_slice(&lo);
    let _ = draw_str(0, 5, core::str::from_utf8(&line3).unwrap_or("PRF"));

    let p = f32::from_bits(ev.pressure_bits);
    let mut pnum = [b' '; 6];
    format_dec(&mut pnum, (p * 1000.0) as u32);
    let mut pline = [0u8; 16];
    pline[..2].copy_from_slice(b"P ");
    pline[2..8].copy_from_slice(&pnum);
    pline[8] = b'm';
    let _ = draw_str(0, 6, core::str::from_utf8(&pline[..9]).unwrap_or("P"));

    let _ = draw_str_centered(7, "GPIO9=NXT");
}

pub fn show_event_browser_empty() {
    if !is_ready() {
        return;
    }
    let _ = clear();
    let _ = draw_str_centered(2, "NO EVENTS");
    let _ = draw_str_centered(4, "ABHI KUCH NAHI");
}
