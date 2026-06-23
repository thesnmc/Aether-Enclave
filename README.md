# Aether Enclave

[![License: AGPL-3.0-or-later](https://img.shields.io/badge/License-AGPL--3.0--or--later-blue.svg)](LICENSE)

Bare-metal firmware for the **ESP32-C6** (no Linux, no Wi‑Fi). On each wake it runs a small WebAssembly check against pressure and a dose channel, writes a chained proof hash to serial / OLED / optional SD card, wipes RAM, and sleeps again.

Built as a **table demo** for defence expos in India. A QEMU/x86 target is included for bench testing without hardware.

**Also read:** [ARCHITECTURE.md](ARCHITECTURE.md) · [ROADMAP.md](ROADMAP.md) · [EVALUATOR_TEST.md](EVALUATOR_TEST.md)

---

## What it does

```text
Sleep → Wake → Run WASM → Log proof → Wipe RAM → Sleep
```

| Output | What you get |
|--------|----------------|
| USB serial | Human lines + one JSON line per cycle |
| OLED | Cycle number, flags, proof hash |
| microSD (optional) | Raw sectors — profile at 2047, proof log from 2048 |

**Demo mode:** hold the wake button at power-on → cycles every 2 s (no sleep between runs).

**What it is not:** flight hardware, a certified dosimeter, or a networked product. The pot stands in for a radiation front-end at the demo; see [ROADMAP.md](ROADMAP.md) for the path to real sensors and a PCB.

---

## Parts list (breadboard kit)

| Part | Notes |
|------|--------|
| WeAct **ESP32-C6-A-N4** (or Espressif DevKitC-1) | Flash via **native USB** port (not a UART-only cable) |
| DFRobot **BMP390L** (or BMP390) breakout | Pressure, I2C |
| **ADS1115** breakout | 16-bit ADC, I2C; AIN0 = pot / future sensor |
| **SSD1306** 128×64 OLED, **4-pin I2C** | VCC, GND, SDA, SCL |
| **OPEN-SMART** (or similar) microSD SPI module | 3.3 V only |
| microSD card | Dedicated card for logging (not your main storage) |
| 10 kΩ potentiometer | Demo dose + wake timer tuning |
| Tactile button | Wake + demo-mode hold |
| Breadboard, jumper wires | |
| Optional: LED + 330 Ω | Status on GPIO10 |
| USB **A→C data** cable | Must carry data, not charge-only |

Typical parts cost **under ₹4,000**; full kit with spares stays **under ₹10,000**.

---

## Wiring guide

Power everything from the board **3.3 V** and **GND** rails. Do not use 5 V on these breakouts.

### Pin map (firmware)

| ESP32-C6 GPIO | Connect to |
|---------------|------------|
| **3.3V** | VCC on BMP390, ADS1115, OLED, SD module |
| **GND** | GND on all modules (common ground) |
| **GPIO6** | I2C **SDA** (shared bus) |
| **GPIO7** | I2C **SCL** (shared bus) |
| **GPIO2** | One side of **button** → other side of button to **3.3V** |
| **GPIO3** | SD **MOSI** |
| **GPIO4** | SD **MISO** |
| **GPIO5** | SD **SCK** |
| **GPIO15** | SD **CS** |
| **GPIO10** | Optional LED → 330 Ω → GND |
| ADS1115 **AIN0** | Pot **wiper** |
| Pot ends | **3.3V** and **GND** |

**I2C addresses (fixed in firmware):** BMP390 `0x76`, ADS1115 `0x48`, OLED `0x3C`.

GPIO6/7 are used so DevKitC onboard LED on GPIO8 does not share the sensor bus. WeAct boards have no conflict on those pins.

### Step-by-step

1. **Power rails** — Board 3.3V and GND to breadboard + and − rails.
2. **I2C bus** — Daisy-chain SDA and SCL on BMP390, ADS1115, and OLED. Add short wires; keep leads under ~20 cm if reads are noisy.
3. **Pot** — Outer pins to 3.3V and GND; wiper to ADS1115 AIN0 only.
4. **Button** — GPIO2 to one leg; other leg to 3.3V. Firmware uses internal pull-**down** on GPIO2. **Do not** wire the button to GND.
5. **SD module** — 3.3V, GND, MOSI/MISO/SCK/CS as in the table. Insert card after power is stable if your module lacks a regulator.
6. **USB** — Plug into the board’s USB port labelled for **USB** / **native** (WeAct: use the USB port on the module, not a separate UART adapter).

```text
                    ┌─────────────────┐
    3.3V ───────────┤ 3.3V            │
    GND  ───────────┤ GND             │
    GPIO6 ──────────┤ SDA ────┬───────┼── BMP390, ADS1115, OLED (I2C)
    GPIO7 ──────────┤ SCL ────┘       │
    GPIO2 ──[btn]── 3.3V              │
    GPIO3/4/5/15 ───┤ SPI ────────────┼── microSD module
                    │  ESP32-C6       │
                    └─────────────────┘
```

### First power-on

Flash firmware (below), open serial monitor, expect:

```text
[AETHER] sensors — BMP390: OK  ADS1115: OK  OLED: OK  SD: OK
```

`MISSING` on a line means that device did not answer on I2C or SPI — check power, SDA/SCL swap, or loose wire. The demo still runs without OLED or SD.

---

## Build and flash

### One-time setup

```bash
cargo install espflash ldproxy
espup install
rustup target add riscv32imac-unknown-none-elf
```

Toolchain file: [`enclave_kernel/rust-toolchain.esp32c6.toml`](enclave_kernel/rust-toolchain.esp32c6.toml) (`channel = "esp"`).

### Flash

```bash
cd enclave_kernel
cargo +esp build --release
cargo +esp run --release
```

### Controls

| Action | Result |
|--------|--------|
| Normal boot | One WASM cycle, then deep sleep |
| Hold GPIO2 at boot | Demo loop, ~2 s between cycles |
| Press button after sleep | Wake (vector 0x20) |
| Turn pot at boot | Wake timer 5–60 s; dose scaling |
| Pot > ~75% at boot | **RELAXED** WASM slot (looser limits) |
| Pot low at boot | **STRICT** slot |
| Blow on BMP390 | Pressure-drop or rapid-leak wake |

Default limits: pressure **0.15 atm**, dose **1000** (ADC scaled by pot). SD profile on sector 2047 overrides limits — see tools below.

---

## microSD layout (optional)

| Sector | Content |
|--------|---------|
| 2047 | Mission profile (`AEPR`) — limits, wake bounds, strict/relaxed slot |
| 2048 | Log metadata (`AETH`) |
| 2049+ | One 512-byte record per cycle (`AEC1`) |

No FAT filesystem. On Linux use `/dev/sdb` (unmount first); on Windows `\\.\PhysicalDriveN` as admin.

```bash
python tools/sd_export.py /dev/sdb
python tools/verify_log.py serial_or_export.txt
python tools/write_mission_profile.py /dev/sdb --mission-id 1 --payload relaxed
```

Evaluator checklist: [EVALUATOR_TEST.md](EVALUATOR_TEST.md).

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

Success exits QEMU with code **33**.

---

## Repo layout

| Crate / path | Role |
|--------------|------|
| `enclave_kernel` | Rust host — ESP32-C6 or QEMU |
| `aerospace_payload` | WASM guest (`evaluate_limits`) |
| `tools/` | SD export, proof verify, profile writer |

Guest imports (`"aether"`): `read_atmospheric_pressure`, `read_radiation_dosimeter`, `read_pressure_limit`, `read_dose_limit`, `commit_telemetry_vector`.

---

## License

**AGPL-3.0-or-later** — review before government or vendor deployment.
