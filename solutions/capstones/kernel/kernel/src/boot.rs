//! The first instructions of the kernel.
//!
//! QEMU loads the ELF image and jumps to `_start` at EL1 with interrupts
//! masked, the MMU off and no stack. Rust code needs a stack and a zeroed
//! `.bss`, so a few lines of assembly prepare both and then call `kmain`.

use core::arch::global_asm;

global_asm!(
    r#"
    .section .text.boot, "ax"
    .global _start
_start:
    // Mask debug, SError, IRQ and FIQ until the kernel is ready for them.
    msr     daifset, #0xf

    // Stay parked unless this is EL1: the rest of the kernel assumes it.
    mrs     x0, CurrentEL
    cmp     x0, #(1 << 2)
    b.ne    3f

    // CPACR_EL1.FPEN = 0b11: do not trap floating-point and SIMD registers.
    // The compiler uses them freely on this target, even for plain copies.
    mov     x0, #(3 << 20)
    msr     cpacr_el1, x0
    isb

    // The stack grows down from the address the linker script reserved.
    adrp    x0, __stack_top
    add     x0, x0, :lo12:__stack_top
    mov     sp, x0

    // Clear .bss. Both symbols are 16-byte aligned (see link.ld), so
    // eight-byte stores cover the range exactly.
    adrp    x0, __bss_start
    add     x0, x0, :lo12:__bss_start
    adrp    x1, __bss_end
    add     x1, x1, :lo12:__bss_end
1:  cmp     x0, x1
    b.hs    2f
    str     xzr, [x0], #8
    b       1b

    // A zero frame pointer and link register end every backtrace here.
2:  mov     x29, #0
    mov     x30, #0
    bl      kmain

    // kmain never returns; park the CPU if it ever does.
3:  wfi
    b       3b
"#
);

// SAFETY: both symbols are defined by `link.ld`. They mark addresses and have
// no content of their own, so this file only ever takes their addresses; the
// type `u8` is a placeholder.
unsafe extern "C" {
    static __bss_start: u8;
    static __bss_end: u8;
}

/// Checks that every byte of `.bss` is zero.
///
/// Only meaningful as the first thing `kmain` does, before any static has
/// been written.
pub fn bss_is_zero() -> bool {
    let start = (&raw const __bss_start).addr();
    let end = (&raw const __bss_end).addr();
    (start..end).step_by(8).all(|addr| {
        // SAFETY: `start..end` is the `.bss` output section, which the
        // linker script aligns to 16 bytes at both ends, so every `addr` is
        // an aligned `u64` inside the kernel image. The entry code has
        // initialised the whole range, and at this point no reference to
        // any static in it exists. The access is a volatile read, so it
        // cannot be merged with or reordered against other accesses.
        unsafe { core::ptr::with_exposed_provenance::<u64>(addr).read_volatile() == 0 }
    })
}
