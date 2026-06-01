# Aether Enclave — Systems Architecture

This document is the technical blueprint for the **Aether Enclave** workspace: a `#![no_std]` x86_64 unikernel that hosts a `wasm32-unknown-unknown` diagnostic payload under **wasmi**, with hardware-tethered MMIO and a mandatory post-run annihilation phase.

---

## Table of contents

1. [Design goals](#1-design-goals)
2. [Physical and logical topology](#2-physical-and-logical-topology)
3. [Ring-0 boot sequence](#3-ring-0-boot-sequence)
4. [Interrupt and dormancy model](#4-interrupt-and-dormancy-model)
5. [Memory architecture](#5-memory-architecture)
6. [WASM host bridge (`aether`)](#6-wasm-host-bridge-aether)
7. [Sovereign bootstrap pipeline](#7-sovereign-bootstrap-pipeline)
8. [MMIO map and proof commit](#8-mmio-map-and-proof-commit)
9. [Self-annihilation](#9-self-annihilation)
10. [QEMU exit code 33](#10-qemu-exit-code-33)
11. [Build and artifact flow](#11-build-and-artifact-flow)

---

## 1. Design goals

| Goal | Mechanism |
|------|-----------|
| **No persistent secrets** | Bump arena reset + sandbox `fill(0)` + GPR `xor` scrub every cycle |
| **Guest isolation** | Separate static regions; `cap_guest_memory` enforces Wasm linear memory ≤ 2 MiB |
| **Deterministic wake work** | No scheduler: ISR → `sovereign_bootstrap` → shutdown |
| **Hardware-attested outcome** | 64-bit proof written to MMIO before power-down |
| **Replaceable mission logic** | Rebuild `aerospace_payload.wasm`; kernel embeds bytes at compile time |

---

## 2. Physical and logical topology

```text
                    ┌─────────────────────────────────────────────┐
                    │           enclave_kernel (Ring 0)            │
                    │  ┌─────────┐  ┌──────────┐  ┌────────────┐ │
  IRQ 0x20/0x21 ───▶│  │   IDT   │─▶│ AetherHost│─▶│ MMIO / UART │ │
                    │  └─────────┘  │  (wasmi)  │  └────────────┘ │
                    │               └─────┬────┘                  │
                    │                     │ imports "aether"       │
                    │               ┌─────▼─────┐                  │
                    │               │ WASM guest │                 │
                    │               │ aerospace_ │                 │
                    │               │  payload   │                 │
                    │               └────────────┘                 │
                    └─────────────────────────────────────────────┘
                              ▲                    │
                              │ MMIO reads/writes  │ proof / telemetry
                              └────────────────────┘
                         (simulated flight sensors)
```

**Trust boundaries:**

- **Inside guest linear memory** — stack, `TelemetryRecord`, Wasm locals only.
- **Host-only** — bump arena, wasmi engine/store, IDT, ISR stack, MMIO simulation atoms.
- **Bridge** — `extern "C"` Wasm imports implemented by `HostCalls` + `Linker::func_wrap`.

---

## 3. Ring-0 boot sequence

| Phase | Component | Behavior |
|-------|-----------|----------|
| 1 | **bootloader** (`bootloader` 0.9) | Loads the kernel ELF, sets up stack and page tables, jumps to `kernel_main` with `BootInfo` |
| 2 | `kernel_main` | `memory::reset_arena()`, `mmio::serial_init()`, `interrupts::init()` (install IDT on x86_64) |
| 3 | Bench harness (dev) | `sim_inject_o2_drop()` + `software_trigger(0x20)` — optional pre-flight simulation |
| 4 | `dormancy_loop` | `sti` → `hlt` until `wake_pending()` → `sovereign_bootstrap` |

The kernel is `#![no_main]`; entry is via `bootloader::entry_point!(kernel_main)`.

**Linking:** `.cargo/config.toml` sets `target = x86_64-unknown-none`, `build-std` for `core`/`alloc`, and `relocation-model=static`. Custom layout may be provided via `enclave_kernel/link.x`.

---

## 4. Interrupt and dormancy model

### Vectors

| Vector | `HardwareInterrupt` | Typical cause |
|--------|---------------------|---------------|
| `0x20` | `AtmosphericPressureThreshold` | Barometric / O₂ partial pressure below mission limit |
| `0x21` | `KineticJointActuation` | Deployment / joint strain pulse |

### ISR contract

On x86_64, ISRs:

1. **`cli`** — suppress nested interrupts during bootstrap.
2. Latch `LAST_VECTOR` and `WAKE_PENDING`.
3. Invoke `runtime::sovereign_bootstrap` (or defer to dormant loop depending on path).

The **dormant core** runs with interrupts **enabled** (`sti`) and blocks on **`hlt`**. This approximates a C1-equivalent wait for the next physical trigger.

### Software trigger (HIL)

`interrupts::software_trigger` sets the wake latch without external hardware—used by QEMU bench runs in `main.rs`.

---

## 5. Memory architecture

All host and guest backing stores are **static**, guarded by `spin::Mutex`, and sized at compile time.

### Region summary

| Region | Constant | Size | Purpose |
|--------|----------|------|---------|
| Guest sandbox backing | `SANDBOX_MEMORY_SIZE` | **2 MiB** | Upper bound for Wasm linear memory validation |
| Host bump arena | `ARENA_SIZE` / `HEAP_SIZE` | **4 MiB** | `#[global_allocator]` for wasmi module compile, instantiate, interpreter |
| ISR stack | `ISR_STACK_SIZE` | 4 KiB | Dedicated interrupt stack |
| Wasm page | `WASM_PAGE_SIZE` | 64 KiB (65,536 B) | Spec page size for `cap_guest_memory` math |

```text
  Host address space (conceptual)
  ┌──────────────────────────────────────┐  high
  │  ARENA (4 MiB) — wasmi, alloc        │
  ├──────────────────────────────────────┤
  │  SANDBOX (2 MiB) — guest mem cap     │
  ├──────────────────────────────────────┤
  │  ISR_STACK (4 KiB)                   │
  └──────────────────────────────────────┘  low / static .bss
```

### Bump arena (`ArenaAllocator`)

- **Alloc:** align cursor, bump, return pointer; **no free** until `reset_arena()`.
- **OOM:** returns null → allocation fails → may panic in wasmi/alloc paths.
- **Per-cycle reset:** `sovereign_bootstrap` calls `memory::reset_arena()` first so each wake gets a fresh 4 MiB budget.

Production note: a 4 MiB arena was required empirically for `Module::new` + `linker.instantiate` on the embedded diagnostic module; 128 KiB caused silent OOM panics before trap logging.

### `cap_guest_memory` — strict linear memory perimeter

After `linker.instantiate`, the host validates exported `memory`:

```rust
let pages = mem.current_pages(&*store);
let page_count = u32::from(pages) as usize;
let guest_bytes = page_count * WASM_PAGE_SIZE;  // pages × 65_536

// Invariant: wasmi slice length must match page-derived size
assert data_len == guest_bytes;

// Policy: reject if guest linear memory exceeds static sandbox policy
if guest_bytes > SANDBOX_MEMORY_SIZE { trap; }
```

| Check | Rationale |
|-------|-----------|
| `pages × 65536` | Wasm spec page size; avoids ambiguity vs raw `len()` alone |
| `data_len == guest_bytes` | Detects inconsistent wasmi state early |
| `guest_bytes ≤ 2 MiB` | Rust `wasm32` modules with 16+ pages (1 MiB+) must not exceed enclave policy |

The checker **does not zero** guest linear memory post-instantiate (data/BSS already initialized by the loader; zeroing caused spurious traps).

Typical `aerospace_payload` module: **16 minimum pages** (1 MiB linear memory) — fits within the 2 MiB cap.

---

## 6. WASM host bridge (`aether`)

### Module naming

| Side | Name |
|------|------|
| Wasm import module | `"aether"` (`HOST_IMPORT_MODULE`) |
| Guest Rust | `#[link(wasm_import_module = "aether")]` |
| Host linker | `Linker::func_wrap("aether", …)` |

### Import / export contract

**Guest imports (required for production `evaluate_limits`):**

| Symbol | Wasm type | Host implementation |
|--------|-----------|---------------------|
| `read_atmospheric_pressure` | `() -> f32` | `mmio::read_atmospheric_pressure()` → `wasmi_core::F32` |
| `read_radiation_dosimeter` | `() -> i32` | `mmio::read_radiation_dosimeter()` as `i32` |
| `commit_telemetry_vector` | `(i32, i32) -> ()` | Bounds-checked copy from guest linear memory → MMIO staging |
| `commit_uplink` | `(i32, i32) -> ()` | Optional guest-driven proof write (host also commits post-run) |

**Guest exports:**

| Symbol | Role |
|--------|------|
| `evaluate_limits` | Primary entry: `() -> i32` status bitmask |
| `diagnostic` | Alias forwarding to `evaluate_limits` |
| `payload_version` | `() -> u32` magic `0xA17E_0001` |
| `memory` | Exported linear memory (Rust `static` data) |

### Guest status bitmask (`evaluate_limits` return)

| Flag | Value | Condition |
|------|-------|-----------|
| `STATUS_PRESSURE_LOW` | `0x1` | `pressure < 0.15` atm |
| `STATUS_DOSE_HIGH` | `0x2` | `dose > 1000` |
| `STATUS_BOTH` | `0x3` | Bench injection: 0.12 atm, 1250 dose |

### Guest source pattern

```rust
#[no_mangle]
pub extern "C" fn evaluate_limits() -> i32 {
    let pressure = unsafe { read_atmospheric_pressure() };
    let dose = unsafe { read_radiation_dosimeter() } as u32;
    // ... limit checks ...
    unsafe { commit_telemetry_vector(ptr, len) };
    flags
}
```

### Host `HostState` (per-store context)

Caches `last_pressure`, `last_dose`, `last_sensor`, `guest_result`, and `trigger` for proof fusion after the guest returns.

### Instantiation sequence (`AetherHost::instantiate`)

1. `Engine::new` / `Store::new`
2. `Module::new(WASM_BYTES)`
3. `link_aether_host` — register all `aether` imports
4. `linker.instantiate` + `ensure_no_start`
5. `cap_guest_memory`
6. Resolve `evaluate_limits` or fallback `diagnostic` as `TypedFunc<(), i32>`

Errors are classified on COM1 without heap formatting: `ERR: Linker`, `Instantiation`, `Trap`, `Unknown`.

---

## 7. Sovereign bootstrap pipeline

`runtime::sovereign_bootstrap(trigger)` — single cooperative “micro-cycle”:

```text
reset_arena()
    → AetherHost::instantiate(trigger)
    → run_diagnostic()          // diagnostic.call(())
    → commit_outcome(guest_result)
    → shutdown::self_annihilate(ShutdownReport { guest_result, proof, vector })
```

On failure: `log_wasmi_error` → `fault_shutdown(-1)` (proof zeroed).

**No scheduler, no threads, no async.** ISR stack and interrupt masking keep the path bounded.

---

## 8. MMIO map and proof commit

### Sensor / actuator registers (placeholder physical addresses)

| Symbol | Address | Width | Content |
|--------|---------|-------|---------|
| `REG_ATOMIC_O2_SENSOR` | `0xFEF0_0000` | u32 | Simulated ADC counts |
| `REG_KINETIC_JOINT` | `0xFEF0_0004` | u32 | Strain gauge |
| `REG_ATMOSPHERIC_PRESSURE` | `0xFEF0_0008` | u32 | `f32` bit pattern (atm) |
| `REG_RADIATION_DOSIMETER` | `0xFEF0_000C` | u32 | Dose units |
| `REG_UPLINK_COMMIT_LO` | `0xFEF0_0010` | u32 | Proof low word |
| `REG_UPLINK_COMMIT_HI` | `0xFEF0_0014` | u32 | Proof high word |
| `REG_PMU_COMMAND` | `0xFEF0_0020` | u32 | `PMU_CMD_DORMANT`, etc. |

Simulation uses atomics + `Mutex` for telemetry buffer (`TELEMETRY_VECTOR_CAP = 64`).

### Telemetry record (guest → host)

```rust
#[repr(C)]
pub struct TelemetryRecord {
    pub flags: u8,
    pub _pad: [u8; 3],
    pub pressure_bits: u32,
    pub dose: u32,
}
```

Guest passes a pointer/length into `commit_telemetry_vector`; host copies from **guest linear memory** after bounds checks.

### 64-bit proof digest (host post-run)

After `evaluate_limits` returns, `commit_outcome` computes:

```text
proof_lo = (guest_result as u32) XOR last_dose XOR last_sensor
proof_hi = last_dose.rotate_left(9) XOR to_bits(last_pressure) XOR 0xA17E_0001
proof    = (proof_hi << 32) | proof_lo
```

Written via `mmio::commit_proof` to uplink registers. This is a **compact cryptographic-style commitment** suitable for ground verification or downstream ZKP circuits—not a full proving system on-device.

### Bench injection

`sim_inject_o2_drop()`:

- Pressure → **0.12** atm  
- Dose → **1250**  
- Triggers software IRQ **0x20**

Expected serial line: `guest=3`, non-zero `proof=0x…`, `vector=0x20`.

---

## 9. Self-annihilation

`shutdown::self_annihilate` is `-> !` and runs on **success** and on **panic** (via `#[panic_handler]` in `main.rs`).

| Step | Action |
|------|--------|
| 1 | Log `ShutdownReport` on COM1 |
| 2 | `memory::annihilate_sandbox()` — zero 2 MiB sandbox, seal |
| 3 | `memory::reset_arena()` — zero 4 MiB arena cursor |
| 4 | `clear_architectural_state()` — `xor` GPRs |
| 5 | `mmio::request_dormancy()` — PMU command register |
| 6 | `enter_absolute_halt()` — see below |

### Hardware halt sequence (x86_64)

```rust
// 1. QEMU test exit (when device present)
Port::<u32>::new(0xf4).write(0x10);

// 2. Absolute halt
asm!("cli", "hlt", options(nomem, nostack, noreturn));
```

**`cli`** — interrupts disabled before final halt (no further ISR delivery).  
**`hlt`** — processor sleep until reset/NMI; in QEMU with `isa-debug-exit`, the I/O write typically ends the VM first.

---

## 10. QEMU exit code 33

The runner attaches:

```text
-device isa-debug-exit,iobase=0xf4,iosize=0x04
```

On success, the kernel writes to I/O port **`0xf4`**:

```rust
const QEMU_DEBUG_EXIT_SUCCESS: u32 = 0x10;
debug_exit.write(QEMU_DEBUG_EXIT_SUCCESS);
```

**QEMU `isa-debug-exit` semantics:** an I/O write of value `V` terminates the virtual machine with host process exit code:

```text
exit_code = (V << 1) | 1
```

For `V = 0x10` (16):

```text
exit_code = (16 << 1) | 1 = 33
```

Therefore **`33` is success**, not failure. CI harnesses should treat `33` as a clean mission cycle completion after the self-annihilation path.

---

## 11. Build and artifact flow

```text
aerospace_payload (wasm32-unknown-unknown, release, cdylib)
        │
        ▼
target/wasm32-unknown-unknown/release/aerospace_payload.wasm
        │
        ▼  enclave_kernel/build.rs
enclave_kernel/src/wasm_payload.rs   // pub const WASM_BYTES: &[u8]
        │
        ▼  cargo +nightly build -Z build-std …
bootimage → QEMU x86_64 bare-metal binary
```

| Input change | Rebuild trigger |
|--------------|-----------------|
| `aerospace_payload/src/**` | `build.rs` rerun → new `WASM_BYTES` |
| `enclave_kernel/src/**` | Normal Rust rebuild |

**Profiles:** workspace `release` uses `opt-level = "z"`, `lto = true`, `panic = "abort"` — appropriate for size-constrained flight images.

---

## Related files

| Path | Responsibility |
|------|----------------|
| `enclave_kernel/src/main.rs` | Boot, dormancy loop, panic handler |
| `enclave_kernel/src/runtime.rs` | wasmi host, `cap_guest_memory`, bootstrap |
| `enclave_kernel/src/memory.rs` | Arena, sandbox, global allocator |
| `enclave_kernel/src/mmio.rs` | Register map, sensors, proof, UART |
| `enclave_kernel/src/interrupts.rs` | IDT, vectors, `hlt`/`cli` helpers |
| `enclave_kernel/src/shutdown.rs` | Annihilation + QEMU exit |
| `aerospace_payload/src/lib.rs` | Mission WASM logic |

---

*Aether Enclave — AGPL-3.0-or-later. Stateless by design.*
