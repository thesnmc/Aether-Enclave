//! Embedded WebAssembly host (`AetherHost`) — wasmi on a static bump arena.
//!
//! Guest modules import [`HostCalls`] from the `"aether"` module. Linear memory, when
//! present, is bounded by [`crate::memory::SANDBOX_MEMORY_SIZE`].

use wasmi::{Caller, Config, Engine, Extern, Func, Instance, Linker, Memory, MemoryType, Module, Store, TypedFunc};
use wasmi_core::F32;

use crate::interrupts::{self, HardwareInterrupt};
use crate::memory::{self, MemoryFault, SandboxRegion};
use crate::mmio;
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

/// Register the full `aether` host API on the linker.
fn link_aether_host(
    linker: &mut Linker<HostState>,
    store: &mut Store<HostState>,
) -> Result<(), HostError> {
    linker
        .define(
            HOST_IMPORT_MODULE,
            "read_atmospheric_pressure",
            Func::wrap(&mut *store, HostCalls::read_atmospheric_pressure),
        )
        .map_err(|_| HostError::Linker)?;
    linker
        .define(
            HOST_IMPORT_MODULE,
            "read_radiation_dosimeter",
            Func::wrap(&mut *store, HostCalls::read_radiation_dosimeter),
        )
        .map_err(|_| HostError::Linker)?;
    linker
        .define(
            HOST_IMPORT_MODULE,
            "commit_telemetry_vector",
            Func::wrap(&mut *store, HostCalls::commit_telemetry_vector),
        )
        .map_err(|_| HostError::Linker)?;
    linker
        .define(
            HOST_IMPORT_MODULE,
            "commit_uplink",
            Func::wrap(&mut *store, HostCalls::commit_uplink),
        )
        .map_err(|_| HostError::Linker)?;
    Ok(())
}

/// ISR / bootstrap entry — full micro-cycle without scheduler involvement.
pub fn sovereign_bootstrap(trigger: Option<HardwareInterrupt>) {
    memory::reset_arena();

    let mut host = match AetherHost::instantiate(trigger) {
        Ok(h) => h,
        Err(_) => {
            fault_shutdown(trigger);
            return;
        }
    };

    let guest_result = match host.run_diagnostic() {
        Ok(v) => v,
        Err(_) => {
            fault_shutdown(trigger);
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

fn fault_shutdown(trigger: Option<HardwareInterrupt>) {
    shutdown::self_annihilate(ShutdownReport {
        guest_result: 0,
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
    pub fn instantiate(trigger: Option<HardwareInterrupt>) -> Result<Self, HostError> {
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

        let module = Module::new(&engine, wasm_payload::WASM_BYTES).map_err(|_| HostError::ModuleParse)?;

        let mut linker = Linker::new(&engine);
        link_aether_host(&mut linker, &mut store)?;

        let instance_pre = linker
            .instantiate(&mut store, &module)
            .map_err(|_| HostError::Instantiate)?;
        let instance = instance_pre
            .ensure_no_start(&mut store)
            .map_err(|_| HostError::Instantiate)?;

        cap_guest_memory(&mut store, &instance)?;

        let diagnostic = instance
            .get_typed_func::<(), i32>(&store, "diagnostic")
            .or_else(|_| instance.get_typed_func::<(), i32>(&store, "evaluate_limits"))
            .map_err(|_| HostError::ExportMissing)?;

        Ok(Self { store, diagnostic })
    }

    /// Call exported guest entry (`diagnostic` or `evaluate_limits`).
    pub fn run_diagnostic(&mut self) -> Result<i32, HostError> {
        let result = self
            .diagnostic
            .call(&mut self.store, ())
            .map_err(|_| HostError::Trap)?;
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

fn cap_guest_memory(store: &mut Store<HostState>, instance: &Instance) -> Result<(), HostError> {
    if let Some(mem) = instance
        .get_export(&mut *store, "memory")
        .and_then(Extern::into_memory)
    {
        let sandbox = SandboxRegion::get();
        let len = mem.data(&mut *store).len();
        if len > sandbox.len() {
            return Err(HostError::SandboxBounds);
        }
        let dest = mem.data_mut(&mut *store);
        // SAFETY: Length checked `<= SANDBOX_MEMORY_SIZE`; disjoint from ISR stack.
        unsafe {
            core::ptr::copy_nonoverlapping(sandbox.base_mut_ptr(), dest.as_mut_ptr(), dest.len());
        }
    } else {
        let _ = Memory::new(
            store,
            MemoryType::new(1, Some(1)).map_err(|_| HostError::MemoryType)?,
        )
        .map_err(|_| HostError::MemoryAllocation)?;
    }
    Ok(())
}

/// Host error taxonomy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostError {
    /// Invalid WASM memory type configuration.
    MemoryType,
    /// Host arena could not satisfy WASM memory allocation.
    MemoryAllocation,
    /// Embedded WASM bytes failed validation.
    ModuleParse,
    /// Host import wiring failed.
    Linker,
    /// Module instantiation failed.
    Instantiate,
    /// Required guest export not found.
    ExportMissing,
    /// Trap during guest execution.
    Trap,
    /// Guest linear memory exceeds sandbox cap.
    SandboxBounds,
}
