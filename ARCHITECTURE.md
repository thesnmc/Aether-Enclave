# Aether Enclave — Architecture

Technical reference for the Aether Enclave workspace: a `#![no_std]` bare-metal kernel that runs a `wasm32-unknown-unknown` payload in **wasmi**, with MMIO-mapped sensors and a mandatory memory wipe after each run.

---

## Table of contents

1. [Design goals](#1-design-goals)
2. [System layout](#2-system-layout)
3. [Boot sequence](#3-boot-sequence)
4. [Wake and sleep model](#4-wake-and-sleep-model)
5. [Memory layout](#5-memory-layout)
6. [WASM host bridge (`aether`)](#6-wasm-host-bridge-aether)
7. [Run pipeline](#7-run-pipeline)
8. [MMIO register map](#8-mmio-register-map)
9. [Memory wipe and sleep](#9-memory-wipe-and-sleep)
10. [QEMU exit code 33](#10-qemu-exit-code-33)
11. [Build flow](#11-build-flow)
12. [ESP32-C6 vs classic ESP32](#12-esp32-c6-vs-classic-esp32)

---

## 1. Design goals

| Goal | How |
|------|-----|
| No data left in RAM after a cycle | Bump arena reset + sandbox `fill(0)` + register scrub |
| Guest isolation | Static sandbox buffer; `cap_guest_memory` enforces Wasm linear memory cap |
| Predictable timing | No scheduler: wake → run WASM → wipe → sleep |
| Checkable output | 64-bit proof hash written to MMIO before sleep |
| Swappable logic | Rebuild `aerospace_payload.wasm`; kernel embeds bytes at compile time |

---

## 2. System layout

```text
                    ┌─────────────────────────────────────────┐
  Wake 0x20/0x21 ─▶│           enclave_kernel               │
                    │  ┌─────────┐  ┌──────────┐  ┌────────┐ │
                    │  │ IDT or  │─▶│ AetherHost│─▶│ MMIO   │ │
                    │  │ wake    │  │  (wasmi)  │  │ / log  │ │
                    │  └─────────┘  └─────┬────┘  └────────┘ │
                    │                     │ imports "aether"  │
                    │               ┌─────▼─────┐             │
                    │               │ WASM guest│             │
                    │               │ aerospace_│             │
                    │               │  payload  │             │
                    │               └───────────┘             │
                    └─────────────────────────────────────────┘
                              ▲                    │
                              │ sensor reads       │ proof hash
                              └────────────────────┘
                         (QEMU sim or I2C on C6)
```

**Trust boundaries:**

- **Guest linear memory** — stack, `TelemetryRecord`, Wasm locals only.
- **Host only** — bump arena, wasmi engine, IDT (x86) or platform drivers (C6), MMIO staging.
- **Bridge** — Wasm imports implemented by `HostCalls` + `Linker::func_wrap`.

---

## 3. Boot sequence

### x86_64 (QEMU)

| Step | Component | Action |
|------|-----------|--------|
| 1 | `bootloader` 0.9 | Load kernel ELF, set up page tables, jump to `kernel_main` |
| 2 | `kernel_main` | `reset_arena()`, COM1 init, IDT install |
| 3 | Bench (dev) | `sim_inject_o2_drop()` + software IRQ `0x20` |
| 4 | `dormancy_loop` | `sti` → `hlt` until wake → run pipeline |

Entry: `bootloader::entry_point!(kernel_main)`.

### ESP32-C6 (hardware)

| Step | Component | Action |
|------|-----------|--------|
| 1 | ROM + esp-hal | `esp_hal::init`, USB Serial/JTAG ready via esp-println |
| 2 | `esp_main` | Platform init (I2C, WDT, GPIO2), `reset_arena()` |
| 3 | Wake decode | If not cold boot, map RTC wake cause → vector 0x20 or 0x21 |
| 4 | Run or sleep | Run pipeline on wake; otherwise deep sleep (10 s + GPIO2) |

Entry: `#[esp_hal::main] fn esp_main()`.

---

## 4. Wake and sleep model

### Vectors (same IDs on both targets)

| Vector | Name | Typical cause |
|--------|------|---------------|
| `0x20` | `AtmosphericPressureThreshold` | Pressure below limit / GPIO2 wake on C6 |
| `0x21` | `KineticJointActuation` | Strain pulse / 10 s timer wake on C6 |

### x86_64

ISRs mask nested interrupts, latch the vector, and call the run pipeline. Dormant loop uses `sti` + `hlt`.

`software_trigger()` injects an IRQ for QEMU bench runs.

### ESP32-C6

There is no IDT. Deep sleep resets the CPU. On boot, `detect_wake_trigger()` reads `SleepSource`:

- `Timer` → vector `0x21`
- `Gpio` / `Ext0` / `Ext1` → vector `0x20`
- Power-on reset → `None` (no WASM run until first wake)

---

## 5. Memory layout

All backing stores are **static** `Mutex` buffers sized at compile time.

### ESP32-C6 (512 KiB SRAM budget)

| Region | Constant | Size | Purpose |
|--------|----------|------|---------|
| Guest sandbox cap | `SANDBOX_MEMORY_SIZE` | 64 KiB | Max Wasm linear memory |
| Host bump arena | `ARENA_SIZE` | 128 KiB | wasmi compile + instantiate + heap |
| ISR stack | `ISR_STACK_SIZE` | 4 KiB | Reserved (x86 ISR path) |

### x86_64 (QEMU — more RAM available)

Same constants in code today; QEMU has headroom if you increase them for larger guests.

### Bump arena

- Alloc: align cursor, bump pointer; no free until `reset_arena()`.
- Each wake calls `reset_arena()` first for a clean heap.

### `cap_guest_memory`

After instantiate, validate exported `memory`:

```rust
guest_bytes = pages × 65_536
if guest_bytes > SANDBOX_MEMORY_SIZE { trap }
```

Typical `aerospace_payload` fits within the 64 KiB cap when built with `opt-level = "z"`.

---

## 6. WASM host bridge (`aether`)

### Import module name: `"aether"`

| Symbol | Type | Host |
|--------|------|------|
| `read_atmospheric_pressure` | `() -> f32` | BMP390 on C6; simulated on x86 |
| `read_radiation_dosimeter` | `() -> i32` | ADS1115 on C6; simulated on x86 |
| `commit_telemetry_vector` | `(i32, i32) -> ()` | Bounds-checked copy from guest memory |
| `commit_uplink` | `(i32, i32) -> ()` | Optional guest proof write |

### Guest exports

| Symbol | Role |
|--------|------|
| `evaluate_limits` | Main entry: `() -> i32` status flags |
| `diagnostic` | Alias to `evaluate_limits` |
| `payload_version` | Returns `0xA17E_0001` |
| `memory` | Linear memory export |

### Status flags (`evaluate_limits` return)

| Flag | Value | Condition |
|------|-------|-----------|
| `STATUS_PRESSURE_LOW` | `0x1` | pressure < 0.15 atm |
| `STATUS_DOSE_HIGH` | `0x2` | dose > 1000 |
| Both | `0x3` | x86 bench: 0.12 atm, 1250 dose |

---

## 7. Run pipeline

`runtime::sovereign_bootstrap(trigger)`:

```text
reset_arena()
  → AetherHost::instantiate(trigger)
  → run_diagnostic()     // call evaluate_limits
  → commit_outcome()
  → shutdown::self_annihilate()
```

On failure: log error class on serial → wipe with proof zeroed.

No threads, no async.

---

## 8. MMIO register map

Logical addresses (stable across targets; backed by atomics or live sensor reads):

| Symbol | Address | Content |
|--------|---------|---------|
| `REG_ATOMIC_O2_SENSOR` | `0xFEF0_0000` | Raw ADC counts |
| `REG_KINETIC_JOINT` | `0xFEF0_0004` | Strain gauge |
| `REG_ATMOSPHERIC_PRESSURE` | `0xFEF0_0008` | `f32` bit pattern (atm) |
| `REG_RADIATION_DOSIMETER` | `0xFEF0_000C` | Dose counts |
| `REG_UPLINK_COMMIT_LO` | `0xFEF0_0010` | Proof low 32 bits |
| `REG_UPLINK_COMMIT_HI` | `0xFEF0_0014` | Proof high 32 bits |
| `REG_PMU_COMMAND` | `0xFEF0_0020` | Sleep command latch |

### Proof hash (host, after guest returns)

```text
proof_lo = (guest_result as u32) XOR last_dose XOR last_sensor
proof_hi = last_dose.rotate_left(9) XOR to_bits(last_pressure) XOR 0xA17E_0001
proof    = (proof_hi << 32) | proof_lo
```

### ESP32-C6 I2C wiring (see `platform/esp32c6.rs`)

- SDA: GPIO8, SCL: GPIO9 @ 400 kHz
- BMP390 @ `0x76`, ADS1115 @ `0x48`
- Wake button: GPIO2 (active high, pull-down)

---

## 9. Memory wipe and sleep

`shutdown::self_annihilate` runs on success and on kernel panic.

| Step | Action |
|------|--------|
| 1 | Log `ShutdownReport` on serial |
| 2 | `annihilate_sandbox()` — zero sandbox |
| 3 | `reset_arena()` — zero heap arena |
| 4 | `clear_architectural_state()` — zero GPRs |
| 5 | `request_dormancy()` — PMU latch |
| 6 | Platform sleep |

**x86:** write QEMU debug exit port `0xf4`, then `cli` + `hlt`.

**ESP32-C6:** RTC deep sleep (feeds WDT during long zero loops).

---

## 10. QEMU exit code 33

Runner device: `isa-debug-exit,iobase=0xf4`.

Kernel writes `0x10` to port `0xf4`. QEMU exit code = `(value << 1) | 1` → **33 = success**.

Not used on ESP32-C6; use the serial proof line instead.

---

## 11. Build flow

```text
aerospace_payload (wasm32-unknown-unknown, release, cdylib)
        │
        ▼
target/wasm-payload/wasm32-unknown-unknown/release/aerospace_payload.wasm
        │
        ▼  enclave_kernel/build.rs
enclave_kernel/src/wasm_payload.rs   // pub const WASM_BYTES: &[u8]
        │
        ├──▶ cargo +esp build  → espflash → ESP32-C6
        └──▶ cargo +nightly build (x86_64) → bootimage → QEMU
```

Release profile: `opt-level = "z"`, `lto = true`, `panic = "abort"`.

---

## 12. ESP32-C6 vs classic ESP32

| | Classic ESP32 | ESP32-C6 |
|---|---------------|----------|
| ISA | Xtensa | RISC-V (`riscv32imac`) |
| Rust bare-metal | Custom Xtensa LLVM; harder without FreeRTOS | First-class esp-hal support |
| USB debug | External UART chip or JTAG probe | On-chip USB Serial/JTAG |
| 802.15.4 radio | Not present | Present (unused in this project) |

This kernel does not implement networking of any kind. The C6 was chosen for RISC-V tooling and native USB flash/debug on a budget board.

---

## Source file map

| Path | Role |
|------|------|
| `enclave_kernel/src/main.rs` | Entry, sleep loop, panic handler |
| `enclave_kernel/src/platform/esp32c6.rs` | I2C sensors, WDT, deep sleep |
| `enclave_kernel/src/runtime.rs` | wasmi host, memory cap, run pipeline |
| `enclave_kernel/src/memory.rs` | Arena, sandbox, global allocator |
| `enclave_kernel/src/mmio.rs` | Registers, sensors, serial / esp-println |
| `enclave_kernel/src/interrupts.rs` | IDT (x86), wake decode (C6) |
| `enclave_kernel/src/shutdown.rs` | Wipe + platform sleep |
| `aerospace_payload/src/lib.rs` | WASM diagnostic logic |

---

*Aether Enclave — AGPL-3.0-or-later*
