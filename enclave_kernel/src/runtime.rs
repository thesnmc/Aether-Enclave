//! Embedded WebAssembly host (`AetherHost`) — wasmi on a static bump arena.
//!
//! Guest modules import [`HostCalls`] from the `"aether"` module. Linear memory, when
//! present, is bounded by [`crate::memory::SANDBOX_MEMORY_SIZE`].

use wasmi::{Caller, Config, Engine, Extern, Func, Instance, Linker, Memory, MemoryType, Module, Store, TypedFunc};

use crate::interrupts::{self, HardwareInterrupt};
use crate::memory::{self, MemoryFault, SandboxRegion};
use crate::mmio;
use crate::shutdown::{self, ShutdownReport};
use crate::wasm_payload;

/// Host-side execution context for one wake cycle.
pub struct HostState {
    /// Physical trigger that launched this cycle.
    pub trigger: Option<HardwareInterrupt>,
    /// Last sensor reading (proof chaining).
    pub last_sensor: u32,
    /// Cached guest return value.
    pub guest_result: i32,
}

/// Secure host syscall surface for the WASM guest (`import "aether" ...`).
pub struct HostCalls;

impl HostCalls {
    /// `read_sensor(channel: i32) -> i32`
    fn read_sensor(mut caller: Caller<'_, HostState>, channel: i32) -> i32 {
        if validate_guest_access(&caller).is_err() {
            return 0;
        }
        let state = caller.data_mut();
        let value = match channel {
            0 => mmio::read_atomic_o2(),
            1 => mmio::read_kinetic_joint(),
            _ => match state.trigger {
                Some(HardwareInterrupt::KineticJointActuation) => mmio::read_kinetic_joint(),
                _ => mmio::read_atomic_o2(),
            },
        };
        state.last_sensor = value;
        value as i32
    }

    /// `commit_uplink(proof_lo: i32, proof_hi: i32)`
    fn commit_uplink(caller: Caller<'_, HostState>, proof_lo: i32, proof_hi: i32) {
        if validate_guest_access(&caller).is_ok() {
            let _ = mmio::commit_proof(proof_lo as u32, proof_hi as u32);
        }
    }
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

/// Embedded WASM host: engine, store, instance, typed `diagnostic` export.
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
                guest_result: 0,
            },
        );

        let module = Module::new(&engine, wasm_payload::WASM_BYTES).map_err(|_| HostError::ModuleParse)?;

        let mut linker = Linker::new(&engine);
        linker
            .define(
                "aether",
                "read_sensor",
                Func::wrap(&mut store, HostCalls::read_sensor),
            )
            .map_err(|_| HostError::Linker)?;
        linker
            .define(
                "aether",
                "commit_uplink",
                Func::wrap(&mut store, HostCalls::commit_uplink),
            )
            .map_err(|_| HostError::Linker)?;

        let instance_pre = linker
            .instantiate(&mut store, &module)
            .map_err(|_| HostError::Instantiate)?;
        let instance = instance_pre
            .ensure_no_start(&mut store)
            .map_err(|_| HostError::Instantiate)?;

        cap_guest_memory(&mut store, &instance)?;

        let diagnostic = instance
            .get_typed_func::<(), i32>(&store, "diagnostic")
            .map_err(|_| HostError::ExportMissing)?;

        Ok(Self { store, diagnostic })
    }

    /// Call exported `diagnostic`.
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
        let sensor = self.store.data().last_sensor;
        let proof_lo = (guest_result as u32) ^ sensor;
        let proof_hi = sensor.rotate_left(13) ^ 0xA17E_0001;
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
        // Mirror static sandbox page into guest memory for deterministic scrubbing.
        let dest = mem.data_mut(&mut *store);
        // SAFETY: Length checked `<= SANDBOX_MEMORY_SIZE`; disjoint from ISR stack.
        unsafe {
            core::ptr::copy_nonoverlapping(sandbox.base_mut_ptr(), dest.as_mut_ptr(), dest.len());
        }
    } else {
        // Provide an implicit single-page memory for modules without a memory section.
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
    MemoryType,
    MemoryAllocation,
    ModuleParse,
    Linker,
    Instantiate,
    ExportMissing,
    Trap,
    SandboxBounds,
}
