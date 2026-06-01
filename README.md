# Aether Enclave

[![License: AGPL-3.0-or-later](https://img.shields.io/badge/License-AGPL--3.0--or--later-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-nightly%20%2B%20stable-orange.svg)](https://www.rust-lang.org/)
[![no_std](https://img.shields.io/badge/no__std-bare--metal-critical.svg)](enclave_kernel/)

**Aether Enclave** is a radiation-resilient, bare-metal WebAssembly unikernel for aerospace, maritime, and extreme edge deployments. It runs sovereign diagnostic logic in an isolated WASM sandbox, commits a verifiable outcome to hardware MMIO, and **self-annihilates**—zeroing memory and returning to deep dormancy—so no persistent attack surface or data residue remains between wake cycles.

Built for operators who require **absolute data sovereignty**, **privacy-first** execution (no network stack, no disk, no OS), and **stateless** wake-run-scrub cycles on every physical interrupt.

---

## Why Aether Enclave?

| Principle | What it means in practice |
|-----------|---------------------------|
| **Data sovereignty** | Sensor reads and uplink commits occur only through a fixed MMIO map you control; the guest never sees host pointers. |
| **Privacy-first** | `#![no_std]` Ring-0 image: no libc, no scheduler, no background services. |
| **Stateless execution** | Each IRQ triggers `reset_arena()` → WASM run → sandbox annihilation → `cli`/`hlt`; nothing survives the cycle. |
| **Edge / radiation posture** | Static allocation, bump arena, strict guest linear-memory cap—predictable memory, no heap fragmentation surprises. |

---

## How It Works

Every mission cycle follows the same deterministic pipeline:

```text
┌─────────────┐    ┌──────────────┐    ┌─────────────────┐    ┌──────────────────┐    ┌──────────────────┐
│  Dormancy   │───▶│ Hardware IRQ │───▶│  WASM Payload   │───▶│ MMIO Proof Commit │───▶│ Self-Annihilation │
│  (HLT/STI)  │    │  0x20 / 0x21 │    │ evaluate_limits │    │ 64-bit digest    │    │ zero + cli/hlt   │
└─────────────┘    └──────────────┘    └─────────────────┘    └──────────────────┘    └──────────────────┘
```

1. **Dormancy** — The core idles with interrupts enabled, executing `hlt` until a physical (or bench-injected) IRQ fires.
2. **Hardware IRQ** — Vectors `0x20` (atmospheric pressure threshold) or `0x21` (kinetic joint) latch the wake; the ISR calls `sovereign_bootstrap` with interrupts masked.
3. **WASM payload** — `aerospace_payload` runs inside **wasmi** on a fresh 4 MiB bump arena; it reads pressure/dose via the `aether` host bridge, evaluates limits, and commits telemetry.
4. **MMIO proof** — The host fuses guest status + sensor state into a 64-bit digest and writes `REG_UPLINK_COMMIT_LO/HI` (verifiable outcome for ground systems / ZKP pipelines).
5. **Self-annihilation** — Sandbox and arena are zeroed, GPRs cleared, PMU dormancy issued, then `cli` + `hlt` (QEMU: `isa-debug-exit` with success code **33**).

See **[ARCHITECTURE.md](ARCHITECTURE.md)** for register maps, memory layout, and boot/IDT details.

---

## Workspace

| Crate | Target | Role |
|-------|--------|------|
| [`enclave_kernel`](enclave_kernel/) | `x86_64-unknown-none` | Ring-0 unikernel: bootloader entry, IDT, MMIO, wasmi host, shutdown |
| [`aerospace_payload`](aerospace_payload/) | `wasm32-unknown-unknown` | `#![no_std]` cdylib guest: limit evaluation + telemetry |

`enclave_kernel/build.rs` compiles the payload at kernel build time and embeds `WASM_BYTES` in `src/wasm_payload.rs` (auto-generated; do not edit by hand).

---

## Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) (toolchain per [`rust-toolchain.toml`](rust-toolchain.toml))
- **Nightly** (for `build-std` on the kernel target)
- [`bootimage`](https://github.com/rust-osdev/bootimage) — `cargo install bootimage`
- **QEMU** `qemu-system-x86_64` with `isa-debug-exit` (configured in [`enclave_kernel/Cargo.toml`](enclave_kernel/Cargo.toml))

```bash
rustup target add x86_64-unknown-none wasm32-unknown-unknown
rustup component add rust-src --toolchain nightly
cargo install bootimage
```

### Build

From the workspace root:

```bash
cargo +nightly build -p enclave_kernel \
  -Z build-std=core,alloc,compiler_builtins \
  -Z build-std-features=compiler-builtins-mem
```

### Run (QEMU)

```bash
cargo +nightly run -p enclave_kernel \
  -Z build-std=core,alloc,compiler_builtins \
  -Z build-std-features=compiler-builtins-mem
```

Serial output appears on **COM1** (`-serial stdio`). The stock bench harness injects an O₂/pressure drop and software IRQ `0x20`; a successful cycle prints:

```text
[AETHER] cycle success — guest=3 proof=0x........ vector=0x20 — self-annihilation
```

QEMU then exits with **process exit code `33`** — that is the expected success path (see [ARCHITECTURE.md § QEMU exit code 33](ARCHITECTURE.md#qemu-exit-code-33)).

### Edit the guest payload

Change [`aerospace_payload/src/lib.rs`](aerospace_payload/src/lib.rs), then rebuild `enclave_kernel`; `wasm_payload.rs` regenerates automatically.

---

## WASM Host Bridge (at a glance)

Guest imports (module `"aether"`):

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

Under bench injection (`sim_inject_o2_drop`: **0.12 atm**, **1250 dose**), `evaluate_limits` returns status bitmask **`3`** (`STATUS_PRESSURE_LOW | STATUS_DOSE_HIGH`).

---

## License

**AGPL-3.0-or-later** — see crate manifests (`enclave_kernel`, `aerospace_payload`). Network deployment or SaaS use may require source distribution to users; review the license before production deployment.

---

## Further reading

- **[ARCHITECTURE.md](ARCHITECTURE.md)** — Ring-0 boot, memory enclave, `cap_guest_memory`, IDT/ISR, proof algebra, self-annihilation
