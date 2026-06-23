# Aether Enclave

[![License: AGPL-3.0-or-later](https://img.shields.io/badge/License-AGPL--3.0--or--later-blue.svg)](LICENSE)

**Indigenous RISC-V witness for sealed compartment integrity** — NBC shelters, stored kit, logistics crates, and UAV bays when the seal matters more than the camera.

Bare-metal firmware on a **reference ESP32-C6 board** (no Linux, no Wi‑Fi). Default mode: **event-only** — wake on pressure/dose change or button, run sandboxed WebAssembly policy, append **tamper-linked proof** to SD/serial, wipe RAM, sleep again.

Built by **[The SNMC](https://github.com/thesnmc/Aether-Enclave)** for **iDEX Open**. QEMU/x86 target included for bench testing.

**iDEX pack:** [IDEX_READY.md](IDEX_READY.md) · [IDEX_APPLICATION.md](IDEX_APPLICATION.md) · [IDEX_REVIEW.md](IDEX_REVIEW.md) · [LIMITS.md](LIMITS.md) · [DEMO_VIDEO.md](DEMO_VIDEO.md) · [EVALUATOR_TEST.md](EVALUATOR_TEST.md) · [ROADMAP.md](ROADMAP.md) · [ARCHITECTURE.md](ARCHITECTURE.md)

**ESP32 wiring:** [Wiring guide](#wiring-guide) (below) · [EVALUATOR_TEST.md](EVALUATOR_TEST.md) setup

---

## Company

**The SNMC** — one-person Indian defence electronics R&D (breadboard POC → PCB product via contract vendors).  
**Product:** **Aether Enclave** — offline environmental **witness**, not CCTV and not payload avionics.

---

## The problem (why CCTV is not enough)

| CCTV / cloud IoT | Sealed compartment needs |
|------------------|---------------------------|
| Watches **outside** the seal | **Pressure inside** NBC tent, crate, bay |
| Needs network + NVR | **Air-gapped** forward depots |
| Subjective video review | **Machine-verifiable** log (`verify_log.py`) |
| Always-on power | **µA sleep** until **event** |

**Gap:** Prove a **closed defence volume** stayed within environmental policy when nobody was watching — without cloud, without a Pi, without mission data in RAM overnight.

**Primary iDEX pitch:** **Sealed NBC / compartment integrity witness.**  
**Same hardware, secondary stories:** logistics crate custody, UAV bay when payload is out.

**Not claiming:** flight certification, satellite deployment, certified dosimeter on breadboard, CCTV replacement.

### Application one-liner

> *Indigenous low-power witness for sealed defence compartments: event-driven sandboxed checks, hash-linked offline proof, RAM wipe between cycles — ESP32-C6 is reference hardware only.*

Full application: **[IDEX_APPLICATION.md](IDEX_APPLICATION.md)**

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

**Demo mode:** hold GPIO2 at boot → cycles every 2 s (evaluator booth only).

---

## Win iDEX — 30-day checklist

1. [ ] Record **[5-min demo](DEMO_VIDEO.md)** — sealed box + tamper fail on `verify_log.py`  
2. [ ] Run **1000-cycle** SD soak; attach verify output to application  
3. [ ] Submit **[IDEX_APPLICATION.md](IDEX_APPLICATION.md)** (NBC witness lead)  
4. [ ] **2 PCB quotes** (same pinout as breadboard)  
5. [ ] Email **2–3** NBC / logistics / UAV workshops for evaluator interest  

**Do not lead with:** satellite, spy payload ML, “zero-allocation flight executive,” shelter-vs-CCTV fight.

---

## iDEX evaluator Q&A

Use this section to prepare for technical review, booth questions, and the application interview.

### Product and problem

**Q: What is Aether Enclave?**  
A: Indigenous **sealed-compartment witness** on bare-metal RISC-V. Event-driven WASM policy check, hash-linked offline proof, RAM wipe, deep sleep. Reference board: ESP32-C6; product: custom PCB.

**Q: What problem does it solve for the Services?**  
A: **Inside-the-seal** pressure integrity + audit log when CCTV/cloud cannot — NBC envelopes, stored kit, crates, idle UAV bays. Complements cameras; does not replace them.

**Q: Why not CCTV?**  
A: CCTV does not measure **pressure inside a sealed volume** or produce a **tamper-evident** machine log without network.

**Q: Why not a Raspberry Pi or Arduino?**  
A: Pi = Linux, high sleep current, large surface. Arduino = no WASM sandbox + proof chain + structured wipe. We target **~10–30 µA** sleep class.

**Q: Drones, satellites, or ground?**  
A: **This grant: ground sealed compartments.** UAV bay = secondary. Satellite / flight = **out of scope**; software may port to other RISC-V with a national lab partner — not this deliverable.

**Q: Who is The SNMC?**  
A: Solo founder (The SNMC). Firmware + bring-up in-house; PCB layout, assembly, and enclosure via vendors.

### Hardware and demo

**Q: What is on the breadboard?**  
A: WeAct ESP32-C6, BMP390L pressure, ADS1115 ADC, I2C OLED, optional SPI microSD, pot + button. Under ₹10k parts.

**Q: Why is there a potentiometer?**  
A: Demo stand-in for **dose front-end** (Phase 2 qualified sensor). Pot &gt;75% = RELAXED WASM; &gt;90% = radio dry-run; &lt;10% = optional interval wake.

**Q: What is EVENT_ONLY mode?**  
A: Default. No RTC timer wake. Logs only on **pressure/dose change** or button. BMP390 **INT → GPIO1** recommended.

**Q: What do evaluators see on the OLED?**  
A: Boot animation, cycle status on **events**, **ALERT** on policy fail, shutdown before sleep. **GPIO9 review button** scrolls the last four stored events (operator UI; SD + `verify_log.py` remain audit truth).

**Q: What is the GPIO9 button?**  
A: Optional **event browser** — press to scroll recent witness records on the OLED after a cycle. Does not change the proof chain; field operator convenience only.

**Q: Does it work without OLED or SD?**  
A: Yes. Serial always works. SD required for **tamper demo** in [DEMO_VIDEO.md](DEMO_VIDEO.md).

### Software and security

**Q: What happens to RAM after each cycle?**  
A: Fixed **128 KiB** arena zeroed; guest dropped; deep sleep. Bounded memory — **not** rad-hard silicon.

**Q: Wi‑Fi? Bluetooth? Cloud?**  
A: **Not used.** Air-gapped default. Optional one-way encrypted uplink = roadmap, off in prototype.

**Q: Why WebAssembly?**  
A: **Policy container** — strict/relaxed limits from SD or pot without reflashing Rust host.

**Q: Why Rust on RISC-V?**  
A: ESP32-C6 is RISC-V (open toolchain, no Xtensa lock-in). Rust + `no_std` fits bare-metal safety goals.

### Operations and test

**Q: What is the proof chain?**  
A: `proof = FNV-hash(prev ‖ guest ‖ sensors ‖ cycle ‖ mission …)`. Tamper one byte → `verify_log.py` fails. **Core iDEX demo.**

**Q: What wake sources exist?**  
A: **Default:** BMP390 INT (GPIO1) + button — **events only**. Optional: RTC timer if interval_wake enabled.

**Q: How do we test it?**  
A: [EVALUATOR_TEST.md](EVALUATOR_TEST.md) — **sealed box event** + **tamper test** (~30 min).

**Q: Expected sleep current?**  
A: ESP32-C6 deep sleep typically **~10–30 µA** (measure on your PCB; serial logs active ms per cycle for budget math).

### iDEX, IP, and process

**Q: Is the code on GitHub?**  
A: Yes — public repo, AGPL-3.0. Transparency for audit. Production deployment can use signed binaries + documented BOM; license can be discussed with adopting agency.

**Q: Did you use AI to build this?**  
A: Yes — AI-assisted editing (e.g. Cursor), under team review. We can explain wake flow, proof chain, WASM host, and wiring without tools in the room. iDEX funds **delivery capability**, not typing speed.

**Q: What do you deliver in 24 months on ₹1.5 Cr?**  
A: PCB + enclosure, 100 boxed units (grant deliverable), 25+ evaluator units, proof tools, docs — see [ROADMAP.md](ROADMAP.md). The SNMC **retains product IP** and sells post-grant. Not full platform flight qualification unless a Service customer scopes it.

**Q: Is ₹1.5 Cr too much for a breadboard?**  
A: The ask is **₹1.5 crore total over 24 months**, not per month. ₹1.5 **lakh/month** founder salary is normal for iDEX R&D. The rest buys PCB spins, 125 units, enclosure, travel, and vendor services. Ceiling ask matches ceiling deliverables.

**Q: Logging only — how does the operator know there was a breach?**  
A: **Detect → alert → log.** On policy fail the device signals **locally** (OLED ALERT, GPIO10, serial). SD log is for **audit after the fact**. Optional encrypted uplink when site policy allows. A logger that never alerts is incomplete; this product does both.

**Q: Is this a viable business after iDEX?**  
A: **Defence niche — yes, but slow** (trials, tenders). **Commercial sealed logistics — possible** with a clear wedge (air-gap + tamper proof + battery life). Not a mass-consumer play. Success = evaluator conversion + one repeat buyer, not overnight scale.

**Q: What milestones prove progress?**  
A: Git release tags, demo video, 1000-cycle SD log, evaluator test sheet results, PCB spin from feedback.

**Q: Biggest risks?**  
A: (1) Sensor qualification path for real dosimeter. (2) Service adoption needs named evaluator. (3) EMI/environment on PCB vs breadboard. Mitigation: Phase 2 front-end, PCB v1 pin-compatible with breadboard, written test scope.

**Q: Why should iDEX fund this vs commercial IoT?**  
A: Commercial loggers are cloud-tied and not built for **hash-linked offline custody proof** on **indigenous RISC-V** at **µA duty cycle**.

**Q: When do Services need this vs when they don't?**  
A: **Need:** sealed NBC/crate/bay, no trusted network, audit trail required. **Don't need:** 24/7 staffed room with CCTV only.

---

## When you need Aether / when you don't

| Need Aether | Don't need Aether |
|-------------|-------------------|
| Pressure **inside** sealed volume | Perimeter CCTV is enough |
| Offline tamper-evident log | Live cloud dashboard required |
| Months on battery, event-only | Always-on mains IoT |
| Air-gapped forward depot | Trusted NVR + staff |
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
| Optional: LED + 330 Ω on **GPIO10** | **Required for iDEX** — breach alert stays ON until GPIO2 ACK |
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

Evaluator checklist: [EVALUATOR_TEST.md](EVALUATOR_TEST.md). Demo video: [DEMO_VIDEO.md](DEMO_VIDEO.md).

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
| `enclave_kernel` | Rust witness host — ESP32-C6 reference or QEMU |
| `aerospace_payload` | WASM policy guest (`evaluate_limits`) |
| `tools/` | SD export, proof verify, profile writer, uplink decrypt |
| `IDEX_APPLICATION.md` | Submit-ready problem / milestones / budget narrative |
| `DEMO_VIDEO.md` | 5-minute sealed-box + tamper demo script |

Guest imports (`"aether"`): `read_atmospheric_pressure`, `read_radiation_dosimeter`, `read_pressure_limit`, `read_dose_limit`, `commit_telemetry_vector`.

---

## License

**AGPL-3.0-or-later** — review before government or vendor deployment.
