//! Bump arena for wasmi. Guest linear memory is capped at [`SANDBOX_MEMORY_SIZE`].

use core::alloc::{GlobalAlloc, Layout};
use spin::Mutex;

/// WebAssembly linear memory page size (64 KiB per Wasm spec).
pub const WASM_PAGE_SIZE: usize = 64 * 1024;

/// Max guest linear memory bytes (wasmi enforces this in `runtime::cap_guest_memory`).
pub const SANDBOX_MEMORY_SIZE: usize = 64 * 1024;

/// Host bump arena — wasmi engine, module compile, transient allocations.
pub const ARENA_SIZE: usize = 128 * 1024;

#[repr(C, align(16))]
struct AlignedBytes<const N: usize>([u8; N]);

struct ArenaState {
    bytes: AlignedBytes<ARENA_SIZE>,
    cursor: usize,
}

static ARENA: Mutex<ArenaState> = Mutex::new(ArenaState {
    bytes: AlignedBytes([0; ARENA_SIZE]),
    cursor: 0,
});

/// Reset arena at boot and zero backing store.
pub fn reset_arena() {
    wipe_host_memory();
}

/// Zero host arena after each cycle (wasmi heap; guest memory is dropped with the store).
pub fn wipe_host_memory() {
    let mut arena = ARENA.lock();
    arena.cursor = 0;
    secure_zero(&mut arena.bytes.0);
}

fn secure_zero(bytes: &mut [u8]) {
    const CHUNK: usize = 4096;
    for chunk in bytes.chunks_mut(CHUNK) {
        #[cfg(target_arch = "riscv32")]
        crate::platform::esp32c6::feed_watchdog();
        chunk.fill(0);
    }
}

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

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[global_allocator]
static GLOBAL: ArenaAllocator = ArenaAllocator;
