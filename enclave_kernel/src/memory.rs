//! Static memory isolation for the WASM sandbox and global allocator.
//!
//! ## Layout
//! - [`SANDBOX_MEMORY`] — guest-linear memory backing store (up to [`SANDBOX_MEMORY_SIZE`]).
//! - [`ARENA`] — separate bump arena for host/runtime allocations (wasmi engine, module compile, etc.).
//! - [`ISR_STACK`] — 4 KiB dedicated stack for interrupt service (isolated from main).
//!
//! All backing stores sit behind [`spin::Mutex`] guards so the modern toolchain never
//! forms shared references to `static mut`. Buffer addresses are link-time stable.

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::NonNull;
use core::sync::atomic::{AtomicBool, Ordering};
use spin::Mutex;

/// WebAssembly linear memory page size (64 KiB per Wasm spec).
pub const WASM_PAGE_SIZE: usize = 64 * 1024;

/// Guest linear memory cap — must cover Rust `wasm32` static data, stack, and heap.
pub const SANDBOX_MEMORY_SIZE: usize = 64 * 1024;

/// Host/runtime bump arena — global heap for `wasmi` and `alloc` (disjoint from guest sandbox).
pub const ARENA_SIZE: usize = 128 * 1024;

/// Alias for the host heap size (`ARENA_SIZE`) — wasmi needs multi-MiB headroom to instantiate.
pub const HEAP_SIZE: usize = ARENA_SIZE;

/// ISR stack size — must satisfy worst-case sovereign bootstrap + interpreter depth.
pub const ISR_STACK_SIZE: usize = 4 * 1024;

/// 16-byte aligned static blob.
#[repr(C, align(16))]
struct AlignedBytes<const N: usize>([u8; N]);

/// Bump arena buffer plus allocation cursor (single lock domain).
struct ArenaState {
    bytes: AlignedBytes<ARENA_SIZE>,
    cursor: usize,
}

/// Guest linear memory backing (WASM address space index 0).
static SANDBOX_MEMORY: Mutex<AlignedBytes<SANDBOX_MEMORY_SIZE>> =
    Mutex::new(AlignedBytes([0; SANDBOX_MEMORY_SIZE]));

/// Host allocation arena (never shared with guest mappings).
static ARENA: Mutex<ArenaState> = Mutex::new(ArenaState {
    bytes: AlignedBytes([0; ARENA_SIZE]),
    cursor: 0,
});

/// Interrupt-service stack (grows downward; SP initialized to top).
static ISR_STACK: Mutex<AlignedBytes<ISR_STACK_SIZE>> =
    Mutex::new(AlignedBytes([0; ISR_STACK_SIZE]));

static SANDBOX_SEALED: AtomicBool = AtomicBool::new(false);

/// Memory protection fault taxonomy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryFault {
    /// Guest pointer + length overflows sandbox.
    SandboxOverflow,
    /// Guest attempted to touch host-only addresses through validation layer.
    HostEscape,
    /// Allocator arena exhausted.
    ArenaExhausted,
    /// Sandbox already zeroed / sealed after shutdown.
    SandboxSealed,
}

/// Immutable view of the sandbox for host-side validation.
///
/// Holds a raw base pointer into the mutex-backed static buffer. The address remains
/// valid for the program lifetime because the storage is never moved.
pub struct SandboxRegion {
    base: *const u8,
    size: usize,
}

impl SandboxRegion {
    /// Obtain the static sandbox region descriptor.
    #[inline]
    pub fn get() -> Self {
        let guard = SANDBOX_MEMORY.lock();
        let base = guard.0.as_ptr();
        drop(guard);
        Self {
            base,
            size: SANDBOX_MEMORY_SIZE,
        }
    }

    /// Base pointer for mapping into guest linear memory.
    #[inline]
    pub fn base_mut_ptr(&self) -> *mut u8 {
        self.base as *mut u8
    }

    /// Validate a guest-relative offset/length pair before host memcpy or MMIO proxy.
    pub fn check_guest_slice(&self, offset: u32, len: usize) -> Result<(), MemoryFault> {
        if SANDBOX_SEALED.load(Ordering::Acquire) {
            return Err(MemoryFault::SandboxSealed);
        }
        let off = offset as usize;
        let end = off.checked_add(len).ok_or(MemoryFault::SandboxOverflow)?;
        if end > self.size {
            return Err(MemoryFault::SandboxOverflow);
        }
        Ok(())
    }

    /// Return a guest slice after validation.
    ///
    /// # Safety
    /// Caller must respect WASM single-threaded aliasing rules and hold the sandbox lock
    /// if another core could contend (unikernel is single-core; ISR masks IRQs).
    pub unsafe fn guest_slice(&self, offset: u32, len: usize) -> Result<&[u8], MemoryFault> {
        self.check_guest_slice(offset, len)?;
        // SAFETY: Bounds checked via `check_guest_slice`; base points at static sandbox storage.
        Ok(unsafe { core::slice::from_raw_parts(self.base.add(offset as usize), len) })
    }

    /// Return a mutable guest slice after validation.
    ///
    /// # Safety
    /// Only one WASM store mutator may exist at a time (enforced by ISR interrupt masking).
    pub unsafe fn guest_slice_mut(
        &self,
        offset: u32,
        len: usize,
    ) -> Result<&mut [u8], MemoryFault> {
        self.check_guest_slice(offset, len)?;
        Ok(unsafe {
            core::slice::from_raw_parts_mut(self.base.add(offset as usize) as *mut u8, len)
        })
    }

    /// Sandbox byte length.
    #[inline]
    pub fn len(&self) -> usize {
        self.size
    }

    /// Reject pointers that refer to host structures outside the sandbox mapping.
    pub fn reject_host_escape(&self, ptr: usize) -> Result<(), MemoryFault> {
        let base = self.base as usize;
        let end = base + self.size;
        if ptr >= base && ptr < end {
            Ok(())
        } else if ptr >= base {
            Err(MemoryFault::SandboxOverflow)
        } else {
            Err(MemoryFault::HostEscape)
        }
    }
}

/// Zero the sandbox and seal it against further guest access (post-run annihilation).
pub fn annihilate_sandbox() {
    let mut sandbox = SANDBOX_MEMORY.lock();
    secure_zero(&mut sandbox.0);
    drop(sandbox);
    SANDBOX_SEALED.store(true, Ordering::Release);
}

/// Reset arena cursor for a fresh boot cycle (called once per wake from dormancy).
pub fn reset_arena() {
    let mut arena = ARENA.lock();
    arena.cursor = 0;
    secure_zero(&mut arena.bytes.0);
    drop(arena);
    SANDBOX_SEALED.store(false, Ordering::Release);
    let mut sandbox = SANDBOX_MEMORY.lock();
    secure_zero(&mut sandbox.0);
}

/// Top-of-stack address for ISR entry (x86 grows down).
pub fn isr_stack_top() -> usize {
    let guard = ISR_STACK.lock();
    guard.0.as_ptr().wrapping_add(ISR_STACK_SIZE) as usize
}

/// Bump allocator backing `wasmi` and transient host structures.
pub struct ArenaAllocator;

unsafe impl GlobalAlloc for ArenaAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let align = layout.align();
        let size = layout.size();

        let mut arena = ARENA.lock();
        let aligned = (arena.cursor + align - 1) & !(align - 1);
        let new_cursor = match aligned.checked_add(size) {
            Some(n) => n,
            None => return core::ptr::null_mut(),
        };
        if new_cursor > ARENA_SIZE {
            return core::ptr::null_mut();
        }
        let ptr = arena.bytes.0.as_mut_ptr().wrapping_add(aligned);
        arena.cursor = new_cursor;
        ptr
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Intentional no-op: bump allocator frees only on `reset_arena`.
    }
}

/// Global allocator registration for the `alloc` crate.
#[global_allocator]
static GLOBAL: ArenaAllocator = ArenaAllocator;

/// Chunked zero-fill; feeds the hardware watchdog on ESP32 during long scrubs.
fn secure_zero(bytes: &mut [u8]) {
    const CHUNK: usize = 4096;
    for chunk in bytes.chunks_mut(CHUNK) {
        #[cfg(target_arch = "riscv32")]
        crate::platform::esp32c3::feed_watchdog();
        chunk.fill(0);
    }
}

/// Non-null pointer helper used by runtime when wiring sandbox pages.
pub fn sandbox_non_null() -> Option<NonNull<u8>> {
    NonNull::new(SandboxRegion::get().base_mut_ptr())
}
