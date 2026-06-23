//! GPIO9 review button — scroll last witness events on OLED (operator UI).

use esp_hal::delay::Delay;
use esp_hal::gpio::{Input, InputConfig, Pull};

use crate::serial_println;

use super::{event_log, esp32c6, oled};
use esp32c6::PlatformState;

const IDLE_EXIT_MS: u32 = 45_000;
const DEBOUNCE_MS: u32 = 40;

/// Run browser when GPIO9 is held low (press review button).
pub(crate) fn run_if_requested(state: &mut PlatformState) {
    if !oled::panel_ready() {
        return;
    }
    let btn = Input::new(
        state.review_gpio.reborrow(),
        InputConfig::default().with_pull(Pull::Up),
    );
    if !btn.is_low() {
        return;
    }
    serial_println!("[AETHER] OLED — event browser (GPIO9, press to scroll)");
    run_loop(&btn);
}

fn run_loop(btn: &Input<'_>) {
    let n = event_log::count();
    if n == 0 {
        oled::show_event_browser_empty();
        Delay::new().delay_millis(1500);
        return;
    }

    let mut index = 0usize;
    let mut idle = 0u32;
    loop {
        esp32c6::feed_watchdog();
        if let Some(ev) = event_log::get(index) {
            oled::show_event_record(index, n, &ev);
        }

        if btn.is_low() {
            if debounce_pressed(btn) {
                index = (index + 1) % n;
                idle = 0;
            }
        } else {
            idle = idle.saturating_add(50);
            if idle >= IDLE_EXIT_MS {
                serial_println!("[AETHER] OLED — browser idle exit");
                break;
            }
        }
        Delay::new().delay_millis(50);
    }
}

fn debounce_pressed(btn: &Input<'_>) -> bool {
    for _ in 0..(DEBOUNCE_MS / 10) {
        if btn.is_high() {
            return false;
        }
        Delay::new().delay_millis(10);
    }
    while btn.is_low() {
        esp32c6::feed_watchdog();
        Delay::new().delay_millis(10);
    }
    true
}
