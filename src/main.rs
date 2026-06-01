//! AETHER-ENCLAVE bare-metal binary — bootloader entry and dormancy lifecycle.
//!
//! ```text
//! Deep Dormancy (HLT)
//!   -> Physical IRQ (0x20 / 0x21)
//!   -> ISR (cli, sovereign_bootstrap)
//!   -> WASM diagnostic + MMIO proof
//!   -> self_annihilate (zero, PMU sleep)
//! ```

#![no_std]
#![no_main]

use core::panic::PanicInfo;

use bootloader::{entry_point, BootInfo};

use aether_enclave::{interrupts, memory, mmio, runtime, shutdown, serial_println};

entry_point!(kernel_main);

/// Kernel entry — stack and page tables initialized by the bootloader; receives [`BootInfo`].
fn kernel_main(_boot_info: &'static BootInfo) -> ! {
    memory::reset_arena();
    mmio::serial_init();
    interrupts::init();

    serial_println!("[AETHER] cold boot — COM1 ready, entering dormancy");

    dormancy_loop();
}

/// Absolute zero-power wait — processor halted until IRQ.
fn dormancy_loop() -> ! {
    loop {
        interrupts::enable();
        interrupts::halt_until_interrupt();

        if interrupts::wake_pending() {
            interrupts::clear_wake();
            let vector = interrupts::last_vector();
            serial_println!(
                "[AETHER] spurious wake — vector 0x{:02X}, handoff to bootstrap",
                vector
            );
            runtime::sovereign_bootstrap(interrupts::HardwareInterrupt::from_vector(vector));
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    shutdown::self_annihilate(shutdown::ShutdownReport {
        guest_result: -1,
        proof: 0,
        vector: interrupts::last_vector(),
    });
}
