//! The kernel binary. This is the only code that the starter gives you.
//!
//! It builds for the bare-metal target and does nothing: there is no entry
//! point, no linker script and no driver yet. Start with SPEC.md, milestone 1.
//!
//! Build it from this directory:
//!
//! ```text
//! cargo build --target aarch64-unknown-none
//! ```

#![no_std]
#![no_main]

use core::panic::PanicInfo;

/// Rust needs exactly one panic handler in a `no_std` program. This one only
/// stops. SPEC 4.3 says what yours has to do.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
