# PROJECT AETHER-ENCLAVE

Cargo workspace: **`enclave_kernel`** (x86_64 bare-metal host) + **`aerospace_payload`** (`wasm32-unknown-unknown` guest).

## Layout

```text
aether_enclave/
├── Cargo.toml                 # [workspace] members
├── rust-toolchain.toml
├── .cargo/config.toml         # build-std + QEMU runner (workspace builds)
├── aerospace_payload/         # #![no_std] WASM diagnostic (cdylib)
│   ├── Cargo.toml
│   └── src/lib.rs
└── enclave_kernel/            # Unikernel
    ├── .cargo/config.toml
    ├── build.rs                 # `cargo build -p aerospace_payload` → wasm_payload.rs
    ├── Cargo.toml
    ├── link.x
    └── src/
        ├── main.rs
        ├── wasm_payload.rs    # AUTO-GENERATED (WASM_BYTES)
        └── …
```

## Build

```bash
rustup target add x86_64-unknown-none wasm32-unknown-unknown
rustup component add rust-src --toolchain nightly

cargo +nightly build -p enclave_kernel \
  -Z build-std=core,alloc,compiler_builtins \
  -Z build-std-features=compiler-builtins-mem
```

`enclave_kernel/build.rs` compiles `aerospace_payload` for `wasm32-unknown-unknown` (release) and embeds `target/wasm32-unknown-unknown/release/aerospace_payload.wasm` as `WASM_BYTES`.

## QEMU

```bash
cargo +nightly run -p enclave_kernel \
  -Z build-std=core,alloc,compiler_builtins \
  -Z build-std-features=compiler-builtins-mem
```

Requires `bootimage` and QEMU (`isa-debug-exit` at port `0xf4` configured in `enclave_kernel/Cargo.toml`).

## Edit the guest payload

Change `aerospace_payload/src/lib.rs`, then rebuild `enclave_kernel` — `wasm_payload.rs` is regenerated automatically.
