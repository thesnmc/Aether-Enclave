//! Memory-mapped I/O register map for sensor ingress and uplink commit.
//!
//! Addresses are placeholders for simulation; on flight hardware these must match
//! the board BSP and remain naturally aligned for the target bus width.

use core::sync::atomic::{AtomicU32, Ordering};

/// Simulated atomic-oxygen density sensor (raw ADC counts).
pub const REG_ATOMIC_O2_SENSOR: usize = 0xFEF0_0000;

/// Simulated kinetic joint strain gauge.
pub const REG_KINETIC_JOINT: usize = 0xFEF0_0004;

/// Uplink commit: low word = proof digest, high word = sequence / status flags.
pub const REG_UPLINK_COMMIT_LO: usize = 0xFEF0_0010;
pub const REG_UPLINK_COMMIT_HI: usize = 0xFEF0_0014;

/// Power-management unit: write `CMD_DORMANT` to enter hard sleep.
pub const REG_PMU_COMMAND: usize = 0xFEF0_0020;
pub const PMU_CMD_DORMANT: u32 = 0x0000_0001;
pub const PMU_CMD_HARD_RESET: u32 = 0xDEAD_0002;

// ---------------------------------------------------------------------------
// Simulation backing store (no real MMIO on hosted/bare-metal bring-up without FPGA)
// ---------------------------------------------------------------------------

static SIM_O2: AtomicU32 = AtomicU32::new(0x0000_4000);
static SIM_KINETIC: AtomicU32 = AtomicU32::new(0);
static SIM_COMMIT_LO: AtomicU32 = AtomicU32::new(0);
static SIM_COMMIT_HI: AtomicU32 = AtomicU32::new(0);
static SIM_PMU: AtomicU32 = AtomicU32::new(0);

/// Inject a simulated atmospheric trigger (test / pre-flight harness).
pub fn sim_inject_o2_drop() {
    SIM_O2.store(0x0000_0100, Ordering::Release);
}

/// Inject a simulated kinetic joint trigger.
pub fn sim_inject_kinetic_pulse() {
    SIM_KINETIC.store(0x0000_8000, Ordering::Release);
}

/// Read the atomic oxygen sensor via MMIO (simulation uses atomic backing).
#[inline]
pub fn read_atomic_o2() -> u32 {
    // SAFETY: Simulation maps all MMIO to atomics; flight hardware guarantees
    // 32-bit aligned device registers at REG_ATOMIC_O2_SENSOR.
    unsafe { read_volatile_u32(REG_ATOMIC_O2_SENSOR) }
}

/// Read kinetic joint sensor.
#[inline]
pub fn read_kinetic_joint() -> u32 {
    unsafe { read_volatile_u32(REG_KINETIC_JOINT) }
}

/// Commit proof words to uplink MMIO and return the combined 64-bit digest view.
pub fn commit_proof(proof_lo: u32, proof_hi: u32) -> u64 {
    unsafe {
        write_volatile_u32(REG_UPLINK_COMMIT_LO, proof_lo);
        write_volatile_u32(REG_UPLINK_COMMIT_HI, proof_hi);
    }
    (u64::from(proof_hi) << 32) | u64::from(proof_lo)
}

/// Last committed proof (simulation readback).
pub fn last_committed_proof() -> u64 {
    let lo = SIM_COMMIT_LO.load(Ordering::Acquire);
    let hi = SIM_COMMIT_HI.load(Ordering::Acquire);
    (u64::from(hi) << 32) | u64::from(lo)
}

/// Issue PMU dormancy command.
pub fn request_dormancy() {
    unsafe {
        write_volatile_u32(REG_PMU_COMMAND, PMU_CMD_DORMANT);
    }
}

/// Issue hard reset (self-annihilation path).
pub fn request_hard_reset() {
    unsafe {
        write_volatile_u32(REG_PMU_COMMAND, PMU_CMD_HARD_RESET);
    }
}

#[inline]
unsafe fn read_volatile_u32(addr: usize) -> u32 {
    match addr {
        REG_ATOMIC_O2_SENSOR => SIM_O2.load(Ordering::Acquire),
        REG_KINETIC_JOINT => SIM_KINETIC.load(Ordering::Acquire),
        REG_UPLINK_COMMIT_LO => SIM_COMMIT_LO.load(Ordering::Acquire),
        REG_UPLINK_COMMIT_HI => SIM_COMMIT_HI.load(Ordering::Acquire),
        REG_PMU_COMMAND => SIM_PMU.load(Ordering::Acquire),
        _ => {
            // SAFETY: Unmapped MMIO reads return zero per bus fault policy (simulation).
            unsafe { core::ptr::read_volatile(addr as *const u32) }
        }
    }
}

#[inline]
unsafe fn write_volatile_u32(addr: usize, value: u32) {
    match addr {
        REG_ATOMIC_O2_SENSOR => SIM_O2.store(value, Ordering::Release),
        REG_KINETIC_JOINT => SIM_KINETIC.store(value, Ordering::Release),
        REG_UPLINK_COMMIT_LO => SIM_COMMIT_LO.store(value, Ordering::Release),
        REG_UPLINK_COMMIT_HI => SIM_COMMIT_HI.store(value, Ordering::Release),
        REG_PMU_COMMAND => SIM_PMU.store(value, Ordering::Release),
        _ => {
            unsafe { core::ptr::write_volatile(addr as *mut u32, value) };
        }
    }
}
