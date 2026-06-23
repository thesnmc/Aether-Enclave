# Aether Enclave

[![License: AGPL-3.0-or-later](https://img.shields.io/badge/License-AGPL--3.0--or--later-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-stable%20%2B%20esp-orange.svg)](https://www.rust-lang.org/)
[![no_std](https://img.shields.io/badge/no__std-bare--metal-critical.svg)](enclave_kernel/)

A bare-metal `#![no_std]` kernel that runs a WebAssembly diagnostic module on each wake cycle. It reads sensors through a fixed host API, writes a 64-bit proof hash to MMIO registers, wipes all runtime memory, and goes back to sleep. There is no operating system, no file system, and no network stack.

**Hardware target:** [ESP32-C6](https://www.espressif.com/en/products/socs/esp32-c6) (RISC-V). **Simulator target:** x86_64 in QEMU for bench testing without a board.

---

## Why ESP32-C6 and not a classic ESP32?

| Topic | Classic ESP32 (WROOM-32) | ESP32-C6 (this project) |
|-------|--------------------------|-------------------------|
| CPU | Xtensa LX6 — needs a custom Rust/LLVM toolchain | RISC-V — standard `riscv32imac-unknown-none-elf` target |
| Flash / debug | No on-chip USB; boards use a separate USB-UART chip or an external JTAG probe | Built-in **USB Serial/JTAG** — one USB-C cable for flash, serial logs, and debug |
| Bare-metal Rust | Possible but harder to maintain without FreeRTOS | Supported by [esp-hal](https://github.com/esp-rs/esp-hal) and [esp-println](https://github.com/esp-rs/esp-hal/tree/main/esp-println) |

This project does **not** use Wi-Fi, Bluetooth, or 802.15.4 mesh. Your friend's point about 802.15.4 is true for the chip family, but it does not apply to this codebase — there is no mesh or radio code here.

---

## How one cycle works

```text
Sleep → Wake (IRQ 0x20 or 0x21) → Run WASM → Write proof → Wipe memory → Sleep
```

1. **Sleep** — x86: `hlt` with interrupts on. ESP32-C6: RTC deep sleep (pot sets 5–60 s timer, or GPIO2 high).
2. **Wake** — Vector `0x20` = pressure threshold. Vector `0x21` = timer / kinetic pulse.
3. **WASM** — `aerospace_payload` runs in [wasmi](https://github.com/wasmi-labs/wasmi); reads pressure and dose via host imports.
4. **Proof** — Host fuses guest status + sensor samples into a 64-bit hash in `REG_UPLINK_COMMIT_LO/HI`.
5. **Wipe** — Sandbox and heap arena zeroed; ESP32-C6 enters deep sleep again (x86: QEMU exit code 33).

See [ARCHITECTURE.md](ARCHITECTURE.md) for register map, memory layout, and build flow.

---

## Workspace

| Crate | Target | Role |
|-------|--------|------|
| [`enclave_kernel`](enclave_kernel/) | `riscv32imac-unknown-none-elf` (default) or `x86_64-unknown-none` | Bare-metal host: boot, MMIO, wasmi, sleep |
| [`aerospace_payload`](aerospace_payload/) | `wasm32-unknown-unknown` | WASM guest: limit checks + telemetry |

`enclave_kernel/build.rs` compiles the guest at kernel build time and embeds `WASM_BYTES` in `src/wasm_payload.rs` (auto-generated).

---

## Build for ESP32-C6 (hardware prototype)

### One-time setup

```bash
cargo install espflash ldproxy
espup install
rustup target add riscv32imac-unknown-none-elf
```

Use the Espressif Rust toolchain (`channel = "esp"`) — see [`enclave_kernel/rust-toolchain.esp32c6.toml`](enclave_kernel/rust-toolchain.esp32c6.toml).

### Build and flash

From `enclave_kernel/`:

```bash
cargo +esp build --release
cargo +esp run --release
```

Plug the DevKit's **USB** port (the one wired to the chip's USB Serial/JTAG, not a separate UART bridge if your board has two). `espflash` flashes the firmware and opens a serial monitor. Panic messages from `esp-println` appear on the same port — no external debugger required.

### Demo features (firmware)

| Feature | How |
|---------|-----|
| **Demo mode** | Hold GPIO2 high while powering on → WASM cycles every 2 s (no sleep) |
| **Cycle counter** | `cycle #N` in serial + JSON line; survives deep sleep via RTC RAM |
| **Pot → dose sensitivity** | Turn pot before a cycle; higher ADC = easier to hit `DOSE_HIGH` |
| **Pot → wake timer** | Turn pot at boot; sets deep-sleep timer between 5–60 s |
| **Pressure-drop wake** | Blow on BMP390 or squeeze a bag; >0.015 atm drop forces vector `0x20` |
| **Status LED** | GPIO10 → 330 Ω → LED → GND lights during WASM run |
| **JSON telemetry** | One `{"cycle":...}` line per cycle for laptop capture |

### Expected serial output (after flash)

```text
[AETHER] ESP32-C6 cold boot — USB Serial/JTAG ready
[AETHER] wake cause — POWER_ON_RESET
[AETHER] sensors — BMP390: OK (0x76)  ADS1115: OK (0x48)
[AETHER] === MISSION READY ===
[AETHER] snapshot — cycle=0 pressure=0.987 atm alt=120 m temp=24.1 C dose=412 (raw 820) wake_timer=10s
[AETHER] cold boot — WASM self-test (vector 0x20)
[AETHER] cycle #1 — guest=0 (OK) proof=0x........ vector=0x20 (PRESSURE) proof_changed=true
{"cycle":1,"guest":0,"flags":"OK","proof":"0x........",...}
```

After sleep, press GPIO2 or wait for the pot-configured timer.

### Event demo (2 minutes)

1. Flash: `cargo +esp run --release`
2. **Cold boot** — self-test cycle + proof line
3. **Hold button at power-on** — demo mode (continuous cycles for the audience)
4. **Pot** — turn up to trigger `DOSE_HIGH`; turn to change wake timer length
5. **BMP390** — blow on sensor for pressure-drop wake
6. **GPIO10 LED** — blinks during each WASM run

If a sensor shows `MISSING`, check wiring (SDA=GPIO6, SCL=GPIO7).

---

## Optional add-ons (OLED / SD card)

Compatible with this stack (`esp-hal` + `#![no_std]`), but **not implemented yet**. Both share the same I2C bus as your sensors.

### OLED display (recommended: SSD1306 128×64 I2C)

| Buy | Notes |
|-----|-------|
| **SSD1306 128×64 I2C module** (~$3–5) | Same wiring as BMP390: SDA/SCL/3.3V/GND on the shared bus |
| Address | Usually `0x3C` (sometimes `0x3D`) — no conflict with BMP390 `0x76` or ADS1115 `0x48` |

**Rust path:** add crate `ssd1306` + `embedded-graphics` (both work on `no_std`). Show cycle #, guest flags, and proof hash on screen after each WASM run.

**Do not buy:** SPI-only OLED modules unless you want extra wires on GPIO12/13 — I2C is simpler with your current breadboard.

### microSD card (SPI mode)

| Buy | Notes |
|-----|-------|
| **microSD SPI module** (~$1–3) | 3.3 V logic level; **not** 5 V SD shields |
| microSD card | Any small card; formatted FAT32 from a PC first |

**Suggested wiring (free GPIOs on DevKitC-1):**

| SD pin | ESP32-C6 GPIO |
|--------|---------------|
| CS | GPIO15 |
| MOSI | GPIO5 |
| MISO | GPIO4 |
| SCK | GPIO3 |

**Rust path:** `embedded-sdmmc` crate + `esp-hal` SPI driver. Use case: append one proof line per cycle to `PROOF.LOG` on the card (black-box audit trail). Adds code size and SPI bus setup — best as phase 2 after the event if serial logging is enough.

### What not to get

- **SH1106 or parallel RGB LCD** — different drivers, more pins
- **SD card in SDIO mode** — harder on bare metal; SPI module is enough
- **5 V I2C OLED** — get 3.3 V logic versions

---

## Build for QEMU (x86 bench, no board)

### Prerequisites

- Rust nightly + `rust-src`
- [`bootimage`](https://github.com/rust-osdev/bootimage)
- QEMU `qemu-system-x86_64` with `isa-debug-exit`

```bash
rustup target add x86_64-unknown-none wasm32-unknown-unknown
rustup component add rust-src --toolchain nightly
cargo install bootimage
```

### Build and run

```bash
cargo +nightly build -p enclave_kernel --target x86_64-unknown-none \
  -Z build-std=core,alloc,compiler_builtins \
  -Z build-std-features=compiler-builtins-mem

cargo +nightly run -p enclave_kernel --target x86_64-unknown-none \
  -Z build-std=core,alloc,compiler_builtins \
  -Z build-std-features=compiler-builtins-mem
```

QEMU injects bench sensor values and fires software IRQ `0x20`. Success exits with process code **33** (see ARCHITECTURE.md).

---

## Hardware wiring (ESP32-C6-DevKitC-1)

| ESP32-C6 pin | Connect to |
|--------------|------------|
| 3.3V, GND | BMP390 + ADS1115 power |
| **GPIO6** | I2C SDA (both sensors) |
| **GPIO7** | I2C SCL (both sensors) |
| GPIO2 | Button to 3.3V (wake + demo mode if held at boot) |
| GPIO10 | Optional status LED → 330 Ω → LED → GND |
| ADS1115 AIN0 | Potentiometer wiper (ends on 3.3V and GND) |

GPIO8 on the DevKit is the onboard RGB LED — I2C uses GPIO6/7 to avoid a pin conflict.

I2C addresses: BMP390 `0x76`, ADS1115 `0x48`.

**Parts list:** DevKit, BMP390, ADS1115, breadboard, wires, button, 10 kΩ pot, optional LED + 330 Ω resistor, USB-C cable.

---

## WASM host imports (guest side)

```rust
#[link(wasm_import_module = "aether")]
extern "C" {
    fn read_atmospheric_pressure() -> f32;
    fn read_radiation_dosimeter() -> i32;
    fn commit_telemetry_vector(ptr: i32, len: i32);
    fn commit_uplink(proof_lo: i32, proof_hi: i32);
}
```

Guest exports: `evaluate_limits`, `diagnostic`, `payload_version`, `memory`.

Pressure limit: **0.15 atm**. Dose limit: **1000** counts. Bench injection on x86 uses 0.12 atm and 1250 dose → guest status **3**.

---

## License

**AGPL-3.0-or-later** — see crate manifests. Review the license before deploying as a network service.
