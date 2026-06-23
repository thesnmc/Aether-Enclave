# Aether Enclave

[![License: AGPL-3.0-or-later](https://img.shields.io/badge/License-AGPL--3.0--or--later-blue.svg)](LICENSE)

**Offline custody receipts for sealed logistics** — when a sealed shipment is violated, prove it at any checkpoint without vendor cloud.

Bare-metal **custody witness module** on a **reference ESP32-C6 board** (no Linux, no Wi‑Fi). Event-only: wake on seal violation (lid open / pressure change), run policy, **latched alert**, issue **hash-linked receipt**, verify on laptop, sleep.

Built by **[The SNMC](https://github.com/thesnmc/Aether-Enclave)**. QEMU/x86 target included for bench testing.

**Docs:** [COMMERCIAL_PITCH.md](COMMERCIAL_PITCH.md) · [PILOT_TEST.md](PILOT_TEST.md) · [DEMO_VIDEO.md](DEMO_VIDEO.md) · [LIMITS.md](LIMITS.md) · [ROADMAP.md](ROADMAP.md) · [ARCHITECTURE.md](ARCHITECTURE.md)

**ESP32 wiring:** [Wiring guide](#wiring-guide) (below) · [PILOT_TEST.md](PILOT_TEST.md) setup

---

## Company

**The SNMC** — one-person electronics R&D (breadboard POC → PCB product via contract vendors).  
**Product:** **Aether Enclave** — **custody witness module** for sealed logistics (anti-pilferage receipts), not CCTV and not a port e-seal clone.

Full pitch: **[COMMERCIAL_PITCH.md](COMMERCIAL_PITCH.md)**

---

## The problem

| Paper seal / cloud tracker | Sealed logistics needs |
|----------------------------|------------------------|
| Broken seal, no machine record | **Custody receipt** at checkpoint |
| Vendor cloud / GPS dashboard | **Air-gapped verify** on laptop |
| Disputed handoff | **Machine-verifiable** export (`verify_log.py`) |
| Always-on tracker cost | **µA sleep** until **violation** |

**Gap:** Prove a **sealed consignment was violated** in transit — pilferage, custody disputes — without cloud, without trusting a vendor portal.

**Not claiming:** port e-seal replacement, NBC certification, operational dosimeter on breadboard.

### One-liner

> *Custody witness for sealed logistics: seal violated → local alert → hash-linked receipt → verify at checkpoint — ESP32-C6 is reference hardware only.*

---

## What it does

```text
Sleep (µA) → Event (pressure / dose / button) → WASM check → Proof log → Wipe RAM → Sleep
```

| Output | Role |
|--------|------|
| USB serial | Human lines + JSON per **event** (not spam in event-only mode) |
| OLED | Boot/cycle/shutdown for demo table |
| microSD | Tamper-linked audit trail (sectors 2047+) |

**Default:** `EVENT_ONLY` — no periodic timer wake, no log until **real sensor change**.  
**Optional:** interval wake via SD profile or pot &lt;10% at boot.  
**Radio:** encrypted uplink scaffold, **OFF by default**.

**Demo mode:** hold GPIO2 at boot → cycles every 2 s (trade-show booth only).

---

## Customer Q&A

### Product

**Q: What is Aether Enclave?**  
A: **Sealed-compartment witness** on bare-metal RISC-V. Event-driven WASM policy check, hash-linked offline proof, RAM wipe, deep sleep. Reference board: ESP32-C6; product: boxed **AE-CM1** module.

**Q: Who buys this?**  
A: Logistics contractors, bonded warehouses, pharma/hazmat shippers, high-value crate integrators — anywhere **air-gap + tamper proof + battery life** beats a cloud dashboard.

**Q: Why not CCTV or a GPS tracker?**  
A: CCTV does not measure **inside the sealed volume** or produce a **tamper-evident** machine log without network. GPS shows location, not **seal integrity**.

**Q: Why not Raspberry Pi or Arduino?**  
A: Pi = Linux, high sleep current. Arduino = no WASM sandbox + proof chain + structured wipe. We target **~10–30 µA** sleep class.

### Hardware and demo

**Q: What is on the breadboard?**  
A: WeAct ESP32-C6, BMP390L pressure, ADS1115 ADC, I2C OLED, optional SPI microSD, pot + button. Under ₹10k parts.

**Q: Why the potentiometer?**  
A: Demo stand-in for **dose front-end** (Phase 2 qualified sensor). Pot &gt;75% = RELAXED WASM; &gt;90% = radio dry-run; &lt;10% = optional interval wake.

**Q: What is EVENT_ONLY mode?**  
A: Default. No RTC timer wake. Logs only on **pressure/dose change** or button. BMP390 **INT → GPIO1** recommended.

**Q: What does the operator see?**  
A: Boot animation, cycle status on **events**, **ALERT** on policy fail, shutdown before sleep. **GPIO9** scrolls the last four stored events (OLED convenience; SD + `verify_log.py` remain audit truth).

**Q: Does it work without OLED or SD?**  
A: Yes. Serial always works. SD required for **tamper demo** in [DEMO_VIDEO.md](DEMO_VIDEO.md).

### Security and operations

**Q: What is the proof chain?**  
A: `proof = FNV-hash(prev ‖ guest ‖ sensors ‖ cycle ‖ mission …)`. Tamper one byte → `verify_log.py` fails. **Core pilot demo.**

**Q: Wi‑Fi? Cloud?**  
A: **Not used.** Air-gapped default. Optional one-way encrypted uplink = roadmap, off in prototype.

**Q: How do we test it?**  
A: [PILOT_TEST.md](PILOT_TEST.md) — **sealed box event** + **tamper test** (~30 min).

**Q: Logging only — how does the operator know there was a breach?**  
A: **Detect → alert → log.** On policy fail: OLED ALERT, GPIO10 latched, serial `BREACH`. SD log is for **audit after the fact**.

---

## When you need Aether / when you don't

| Need Aether | Don't need Aether |
|-------------|-------------------|
| Pressure **inside** sealed volume | Perimeter CCTV is enough |
| Offline tamper-evident log | Live cloud dashboard required |
| Months on battery, event-only | Always-on mains IoT |
| Air-gapped site | Trusted NVR + staff |
| Policy swap (WASM/SD) | Fixed firmware OK |

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
| Tactile button (GPIO2) | Wake + demo-mode hold |
| Review button (GPIO9) | Scroll last events on OLED (optional) |
| Breadboard, jumper wires | |
| Optional: LED + 330 Ω on **GPIO10** | Breach alert stays ON until GPIO2 ACK |
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
| **GPIO2** | One side of **wake button** → other side to **3.3V** |
| **GPIO9** | One side of **review button** → other side to **GND** (internal pull-up) |
| **GPIO1** | BMP390 **INT** (top row on 7-pin header) |
| **GPIO3** | SD **MOSI** |
| **GPIO4** | SD **MISO** |
| **GPIO5** | SD **SCK** |
| **GPIO15** | SD **CS** |
| **GPIO10** | Alert LED → **330 Ω** → **GND** (solid ON when breach latched) |
| ADS1115 **AIN0** | Pot **wiper** |
| Pot ends | **3.3V** and **GND** |

**I2C addresses (fixed in firmware):** BMP390 `0x76`, ADS1115 `0x48`, OLED `0x3C`.

GPIO6/7 are the shared I2C bus. **GPIO1** takes BMP390 **INT** for hardware wake while the CPU sleeps.

### BMP390 INT (optional but recommended)

Use the **bottom 4-pin I2C row** (VCC, GND, SCL, SDA) plus one wire from the **top-row INT** pin to **GPIO1**. INT is open-drain active-low; firmware wakes on INT low during deep sleep.

### Step-by-step

1. **Power rails** — Board 3.3V and GND to breadboard + and − rails.
2. **I2C bus** — Daisy-chain SDA and SCL on BMP390, ADS1115, and OLED. Add short wires; keep leads under ~20 cm if reads are noisy.
3. **Pot** — Outer pins to 3.3V and GND; wiper to ADS1115 AIN0 only.
4. **Wake button (GPIO2)** — GPIO2 to one leg; other leg to 3.3V. Firmware uses internal pull-**down** on GPIO2. **Do not** wire GPIO2 to GND.
5. **Review button (GPIO9)** — GPIO9 to one leg; other leg to **GND**. Internal pull-**up**; press = scroll OLED event log.
6. **BMP390 INT** — Top-header **INT** → **GPIO1** (same GND as ESP32).
7. **SD module** — 3.3V, GND, MOSI/MISO/SCK/CS as in the table. Insert card after power is stable if your module lacks a regulator.
8. **Alert LED (GPIO10)** — GPIO10 → 330 Ω → LED → GND. Lights on policy fail; stays on through sleep until GPIO2 ACK.
9. **USB** — Plug into the board’s USB port labelled for **USB** / **native** (WeAct: use the USB port on the module, not a separate UART adapter).

```text
                    ┌─────────────────┐
    3.3V ───────────┤ 3.3V            │
    GND  ───────────┤ GND             │
    GPIO6 ──────────┤ SDA ────┬───────┼── BMP390, ADS1115, OLED (I2C)
    GPIO7 ──────────┤ SCL ────┘       │
    GPIO2 ──[wake]── 3.3V            │
    GPIO9 ──[review]── GND          │
    GPIO1 ──────────┤ INT ────────────┼── BMP390 (top header)
    GPIO3/4/5/15 ───┤ SPI ────────────┼── microSD module
    GPIO10 ──[330R]── LED ── GND     │
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
| Normal boot | **EVENT_ONLY** — baseline, sleep until change |
| Blow / open sealed box | Pressure event → WASM → SD log |
| Press GPIO2 wake button | Manual witness cycle |
| Press GPIO9 review button | Scroll last events on OLED |
| Hold GPIO2 at boot | Demo loop (~2 s) — booth only |
| Pot &lt;10% at boot | Optional **interval** wake enabled |
| Pot &gt;75% | RELAXED WASM limits |
| Pot &gt;90% | Radio dry-run hex on serial (RF off) |
| SD `--interval-wake` | Scheduled checks + events |

Pilot checklist: [PILOT_TEST.md](PILOT_TEST.md). Demo video: [DEMO_VIDEO.md](DEMO_VIDEO.md).

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
python tools/write_mission_profile.py /dev/sdb --interval-wake   # optional scheduled checks
python tools/write_mission_profile.py /dev/sdb --radio           # uplink dry-run only
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

Success exits QEMU with code **33**.

---

## Repo layout

| Crate / path | Role |
|--------------|------|
| `enclave_kernel` | Rust witness host — ESP32-C6 reference or QEMU |
| `aerospace_payload` | WASM policy guest (`evaluate_limits`) |
| `tools/` | SD export, proof verify, profile writer, uplink decrypt |
| `COMMERCIAL_PITCH.md` | Product pitch, SKUs, business model |
| `PILOT_TEST.md` | 30-minute customer pilot procedure |
| `DEMO_VIDEO.md` | 5-minute sealed-box + tamper demo script |

Guest imports (`"aether"`): `read_atmospheric_pressure`, `read_radiation_dosimeter`, `read_pressure_limit`, `read_dose_limit`, `commit_telemetry_vector`.

---

## License

**AGPL-3.0-or-later** — review before commercial deployment or OEM integration.
