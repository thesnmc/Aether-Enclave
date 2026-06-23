# Aether Enclave — Architecture

Plain-language technical reference for the repo. Target hardware: **ESP32-C6-DevKitC-1** at a defence expo demo. Bench target: **QEMU x86_64**.

---

## Contents

1. [Purpose](#1-purpose)
2. [Block diagram](#2-block-diagram)
3. [Boot flow (ESP32-C6)](#3-boot-flow-esp32-c6)
4. [Wake sources](#4-wake-sources)
5. [Memory](#5-memory)
6. [WASM bridge](#6-wasm-bridge)
7. [Run pipeline](#7-run-pipeline)
8. [MMIO registers](#8-mmio-registers)
9. [Proof hash](#9-proof-hash)
10. [After each cycle](#10-after-each-cycle)
11. [Platform files](#11-platform-files)
12. [Build flow](#12-build-flow)

---

## 1. Purpose

Run a fixed WebAssembly diagnostic on every wake. Keep no secrets in RAM between runs. Output a verifiable hash for serial, OLED, and optional SD log.

Design constraints:

- No OS, no network stack, no full file system (SD uses fixed sectors, not FAT).
- Guest logic is replaceable by rebuilding `aerospace_payload.wasm`.
- Host enforces memory limits and sensor access.

---

## 2. Block diagram

```text
         ┌──────────────────────────────────────────────┐
 Wake ──▶│  enclave_kernel                              │
         │   wake decode → wasmi host → MMIO / serial   │
         │                      │                        │
         │                      ▼                        │
         │              aerospace_payload (WASM)         │
         └──────────┬───────────────────────┬───────────┘
                    │ I2C                   │ proof
                    ▼                       ▼
              BMP390, ADS1115          OLED + USB log + SD (optional)
              (GPIO6/7)                (esp-println)   (GPIO3/4/5/15)
```

---

## 3. Boot flow (ESP32-C6)

| Step | Code | Action |
|------|------|--------|
| 1 | `esp_hal::init` | CPU, USB Serial/JTAG |
| 2 | `esp32c6::init` | I2C, WDT, GPIO2 wake, GPIO10 LED, probe sensors + OLED + SD |
| 3 | `apply_pot_mission_profile` | Pot → dose scale + sleep timer (RTC RAM) |
| 4 | `detect_demo_mode_hold` | GPIO2 held → continuous demo loop |
| 5 | `resolve_trigger` | Wake cause or pressure drop or cold-boot self-test |
| 6 | `run_mission_cycle` | WASM run → `finish_cycle` → deep sleep (or demo loop) |

Entry: `#[esp_hal::main] fn esp_main()` in `main.rs`.

---

## 4. Wake sources

Same vector IDs on QEMU and ESP32-C6:

| Vector | Name | ESP32-C6 cause |
|--------|------|----------------|
| `0x20` | Pressure path | GPIO2 button, pressure drop vs last sleep |
| `0x21` | Timer path | RTC timer (5–60 s from pot) |

Deep sleep resets the CPU; there is no interrupt table on C6. Wake cause is read once at boot via `SleepSource`.

**RTC RAM** (`rtc_state.rs`) keeps across sleep: cycle count, last proof, last pressure, wake timer seconds.

---

## 5. Memory

Static bump arena only (ESP32-C6 ~512 KiB SRAM total):

| Buffer | Size | Use |
|--------|------|-----|
| `ARENA` | 128 KiB | wasmi compile + heap |
| Guest cap | 64 KiB | Max WASM linear memory (enforced at load) |

Each cycle: `reset_arena()` at start, `wipe_host_memory()` after run.

---

## 6. WASM bridge

Import module `"aether"`:

| Import | Host on C6 |
|--------|------------|
| `read_atmospheric_pressure` | BMP390 (3-sample average) |
| `read_radiation_dosimeter` | ADS1115 AIN0, scaled by pot |
| `commit_telemetry_vector` | Copy from guest RAM (bounds checked) |

Guest export: `evaluate_limits() -> i32` flags:

| Value | Meaning |
|-------|---------|
| `0` | OK |
| `1` | Pressure below 0.15 atm |
| `2` | Dose above 1000 |
| `3` | Both |

---

## 7. Run pipeline

```text
run_mission_cycle(trigger)
  → reset_arena()
  → wasmi: load WASM_BYTES, link imports, call evaluate_limits
  → commit_outcome() → proof hash
  → finish_cycle() → log, OLED, wipe RAM
  → enter_absolute_halt() → deep sleep
```

Errors log `ERR: Linker / Instantiation / Trap` on serial then fault shutdown.

---

## 8. MMIO registers

Logical addresses (same on all targets; C6 backs them with atomics + live I2C):

| Register | Address | Content |
|----------|---------|---------|
| Atmospheric pressure | `0xFEF0_0008` | `f32` atm |
| Radiation dose | `0xFEF0_000C` | u32 counts |
| Proof low / high | `0xFEF0_0010` / `14` | u32 each |
| PMU command | `0xFEF0_0020` | sleep latch |

---

## 9. Proof hash

After guest returns:

```text
proof_lo = guest_result XOR last_dose XOR last_sensor
proof_hi = rotate_left(last_dose, 9) XOR pressure_bits XOR 0xA17E_0001
proof    = (proof_hi << 32) | proof_lo
```

Printed on serial, shown on OLED, stored in RTC RAM for `proof_changed` compare. If SD init succeeded, one sector append per cycle (`sd_log.rs`).

---

## 10. After each cycle

`finish_cycle()`:

1. Log human line + JSON line  
2. Update OLED (`show_cycle`)  
3. Append SD sector if card present  
4. Turn off GPIO10 LED  
5. Zero sandbox and arena  
6. Clear selected CPU registers  
7. `request_deep_sleep()` — timer + GPIO2 wake  

Panic path uses the same wipe then sleep.

---

## 11. Platform files

| File | Role |
|------|------|
| `platform/esp32c6.rs` | I2C sensors, sleep, LED, pot profile |
| `platform/oled.rs` | SSD1306 128×64 status screen |
| `platform/sd_log.rs` | microSD SPI append-only proof sectors |
| `platform/rtc_state.rs` | Cycle count, proof, timer in RTC RAM |
| `platform/demo.rs` | Flag text, JSON line, altitude helper |
| `runtime.rs` | wasmi host |
| `shutdown.rs` | Wipe + sleep |
| `main.rs` | Boot, demo loop, trigger resolve |

### Wiring (production breadboard)

- I2C: GPIO6 SDA, GPIO7 SCL @ 400 kHz  
- Wake: GPIO2, pull-down, active high  
- Status LED: GPIO10  
- OLED: SSD1306 @ 0x3C on same I2C bus  
- SD: SPI2 — GPIO3 MOSI, GPIO4 MISO, GPIO5 SCK, GPIO15 CS @ 400 kHz  

## 12. Build flow

```text
aerospace_payload (wasm32, release)
        → build.rs embeds WASM_BYTES
        → cargo +esp build → espflash → ESP32-C6
        → cargo +nightly build (x86_64) → bootimage → QEMU
```

Release profile: `opt-level = "z"`, LTO, `panic = "abort"`.

---

## QEMU exit code 33

Port `0xf4`, write `0x10` → exit `(16 << 1) | 1 = 33`. Success on bench only.

---

*AGPL-3.0-or-later*
