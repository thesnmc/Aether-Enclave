//! Static memory isolation for the WASM sandbox and global allocator.
//!
//! ## Layout
//! - [`SANDBOX_MEMORY`] — 64 KiB guest-linear memory backing store (WASM page).
//! - [`ARENA`] — separate bump arena for host/runtime allocations (wasmi engine, etc.).
//! - [`ISR_STACK`] — 4 KiB dedicated stack for interrupt service (isolated from main).
//!
//! No heap fragmentation: all sizes are compile-time constants.

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::NonNull;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use spin::Mutex;

/// WASM linear memory page size (64 KiB) — single static sandbox page.
pub const SANDBOX_MEMORY_SIZE: usize = 64 * 1024;

/// Host/runtime bump arena (128 KiB) — disjoint from guest sandbox.
pub const ARENA_SIZE: usize = 128 * 1024;

/// ISR stack size — must satisfy worst-case sovereign bootstrap + interpreter depth.
pub const ISR_STACK_SIZE: usize = 4 * 1024;

/// 16-byte aligned static blob (Rust does not allow `#[repr(align)]` on statics directly).
#[repr(C, align(16))]
struct AlignedBytes<const N: usize>([u8; N]);

/// Guest linear memory backing (WASM address space index 0).
static mut SANDBOX_MEMORY: AlignedBytes<SANDBOX_MEMORY_SIZE> = AlignedBytes([0; SANDBOX_MEMORY_SIZE]);

/// Host allocation arena (never shared with guest mappings).
static mut ARENA: AlignedBytes<ARENA_SIZE> = AlignedBytes([0; ARENA_SIZE]);

/// Interrupt-service stack (grows downward; SP initialized to top).
static mut ISR_STACK: AlignedBytes<ISR_STACK_SIZE> = AlignedBytes([0; ISR_STACK_SIZE]);

static ARENA_CURSOR: AtomicUsize = AtomicUsize::new(0);
static ARENA_LOCK: Mutex<()> = Mutex::new(());
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
pub struct SandboxRegion {
    base: *const u8,
    size: usize,
}

impl SandboxRegion {
    /// Obtain the static sandbox region descriptor.
    #[inline]
    pub fn get() -> Self {
        // SAFETY: `SANDBOX_MEMORY` is `'static` and not relocated after link.
        Self {
            base: unsafe { SANDBOX_MEMORY.0.as_ptr() },
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
    /// Caller must respect WASM single-threaded aliasing rules.
    pub unsafe fn guest_slice(&self, offset: u32, len: usize) -> Result<&[u8], MemoryFault> {
        self.check_guest_slice(offset, len)?;
        // SAFETY: Bounds checked via `check_guest_slice`; base is valid for `'static` sandbox storage.
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
    // SAFETY: Unique ownership of SANDBOX_MEMORY during shutdown; interrupts masked.
    unsafe {
        core::ptr::write_bytes(SANDBOX_MEMORY.0.as_mut_ptr(), 0, SANDBOX_MEMORY_SIZE);
    }
    SANDBOX_SEALED.store(true, Ordering::Release);
}

/// Reset arena cursor for a fresh boot cycle (called once per wake from dormancy).
pub fn reset_arena() {
    let _guard = ARENA_LOCK.lock();
    ARENA_CURSOR.store(0, Ordering::Release);
    SANDBOX_SEALED.store(false, Ordering::Release);
    unsafe {
        core::ptr::write_bytes(SANDBOX_MEMORY.0.as_mut_ptr(), 0, SANDBOX_MEMORY_SIZE);
    }
}

/// Top-of-stack address for ISR entry (x86 grows down).
pub fn isr_stack_top() -> usize {
    // SAFETY: ISR_STACK is static; address of one-past-last element is 16-byte aligned.
        unsafe { ISR_STACK.0.as_ptr().add(ISR_STACK_SIZE) as usize }
}

/// Bump allocator backing `wasmi` and transient host structures.
pub struct ArenaAllocator;

unsafe impl GlobalAlloc for ArenaAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _guard = ARENA_LOCK.lock();
        let align = layout.align();
        let size = layout.size();
        let cursor = ARENA_CURSOR.load(Ordering::Relaxed);
        let aligned = (cursor + align - 1) & !(align - 1);
        let new_cursor = match aligned.checked_add(size) {
            Some(n) => n,
            None => return core::ptr::null_mut(),
        };
        if new_cursor > ARENA_SIZE {
            return core::ptr::null_mut();
        }
        ARENA_CURSOR.store(new_cursor, Ordering::Relaxed);
        // SAFETY: `[aligned, new_cursor)` lies wholly inside ARENA by bounds check above.
        unsafe { ARENA.0.as_mut_ptr().add(aligned) }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Intentional no-op: bump allocator frees only on `reset_arena`.
    }
}

/// Global allocator registration for the `alloc` crate.
#[global_allocator]
static GLOBAL: ArenaAllocator = ArenaAllocator;

/// Non-null pointer helper used by runtime when wiring sandbox pages.
pub fn sandbox_non_null() -> Option<NonNull<u8>> {
    NonNull::new(SandboxRegion::get().base_mut_ptr())
}
