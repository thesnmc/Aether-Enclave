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

1. **Sleep** — x86: `hlt` with interrupts on. ESP32-C6: RTC deep sleep (10 s timer or GPIO2 high).
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

### Expected serial output (after flash)

```text
[AETHER] ESP32-C6 cold boot — USB Serial/JTAG ready
[AETHER] sensors — BMP390: OK (0x76)  ADS1115: OK (0x48)
[AETHER] snapshot — pressure=0.987 atm  dose=412 counts
[AETHER] cold boot — running WASM self-test (vector 0x20)
[AETHER] cycle done — guest=0 proof=0x........ vector=0x20 — wiping memory
```

After sleep, press the GPIO2 button or wait 10 s for the next cycle.

### Event demo (2 minutes)

1. Flash before the event: `cargo +esp run --release`
2. Serial monitor stays open on the USB port
3. **Cold boot** — self-test cycle runs automatically (proof line on screen)
4. **Button** (GPIO2 → 3.3V) — wake vector `0x20`, new cycle
5. **Pot** on ADS1115 AIN0 — turn up dose past 1000 → guest status `2` or `3`
6. **Wait 10 s** — timer wake vector `0x21`, cycle runs without touching anything

If a sensor shows `MISSING`, check power and I2C wiring before the demo — the kernel still runs, but reads will be zero.

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
| GPIO8 | I2C SDA (both sensors) |
| GPIO9 | I2C SCL (both sensors) |
| GPIO2 | Button to 3.3V (wake, vector 0x20) |
| ADS1115 AIN0 | Potentiometer wiper or sensor analog out |

I2C addresses: BMP390 `0x76`, ADS1115 `0x48`.

**Parts list (minimal):** ESP32-C6-DevKitC-1, BMP390 breakout, ADS1115 breakout, breadboard, jumper wires, tactile button, optional 10 kΩ pot for dose bench testing, USB-C cable.

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
