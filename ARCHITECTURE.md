# Aether Enclave — Architecture

Technical reference for this repo. Hardware target: **WeAct ESP32-C6-A-N4** (or DevKitC-1) on a breadboard. Bench target: **QEMU x86_64**.

Wiring steps and parts list: [README.md](README.md).

---

## 1. Purpose

On every wake, load a WebAssembly module, read sensors through a fixed host API, compare against limits, compute a proof hash linked to the previous cycle, log to serial/OLED/SD, zero RAM, and sleep.

Constraints:

- No OS, no network stack, no FAT on SD (fixed LBA sectors only).
- Two WASM binaries (strict / relaxed) are embedded in flash; host picks one per mission profile or pot position.
- Limits can also come from SD sector 2047 or host defaults.

---

## 2. Block diagram

```text
  Wake (button / timer / pressure)
           │
           ▼
  ┌────────────────────────────────────┐
  │ enclave_kernel (Rust, esp-hal)      │
  │  mission_profile ← SD sector 2047   │
  │  wasmi host → MMIO / serial         │
  │         │                           │
  │         ▼                           │
  │  aerospace_payload.wasm (strict or  │
  │  relaxed, selected at runtime)      │
  └────────┬───────────────┬───────────┘
           │ I2C GPIO6/7   │ proof
           ▼               ▼
     BMP390, ADS1115   OLED, USB log, SD SPI GPIO3/4/5/15
```

---

## 3. Boot sequence (ESP32-C6)

| Step | Location | Action |
|------|----------|--------|
| 1 | `esp_hal::init` | CPU, USB Serial/JTAG |
| 2 | `esp32c6::init` | I2C 400 kHz, WDT, GPIO2 wake, probe BMP390/ADS1115/OLED/SD |
| 3 | `mission_profile::load_from_sd` | Read sector 2047 if card present |
| 4 | `apply_pot_mission_profile` | Pot → wake timer, dose scale, optional relaxed slot |
| 5 | `detect_demo_mode_hold` | GPIO2 held 400 ms → demo loop (no sleep) |
| 6 | `resolve_trigger` | Wake cause, pressure wake, or cold-boot self-test |
| 7 | `run_mission_cycle` | WASM → `finish_cycle` → deep sleep |

Entry: `esp_main()` in `main.rs`.

---

## 4. Wake sources

| Vector | Label | ESP32-C6 source |
|--------|-------|-----------------|
| `0x20` | Pressure path | GPIO2 high, absolute pressure drop, or rapid leak rate |
| `0x21` | Timer path | RTC timer (seconds set from pot, clamped by profile) |

After deep sleep the CPU resets. Wake cause is read once from `SleepSource`; there is no ISR vector table on C6.

**RTC fast RAM** (`rtc_state.rs`, survives deep sleep): cycle count, last proof, last pressure (f32 bits), wake timer seconds, last cycle timestamp (ms).

---

## 5. Memory

| Resource | Size | Use |
|----------|------|-----|
| `ARENA` | 128 KiB | wasmi engine + store |
| Guest cap | 64 KiB | Max WASM linear memory at load |
| WASM blobs | flash `.rodata` | `WASM_STRICT` + `WASM_RELAXED` |

Each cycle: `reset_arena()` at start, `wipe_host_memory()` after `finish_cycle()`.

---

## 6. WASM bridge

### Host imports (`"aether"`)

| Import | ESP32-C6 source |
|--------|-----------------|
| `read_atmospheric_pressure` | BMP390, 3-sample average |
| `read_radiation_dosimeter` | ADS1115 AIN0, scaled by pot |
| `read_pressure_limit` | Mission profile (default 0.15 atm) |
| `read_dose_limit` | Mission profile (default 1000) |
| `commit_telemetry_vector` | Copy from guest RAM (bounds checked) |

### Guest export

`evaluate_limits() -> i32`:

| Value | Meaning |
|-------|---------|
| `0` | OK |
| `1` | Pressure below limit |
| `2` | Dose above limit |
| `3` | Both |

### Payload selection

`build.rs` compiles `aerospace_payload` twice (default + `relaxed` feature). `wasm_payload::wasm_bytes_for_slot(0|1)` picks the module. Slot comes from SD profile byte 5 or pot > ~75% when no SD profile.

---

## 7. Run pipeline

```text
run_mission_cycle(trigger)
  → power_log::mark_cycle_start()
  → reset_arena()
  → wasmi: load WASM for slot, link imports, call evaluate_limits
  → proof::chain_proof(prev, guest, sensors, vector, cycle, mission, slot)
  → finish_cycle() → serial, JSON, OLED, SD, wipe
  → enter_absolute_halt() → deep sleep (or demo loop repeats step)
```

Linker / trap errors print `ERR: …` on serial then fault shutdown (proof 0, still wipes and sleeps).

---

## 8. Proof chain

FNV-1a style 64-bit hash (`proof.rs`):

```text
hash ← FNV_OFFSET
hash ← mix(hash, prev_proof_lo/hi)
hash ← mix(hash, guest_result, pressure_bits, dose, vector, cycle, mission_id, payload_slot)
```

Serial and JSON include `prev_proof`. `proof_changed` is true when the new hash differs from RTC-stored last proof (expected every normal cycle).

Verify offline: `python tools/verify_log.py <logfile>`.

---

## 9. microSD sectors

| LBA | Magic | Content |
|-----|-------|---------|
| 2047 | `AEPR` | Mission profile (28-byte header + padding) |
| 2048 | `AETH` | Log write pointer + cycle count |
| 2049–2560 | `AEC1` | One text line per cycle (512 B each) |

Write profile from PC: `tools/write_mission_profile.py`. Export log: `tools/sd_export.py`.

---

## 10. After each cycle

`finish_cycle()`:

1. Log line + JSON (cycle, proof, prev_proof, mission, payload, active_ms)
2. `rtc_state::record_cycle`
3. OLED `show_cycle`
4. SD append if card mounted
5. `power_log::log_power_budget` (active ms + next sleep seconds)
6. GPIO10 LED off
7. Wipe arena and sandbox
8. Deep sleep — RTC timer + GPIO2 high wake

---

## 11. Source files

| File | Role |
|------|------|
| `main.rs` | Boot, demo loop, trigger resolve |
| `runtime.rs` | wasmi host, payload pick, proof commit |
| `proof.rs` | Chained hash |
| `shutdown.rs` | Log + wipe + sleep entry |
| `platform/esp32c6.rs` | I2C sensors, sleep, pressure wake |
| `platform/mission_profile.rs` | SD profile + runtime limits |
| `platform/rtc_state.rs` | Persistent RTC RAM |
| `platform/sd_log.rs` | SPI SD driver |
| `platform/oled.rs` | SSD1306 status |
| `platform/power_log.rs` | Cycle timing for serial |
| `platform/demo.rs` | Flag text, JSON helper |
| `build.rs` | Build + embed strict/relaxed WASM |

---

## 12. Build

```text
aerospace_payload (wasm32, strict + relaxed)
        → build.rs → wasm_payload.rs
        → cargo +esp build → espflash → ESP32-C6
        → cargo +nightly build (x86_64) → bootimage → QEMU
```

Release: `opt-level = "z"`, LTO, `panic = "abort"`.

---

## QEMU exit 33

Port `0xf4`, write `0x10` → QEMU exit code `(16 << 1) | 1 = 33`. Bench path only.

---

*AGPL-3.0-or-later*
