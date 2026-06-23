# Aether Enclave

[![License: AGPL-3.0-or-later](https://img.shields.io/badge/License-AGPL--3.0--or--later-blue.svg)](LICENSE)

A small bare-metal computer program (no Linux, no Wi-Fi) that runs a WebAssembly health-check module when sensors or a timer wake it up. It writes a proof hash, wipes its RAM, and goes back to sleep.

Built for a **defence expo booth demo** in India on the **ESP32-C6** (RISC-V). A QEMU/x86 build is included for bench testing without hardware.

**Read next:** [ARCHITECTURE.md](ARCHITECTURE.md) · [ROADMAP.md](ROADMAP.md)

---

## What you see at the booth

1. Board powers on → serial log + OLED shows `AETHER ENCLAVE` / sensor status.
2. WASM module runs → reads pressure (BMP390) and dose channel (ADS1115 + pot).
3. Screen and serial show **cycle number**, **alert flags**, and **64-bit proof hash**.
4. RAM is cleared → board sleeps until button press or timer (pot sets 5–60 s).
5. **Demo mode:** hold GPIO2 at power-on → cycles repeat every 2 s for live audience.

No cloud. No phone app. One USB-C cable for flash and logs.

---

## Why ESP32-C6 (not classic ESP32)

| | Classic ESP32 | ESP32-C6 (this project) |
|---|---------------|-------------------------|
| CPU | Xtensa — custom Rust toolchain | RISC-V — standard open toolchain |
| Debug | Extra UART chip or JTAG probe | USB-C built in (flash + serial) |
| Cost | Similar | DevKit ~₹700–1,200 |

This firmware does **not** use Wi-Fi, Bluetooth, or mesh radio.

---

## One mission cycle

```text
Sleep → Wake → Run WASM → Write proof → Wipe RAM → Sleep
```

| Step | What happens |
|------|----------------|
| Sleep | Deep sleep; timer length set by pot at boot |
| Wake | Button (vector 0x20), timer (0x21), or pressure drop on BMP390 |
| WASM | `aerospace_payload` checks limits via host sensor calls |
| Proof | 64-bit hash to MMIO + serial + OLED |
| Wipe | Sandbox and heap zeroed; registers cleared |

---

## Hardware (DefExpo breadboard)

| ESP32-C6 pin | Connect to |
|--------------|------------|
| 3.3V, GND | BMP390, ADS1115, OLED (shared) |
| **GPIO6** | I2C SDA (all I2C devices) |
| **GPIO7** | I2C SCL |
| GPIO2 | Button → 3.3V (wake; hold at boot = demo mode) |
| GPIO10 | Optional LED → 330 Ω → GND |
| ADS1115 AIN0 | Pot wiper (ends on 3.3V and GND) |

**I2C addresses:** BMP390 `0x76`, ADS1115 `0x48`, SSD1306 OLED `0x3C`.

GPIO8 on the DevKit is the onboard RGB LED — sensor I2C uses GPIO6/7 to avoid a pin clash.

**Shopping list (~₹2,500–4,000):** ESP32-C6-DevKitC-1, BMP390, ADS1115, SSD1306 128×64 I2C OLED, breadboard, wires, button, 10 kΩ pot, optional LED + 330 Ω, USB-C cable.

---

## Build and flash

### Setup (once)

```bash
cargo install espflash ldproxy
espup install
rustup target add riscv32imac-unknown-none-elf
```

Toolchain: [`enclave_kernel/rust-toolchain.esp32c6.toml`](enclave_kernel/rust-toolchain.esp32c6.toml) (`channel = "esp"`).

### Flash

From `enclave_kernel/`:

```bash
cargo +esp build --release
cargo +esp run --release
```

Use the DevKit **USB** port (Serial/JTAG). No external debugger.

### Demo controls

| Action | Result |
|--------|--------|
| Normal boot | Self-test cycle, then sleep |
| Hold GPIO2 at boot | Demo mode — cycle every 2 s |
| Press button after sleep | Wake vector 0x20 |
| Turn pot | Changes dose sensitivity and sleep timer |
| Blow on BMP390 | Pressure-drop wake |

### Sample serial output

```text
[AETHER] ESP32-C6 cold boot — USB Serial/JTAG ready
[AETHER] sensors — BMP390: OK (0x76)  ADS1115: OK (0x48)  OLED: OK
[AETHER] === MISSION READY ===
[AETHER] cycle #1 — guest=0 (OK) proof=0x........ vector=0x20 (PRESSURE) proof_changed=true
{"cycle":1,"guest":0,"flags":"OK","proof":"0x........",...}
```

---

## QEMU bench (no board)

```bash
rustup target add x86_64-unknown-none wasm32-unknown-unknown
rustup component add rust-src --toolchain nightly
cargo install bootimage

cargo +nightly run -p enclave_kernel --target x86_64-unknown-none \
  -Z build-std=core,alloc,compiler_builtins \
  -Z build-std-features=compiler-builtins-mem
```

Success exits with code **33** (see ARCHITECTURE.md).

---

## Workspace

| Crate | Role |
|-------|------|
| `enclave_kernel` | Bare-metal host (ESP32-C6 or QEMU) |
| `aerospace_payload` | WASM guest (`evaluate_limits`) |

Guest imports module `"aether"`: `read_atmospheric_pressure`, `read_radiation_dosimeter`, `commit_telemetry_vector`, `commit_uplink`.

Pressure limit: **0.15 atm**. Dose limit: **1000** (host scales ADC via pot).

---

## Optional: SD card (not in firmware yet)

SPI microSD module (~₹150) on GPIO3/4/5/15 for append-only proof logs. See [ROADMAP.md](ROADMAP.md) Phase 1.

---

## License

**AGPL-3.0-or-later** — review before government or vendor deployment.
