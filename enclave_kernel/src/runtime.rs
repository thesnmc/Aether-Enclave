//! Embedded WebAssembly host (`AetherHost`) — wasmi on a static bump arena.
//!
//! Guest modules import [`HostCalls`] from the `"aether"` module. Linear memory, when
//! present, is bounded by [`crate::memory::SANDBOX_MEMORY_SIZE`].

use wasmi::{
    Caller, Config, Engine, Error, Extern, Instance, Linker, Memory, MemoryType, Module, Store,
    TypedFunc,
};
use wasmi::errors::MemoryError;
use wasmi_core::F32;

use crate::interrupts::{self, HardwareInterrupt};
use crate::memory::{self, MemoryFault, SandboxRegion, SANDBOX_MEMORY_SIZE, WASM_PAGE_SIZE};
use crate::mmio;
use crate::serial_println;
use crate::shutdown::{self, ShutdownReport};
use crate::wasm_payload;

/// Host-side execution context for one wake cycle.
pub struct HostState {
    /// Physical trigger that launched this cycle.
    pub trigger: Option<HardwareInterrupt>,
    /// Last legacy sensor reading (proof chaining).
    pub last_sensor: u32,
    /// Last atmospheric pressure sample.
    pub last_pressure: f32,
    /// Last dosimeter reading.
    pub last_dose: u32,
    /// Cached guest return value.
    pub guest_result: i32,
}

/// Wasm import module namespace (must match `aerospace_payload` `wasm_import_module`).
pub const HOST_IMPORT_MODULE: &str = "aether";

/// Secure host syscall surface for the WASM guest (`import "aether" ...`).
pub struct HostCalls;

impl HostCalls {
    /// `read_atmospheric_pressure() -> f32` (Wasm result type `f32` / `0x7D`).
    fn read_atmospheric_pressure(mut caller: Caller<'_, HostState>) -> F32 {
        let pressure = mmio::read_atmospheric_pressure();
        caller.data_mut().last_pressure = pressure;
        F32::from_bits(pressure.to_bits())
    }

    /// `read_radiation_dosimeter() -> i32` (Wasm has no `u32`; uses `i32` / `0x7F`).
    fn read_radiation_dosimeter(mut caller: Caller<'_, HostState>) -> i32 {
        let dose = mmio::read_radiation_dosimeter();
        caller.data_mut().last_dose = dose;
        dose as i32
    }

    /// `commit_uplink(proof_lo: i32, proof_hi: i32)` (legacy 64-bit proof commit).
    fn commit_uplink(caller: Caller<'_, HostState>, proof_lo: i32, proof_hi: i32) {
        if validate_guest_access(&caller).is_ok() {
            let _ = mmio::commit_proof(proof_lo as u32, proof_hi as u32);
        }
    }

    /// `commit_telemetry_vector(ptr: i32, len: i32)`
    fn commit_telemetry_vector(caller: Caller<'_, HostState>, ptr: i32, len: i32) {
        if len <= 0 {
            return;
        }
        let len = len as usize;
        if len > mmio::TELEMETRY_VECTOR_CAP {
            return;
        }
        let Some(slice) = guest_memory_slice(&caller, ptr, len) else {
            return;
        };
        let _ = mmio::commit_telemetry_vector(slice);
    }
}

/// Classify a wasmi failure on COM1 without formatting the error (no alloc).
fn log_wasmi_error(e: &Error) {
    match e {
        Error::Linker(_) => {
            serial_println!("[AETHER] ERR: Linker (Import Mismatch)");
        }
        Error::Instantiation(_) => {
            serial_println!("[AETHER] ERR: Instantiation");
        }
        Error::Trap(_) => {
            serial_println!("[AETHER] ERR: Trap");
        }
        _ => {
            serial_println!("[AETHER] ERR: Unknown");
        }
    }
}

/// Register the full `aether` host API on the linker (signatures must match guest imports).
fn link_aether_host(linker: &mut Linker<HostState>) -> Result<(), Error> {
    linker
        .func_wrap(
            HOST_IMPORT_MODULE,
            "read_atmospheric_pressure",
            HostCalls::read_atmospheric_pressure,
        )
        .map_err(Error::from)?;
    linker
        .func_wrap(
            HOST_IMPORT_MODULE,
            "read_radiation_dosimeter",
            HostCalls::read_radiation_dosimeter,
        )
        .map_err(Error::from)?;
    linker
        .func_wrap(
            HOST_IMPORT_MODULE,
            "commit_telemetry_vector",
            HostCalls::commit_telemetry_vector,
        )
        .map_err(Error::from)?;
    linker
        .func_wrap(HOST_IMPORT_MODULE, "commit_uplink", HostCalls::commit_uplink)
        .map_err(Error::from)?;
    Ok(())
}

/// ISR / bootstrap entry — full micro-cycle without scheduler involvement.
pub fn sovereign_bootstrap(trigger: Option<HardwareInterrupt>) {
    memory::reset_arena();

    let mut host = match AetherHost::instantiate(trigger) {
        Ok(h) => h,
        Err(e) => {
            log_wasmi_error(&e);
            fault_shutdown(trigger, -1);
            return;
        }
    };

    let guest_result = match host.run_diagnostic() {
        Ok(v) => v,
        Err(e) => {
            log_wasmi_error(&e);
            fault_shutdown(trigger, -1);
            return;
        }
    };

    let proof = host.commit_outcome(guest_result);

    shutdown::self_annihilate(ShutdownReport {
        guest_result,
        proof,
        vector: trigger.map(|t| t as u8).unwrap_or(interrupts::last_vector()),
    });
}

fn fault_shutdown(trigger: Option<HardwareInterrupt>, guest_result: i32) {
    serial_println!("[AETHER] FATAL: Entering fault_shutdown handler");
    shutdown::self_annihilate(ShutdownReport {
        guest_result,
        proof: 0,
        vector: trigger.map(|t| t as u8).unwrap_or(0),
    });
}

fn validate_guest_access(caller: &Caller<'_, HostState>) -> Result<(), MemoryFault> {
    let sandbox = SandboxRegion::get();
    if let Some(mem) = caller.get_export("memory").and_then(Extern::into_memory) {
        let len = mem.data(caller).len();
        if len > sandbox.len() {
            return Err(MemoryFault::SandboxOverflow);
        }
    }
    Ok(())
}

/// Bounds-checked view into guest linear memory for MMIO ingest syscalls.
fn guest_memory_slice<'a>(
    caller: &'a Caller<'_, HostState>,
    ptr: i32,
    len: usize,
) -> Option<&'a [u8]> {
    if ptr < 0 {
        return None;
    }
    let ptr = ptr as usize;
    let mem = caller.get_export("memory").and_then(Extern::into_memory)?;
    let data = mem.data(caller);
    let end = ptr.checked_add(len)?;
    if end > data.len() {
        return None;
    }
    Some(&data[ptr..end])
}

/// Embedded WASM host: engine, store, instance, typed guest entry.
pub struct AetherHost {
    store: Store<HostState>,
    diagnostic: TypedFunc<(), i32>,
}

impl AetherHost {
    /// Parse module, wire host imports, optionally cap guest memory.
    pub fn instantiate(trigger: Option<HardwareInterrupt>) -> Result<Self, Error> {
        let mut config = Config::default();
        config.consume_fuel(false);
        let engine = Engine::new(&config);

        let mut store = Store::new(
            &engine,
            HostState {
                trigger,
                last_sensor: 0,
                last_pressure: 0.0,
                last_dose: 0,
                guest_result: 0,
            },
        );

        let module = Module::new(&engine, wasm_payload::WASM_BYTES)?;

        let mut linker = Linker::new(&engine);
        link_aether_host(&mut linker)?;

        let instance_pre = linker.instantiate(&mut store, &module)?;
        let instance = instance_pre.ensure_no_start(&mut store)?;

        cap_guest_memory(&mut store, &instance)?;

        // Primary guest entry is `evaluate_limits` (`#[no_mangle]` in aerospace_payload);
        // `diagnostic` is a thin alias that forwards to the same logic.
        let entry = instance
            .get_typed_func::<(), i32>(&store, "evaluate_limits")
            .or_else(|_| instance.get_typed_func::<(), i32>(&store, "diagnostic"))?;

        Ok(Self {
            store,
            diagnostic: entry,
        })
    }

    /// Call exported `evaluate_limits` / `diagnostic` (must return `i32` status flags).
    pub fn run_diagnostic(&mut self) -> Result<i32, Error> {
        let result = self.diagnostic.call(&mut self.store, ())?;
        self.store.data_mut().guest_result = result;
        Ok(result)
    }

    /// Derive proof digest and commit to uplink MMIO.
    pub fn commit_outcome(&self, guest_result: i32) -> u64 {
        let state = self.store.data();
        let proof_lo = (guest_result as u32) ^ state.last_dose ^ state.last_sensor;
        let proof_hi = state.last_dose.rotate_left(9)
            ^ f32::to_bits(state.last_pressure)
            ^ 0xA17E_0001;
        mmio::commit_proof(proof_lo, proof_hi)
    }
}

fn cap_guest_memory(store: &mut Store<HostState>, instance: &Instance) -> Result<(), Error> {
    if let Some(mem) = instance
        .get_export(&mut *store, "memory")
        .and_then(Extern::into_memory)
    {
        let pages = mem.current_pages(&*store);
        let page_count = u32::from(pages) as usize;
        let guest_bytes = page_count.checked_mul(WASM_PAGE_SIZE).ok_or_else(|| {
            serial_println!("[AETHER] WASM TRAP (memory): guest page count overflow");
            Error::from(MemoryError::OutOfBoundsAccess)
        })?;

        // Cross-check: wasmi's byte slice must match pages × 64 KiB.
        let data_len = mem.data(&mut *store).len();
        if data_len != guest_bytes {
            serial_println!(
                "[AETHER] WASM TRAP (memory): page/byte mismatch (data={} pages×{}={})",
                data_len,
                page_count,
                guest_bytes
            );
            return Err(Error::from(MemoryError::OutOfBoundsAccess));
        }

        if guest_bytes > SANDBOX_MEMORY_SIZE {
            serial_println!(
                "[AETHER] WASM TRAP (memory): guest {} bytes exceeds sandbox {} bytes",
                guest_bytes,
                SANDBOX_MEMORY_SIZE
            );
            return Err(Error::from(MemoryError::OutOfBoundsAccess));
        }
        // Do not overwrite guest linear memory here — the module loader has already
        // initialized data/bss (including static telemetry). Zeroing caused traps and
        // `guest=-1` via the fault path even when instantiation succeeded.
    } else {
        Memory::new(
            store,
            MemoryType::new(1, Some(1)).map_err(|_| Error::from(MemoryError::InvalidMemoryType))?,
        )
        .map_err(|_| Error::from(MemoryError::OutOfBoundsAllocation))?;
    }
    Ok(())
}
