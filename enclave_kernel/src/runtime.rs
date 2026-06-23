//! wasmi host — loads embedded WASM, runs `evaluate_limits`, commits chained proof.

use wasmi::{
    Caller, Config, Engine, Error, Extern, Instance, Linker, Memory, MemoryType, Module, Store,
    TypedFunc,
};
use wasmi::errors::MemoryError;
use wasmi_core::F32;

use crate::interrupts::{self, HardwareInterrupt};
use crate::memory::{self, SANDBOX_MEMORY_SIZE, WASM_PAGE_SIZE};
use crate::mmio;
use crate::proof;
use crate::serial_println;
use crate::shutdown::{self, ShutdownReport};
use crate::wasm_payload;

pub struct HostState {
    pub last_pressure: f32,
    pub last_dose: u32,
    pub guest_result: i32,
}

pub const HOST_IMPORT_MODULE: &str = "aether";

pub struct HostCalls;

impl HostCalls {
    fn read_atmospheric_pressure(mut caller: Caller<'_, HostState>) -> F32 {
        let pressure = mmio::read_atmospheric_pressure();
        caller.data_mut().last_pressure = pressure;
        F32::from_bits(pressure.to_bits())
    }

    fn read_radiation_dosimeter(mut caller: Caller<'_, HostState>) -> i32 {
        let dose = mmio::read_radiation_dosimeter();
        caller.data_mut().last_dose = dose;
        dose as i32
    }

    fn read_pressure_limit(_caller: Caller<'_, HostState>) -> F32 {
        #[cfg(target_arch = "riscv32")]
        let limit = crate::platform::mission_profile::pressure_limit_atm();
        #[cfg(not(target_arch = "riscv32"))]
        let limit = 0.15f32;
        F32::from_bits(limit.to_bits())
    }

    fn read_dose_limit(_caller: Caller<'_, HostState>) -> i32 {
        #[cfg(target_arch = "riscv32")]
        let limit = crate::platform::mission_profile::dose_limit();
        #[cfg(not(target_arch = "riscv32"))]
        let limit = 1_000u32;
        limit as i32
    }

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
            "read_pressure_limit",
            HostCalls::read_pressure_limit,
        )
        .map_err(Error::from)?;
    linker
        .func_wrap(
            HOST_IMPORT_MODULE,
            "read_dose_limit",
            HostCalls::read_dose_limit,
        )
        .map_err(Error::from)?;
    linker
        .func_wrap(
            HOST_IMPORT_MODULE,
            "commit_telemetry_vector",
            HostCalls::commit_telemetry_vector,
        )
        .map_err(Error::from)?;
    Ok(())
}

pub fn run_mission_cycle(trigger: Option<HardwareInterrupt>) {
    #[cfg(target_arch = "riscv32")]
    {
        crate::platform::esp32c6::feed_watchdog();
        crate::platform::esp32c6::status_led_on();
        crate::platform::power_log::mark_cycle_start();
    }

    memory::reset_arena();

    let vector = trigger.map(|t| t as u8).unwrap_or(interrupts::last_vector());

    let mut host = match AetherHost::instantiate() {
        Ok(h) => h,
        Err(e) => {
            log_wasmi_error(&e);
            fault_shutdown(trigger, -1);
            return;
        }
    };

    let guest_result = match host.run_evaluate_limits() {
        Ok(v) => v,
        Err(e) => {
            log_wasmi_error(&e);
            fault_shutdown(trigger, -1);
            return;
        }
    };

    let proof = host.commit_outcome(guest_result, vector);

    shutdown::finish_cycle(ShutdownReport {
        guest_result,
        proof,
        vector,
    });
}

pub fn sovereign_bootstrap(trigger: Option<HardwareInterrupt>) -> ! {
    run_mission_cycle(trigger);
    shutdown::enter_absolute_halt();
}

fn fault_shutdown(trigger: Option<HardwareInterrupt>, guest_result: i32) {
    serial_println!("[AETHER] FATAL: fault shutdown");
    shutdown::self_annihilate(ShutdownReport {
        guest_result,
        proof: 0,
        vector: trigger.map(|t| t as u8).unwrap_or(0),
    });
}

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

pub struct AetherHost {
    store: Store<HostState>,
    evaluate_limits: TypedFunc<(), i32>,
}

impl AetherHost {
    pub fn instantiate() -> Result<Self, Error> {
        let mut config = Config::default();
        config.consume_fuel(false);
        let engine = Engine::new(&config);

        let mut store = Store::new(
            &engine,
            HostState {
                last_pressure: 0.0,
                last_dose: 0,
                guest_result: 0,
            },
        );

        #[cfg(target_arch = "riscv32")]
        let wasm_bytes = wasm_payload::wasm_bytes_for_slot(crate::platform::mission_profile::payload_slot());
        #[cfg(not(target_arch = "riscv32"))]
        let wasm_bytes = wasm_payload::wasm_bytes_for_slot(0);

        let module = Module::new(&engine, wasm_bytes)?;

        #[cfg(target_arch = "riscv32")]
        crate::platform::esp32c6::feed_watchdog();

        let mut linker = Linker::new(&engine);
        link_aether_host(&mut linker)?;

        let instance_pre = linker.instantiate(&mut store, &module)?;

        #[cfg(target_arch = "riscv32")]
        crate::platform::esp32c6::feed_watchdog();

        let instance = instance_pre.ensure_no_start(&mut store)?;
        cap_guest_memory(&mut store, &instance)?;

        let evaluate_limits = instance.get_typed_func::<(), i32>(&store, "evaluate_limits")?;

        Ok(Self {
            store,
            evaluate_limits,
        })
    }

    pub fn run_evaluate_limits(&mut self) -> Result<i32, Error> {
        let result = self.evaluate_limits.call(&mut self.store, ())?;
        self.store.data_mut().guest_result = result;
        Ok(result)
    }

    pub fn commit_outcome(&self, guest_result: i32, vector: u8) -> u64 {
        let state = self.store.data();
        let pressure_bits = state.last_pressure.to_bits();

        #[cfg(target_arch = "riscv32")]
        {
            let prev = crate::platform::rtc_state::last_proof();
            let cycle = crate::platform::rtc_state::cycle_count().saturating_add(1);
            let mission_id = crate::platform::mission_profile::mission_id();
            let slot = crate::platform::mission_profile::payload_slot();
            let proof = proof::chain_proof(
                prev,
                guest_result,
                pressure_bits,
                state.last_dose,
                vector,
                cycle,
                mission_id,
                slot,
            );
            return mmio::commit_proof(proof as u32, (proof >> 32) as u32);
        }

        #[cfg(not(target_arch = "riscv32"))]
        {
            let proof_lo = (guest_result as u32) ^ state.last_dose;
            let proof_hi = state.last_dose.rotate_left(9)
                ^ pressure_bits
                ^ 0xA17E_0001;
            mmio::commit_proof(proof_lo, proof_hi)
        }
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
                "[AETHER] WASM TRAP (memory): guest {} bytes exceeds cap {} bytes",
                guest_bytes,
                SANDBOX_MEMORY_SIZE
            );
            return Err(Error::from(MemoryError::OutOfBoundsAccess));
        }
    } else {
        Memory::new(
            store,
            MemoryType::new(1, Some(1)).map_err(|_| Error::from(MemoryError::InvalidMemoryType))?,
        )
        .map_err(|_| Error::from(MemoryError::OutOfBoundsAllocation))?;
    }
    Ok(())
}
