//! Thin wrappers around AArch64 instructions that Rust has no syntax for.
//!
//! Keeping every such instruction in one small file means the rest of the
//! kernel can read `cpu::current_el()` instead of an `asm!` block.

use core::arch::asm;
use core::sync::atomic::{AtomicBool, Ordering};

/// Set by [`set_vector_table`]. Until then an exception would send the CPU
/// to whatever address `VBAR_EL1` happens to hold, so the two functions that
/// make exceptions possible ([`irq_enable`] and [`svc`]) check this flag.
static VECTORS_INSTALLED: AtomicBool = AtomicBool::new(false);

/// The exception level the CPU runs at: 0, 1, 2 or 3.
pub fn current_el() -> u8 {
    let value: u64;
    // SAFETY: reading `CurrentEL` is allowed at EL1 and above and has no
    // side effects. The instruction touches neither memory nor flags.
    unsafe {
        asm!("mrs {}, CurrentEL", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    // The level is in bits 3:2.
    ((value >> 2) & 0b11) as u8
}

/// Masks IRQs while it is alive and restores the previous state when it is
/// dropped. Guards nest: an inner guard restores "masked", the outer one
/// restores whatever was there before.
pub struct IrqGuard {
    saved_daif: u64,
}

impl IrqGuard {
    pub fn new() -> Self {
        let saved_daif: u64;
        // SAFETY: reading `DAIF` and setting its I bit are allowed at EL1.
        // Masking an interrupt can delay it but cannot break memory safety.
        unsafe {
            asm!(
                "mrs {}, daif",
                "msr daifset, #2",
                out(reg) saved_daif,
                options(nomem, nostack, preserves_flags),
            );
        }
        Self { saved_daif }
    }
}

impl Drop for IrqGuard {
    fn drop(&mut self) {
        // SAFETY: writes back the value that `new` read from the same
        // register, so the CPU returns to an interrupt state the kernel was
        // already running in.
        unsafe {
            asm!("msr daif, {}", in(reg) self.saved_daif, options(nomem, nostack, preserves_flags));
        }
    }
}

/// Points `VBAR_EL1` at a vector table.
///
/// # Safety
///
/// `table` must be the address of an exception vector table: 2 KiB aligned,
/// sixteen entries, in place for the rest of the run. An entry that returns
/// must restore every register of the interrupted code first; the entry for
/// synchronous exceptions from EL1 may change x0 after an `svc`.
pub unsafe fn set_vector_table(table: usize) {
    // SAFETY: writing `VBAR_EL1` is allowed at EL1, and the caller
    // guarantees that `table` is a vector table. The `isb` makes the new
    // value take effect before the next instruction.
    unsafe {
        asm!("msr vbar_el1, {}", "isb", in(reg) table, options(nomem, nostack, preserves_flags));
    }
    VECTORS_INSTALLED.store(true, Ordering::Relaxed);
}

fn assert_vectors_installed(operation: &str) {
    assert!(
        VECTORS_INSTALLED.load(Ordering::Relaxed),
        "{operation} before the exception vectors are installed"
    );
}

/// Unmasks IRQs.
///
/// # Panics
///
/// Panics if no vector table is installed yet.
///
/// Calling this inside a critical section ([`IrqGuard`], `IrqLock`) is a
/// logic error, not a memory-safety error: an `IrqLock` notices a second
/// user and panics.
pub fn irq_enable() {
    assert_vectors_installed("irq_enable");
    // SAFETY: clearing the I bit lets the CPU take IRQs. The check above
    // proves that a vector table is installed, and `set_vector_table`
    // requires that its entries save and restore every register of the
    // interrupted code. State that handlers share with other code is an
    // atomic or sits in an `IrqLock`.
    unsafe {
        asm!("msr daifclr, #2", options(nomem, nostack, preserves_flags));
    }
}

/// Masks IRQs and leaves them masked.
pub fn irq_disable() {
    // SAFETY: setting the I bit is allowed at EL1 and only delays interrupts.
    unsafe {
        asm!("msr daifset, #2", options(nomem, nostack, preserves_flags));
    }
}

/// Sleeps until an interrupt is pending.
pub fn wait_for_interrupt() {
    // SAFETY: `wfi` only suspends the CPU; execution continues with the next
    // instruction when an interrupt becomes pending.
    unsafe {
        asm!("wfi", options(nomem, nostack, preserves_flags));
    }
}

/// Executes `svc #NUMBER` and returns what the exception handler left in x0.
///
/// # Panics
///
/// Panics if no vector table is installed yet.
pub fn svc<const NUMBER: u16>() -> u64 {
    assert_vectors_installed("svc");
    let result: u64;
    // SAFETY: `svc` raises a synchronous exception. The check above proves
    // that a vector table is installed, and `set_vector_table` requires
    // that its handler restores every register except x0, which is declared
    // as an output here. The handler builds its frame on the current stack,
    // so this block must not claim `nostack` (or `nomem`).
    unsafe {
        asm!("svc #{number}", number = const NUMBER, lateout("x0") result);
    }
    result
}
