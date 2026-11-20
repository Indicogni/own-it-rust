//! A small kernel for the QEMU `virt` machine on AArch64.
//!
//! Where to start reading: `boot.rs` runs first and calls [`kmain`] below.
//! DESIGN.md explains how the files fit together.

#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]

extern crate alloc;

// First, because the macros it defines are visible only to modules that
// are declared after it.
#[macro_use]
mod console;

mod board;
mod boot;
mod cpu;
mod drivers;
mod exception;
mod heap;
mod ipc;
mod milestones;
mod mmio;
mod psci;
mod sync;
mod task;

use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, Ordering};

/// The Rust entry point, called by the assembly in `boot.rs` with a stack,
/// a zeroed `.bss` and IRQs masked.
// SAFETY: `no_mangle` gives the function the exact symbol name the assembly
// branches to. The attribute is unsafe because a second symbol named `kmain`
// would clash at link time; neither this crate nor `core`, `alloc` or
// `kcore` defines one.
#[unsafe(no_mangle)]
extern "C" fn kmain() -> ! {
    // Must come before anything writes to a static.
    let bss_zeroed = boot::bss_is_zero();

    board::init_console();
    println!("[m1] boot: EL{}", cpu::current_el());

    // SPEC 4.3: a build with this feature checks the panic path.
    if cfg!(feature = "selftest-panic") {
        panic!("selftest");
    }

    assert!(bss_zeroed, ".bss is not zero at entry to kmain");
    println!("[m1] bss: zeroed");

    milestones::m2_drivers();
    milestones::m3_heap();
    milestones::m4_interrupts();
    milestones::m5_tasks();
    milestones::m6_ipc();

    power_off()
}

fn power_off() -> ! {
    cpu::irq_disable();
    board::timer().stop();
    println!("[m1] power off");
    board::console().flush();
    psci::system_off()
}

/// Set by the first panic, so that a panic inside the handler does not
/// print (and panic) again without end.
static PANICKING: AtomicBool = AtomicBool::new(false);

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    cpu::irq_disable();
    if !PANICKING.swap(true, Ordering::Relaxed) {
        // `println!` takes no lock, so this works even if the panic came
        // from inside a critical section.
        match info.location() {
            Some(at) => println!("[panic] {}:{}: {}", at.file(), at.line(), info.message()),
            None => println!("[panic] {}", info.message()),
        }
        board::console().flush();
    }
    psci::system_off()
}
