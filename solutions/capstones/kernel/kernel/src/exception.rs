//! Exceptions and interrupts: the vector table, the trap frame and the
//! handlers.
//!
//! When the CPU takes an exception to EL1 it saves the return address in
//! `ELR_EL1` and the processor state in `SPSR_EL1`, masks interrupts and
//! jumps to one of sixteen entries in the table that `VBAR_EL1` points at.
//! Nothing else is saved. The assembly below stores every register the
//! interrupted code could be using into a [`TrapFrame`] on the stack, calls
//! [`trap_handler`], restores the registers and returns with `eret`.

use core::arch::{asm, global_asm};
use core::mem::offset_of;
use core::sync::atomic::{AtomicU64, Ordering};

use kcore::reg::{ExceptionClass, exception_class, svc_number};

use crate::{board, cpu, task};

/// Everything the entry code saves, in the order it is stored.
///
/// The floating-point registers are part of the frame because the compiler
/// uses them in ordinary kernel code on this target. An interrupt handler
/// that changed them would corrupt whatever it interrupted.
#[repr(C)]
pub struct TrapFrame {
    /// x0 to x30.
    pub x: [u64; 31],
    /// `ELR_EL1`: where `eret` continues.
    pub elr: u64,
    /// `SPSR_EL1`: the processor state `eret` restores.
    pub spsr: u64,
    pub fpcr: u64,
    pub fpsr: u64,
    /// q0 to q31. `u128` is 16-byte aligned, which adds 8 bytes of padding
    /// in front of this field.
    pub q: [u128; 32],
}

// The assembly uses these numbers; keep the two in step.
const _: () = {
    assert!(offset_of!(TrapFrame, x) == 0);
    assert!(offset_of!(TrapFrame, elr) == 248);
    assert!(offset_of!(TrapFrame, spsr) == 256);
    assert!(offset_of!(TrapFrame, fpcr) == 264);
    assert!(offset_of!(TrapFrame, fpsr) == 272);
    assert!(offset_of!(TrapFrame, q) == 288);
    assert!(size_of::<TrapFrame>() == 800);
    // AArch64 requires a 16-byte aligned stack pointer.
    assert!(size_of::<TrapFrame>().is_multiple_of(16));
};

global_asm!(
    r#"
    // One table entry. An entry may be at most 32 instructions long, far
    // too short to save all registers, so each one makes room for the
    // frame, frees x0 and x1, records which entry it is and jumps to the
    // shared code.
    .macro VECTOR kind
    .balign 0x80
    sub     sp, sp, #{frame_size}
    stp     x0, x1, [sp, #0]
    mov     x0, #\kind
    b       __trap_entry
    .endm

    .section .text.vectors, "ax"
    .balign 0x800
    .global __exception_vectors
__exception_vectors:
    // Taken from the current EL while using SP_EL0 (never used here).
    VECTOR 0        // synchronous
    VECTOR 1        // IRQ
    VECTOR 2        // FIQ
    VECTOR 3        // SError
    // Taken from the current EL while using SP_EL1: the kernel itself.
    VECTOR 4
    VECTOR 5
    VECTOR 6
    VECTOR 7
    // Taken from a lower EL running AArch64 (user tasks: stretch goal).
    VECTOR 8
    VECTOR 9
    VECTOR 10
    VECTOR 11
    // Taken from a lower EL running AArch32.
    VECTOR 12
    VECTOR 13
    VECTOR 14
    VECTOR 15

__trap_entry:
    stp     x2, x3, [sp, #16]
    stp     x4, x5, [sp, #32]
    stp     x6, x7, [sp, #48]
    stp     x8, x9, [sp, #64]
    stp     x10, x11, [sp, #80]
    stp     x12, x13, [sp, #96]
    stp     x14, x15, [sp, #112]
    stp     x16, x17, [sp, #128]
    stp     x18, x19, [sp, #144]
    stp     x20, x21, [sp, #160]
    stp     x22, x23, [sp, #176]
    stp     x24, x25, [sp, #192]
    stp     x26, x27, [sp, #208]
    stp     x28, x29, [sp, #224]
    mrs     x1, elr_el1
    stp     x30, x1, [sp, #240]
    mrs     x1, spsr_el1
    mrs     x2, fpcr
    stp     x1, x2, [sp, #256]
    mrs     x1, fpsr
    str     x1, [sp, #272]
    stp     q0, q1, [sp, #288]
    stp     q2, q3, [sp, #320]
    stp     q4, q5, [sp, #352]
    stp     q6, q7, [sp, #384]
    stp     q8, q9, [sp, #416]
    stp     q10, q11, [sp, #448]
    stp     q12, q13, [sp, #480]
    stp     q14, q15, [sp, #512]
    stp     q16, q17, [sp, #544]
    stp     q18, q19, [sp, #576]
    stp     q20, q21, [sp, #608]
    stp     q22, q23, [sp, #640]
    stp     q24, q25, [sp, #672]
    stp     q26, q27, [sp, #704]
    stp     q28, q29, [sp, #736]
    stp     q30, q31, [sp, #768]

    // x0 = which vector, x1 = address of the frame.
    mov     x1, sp
    bl      {handler}

    ldp     q0, q1, [sp, #288]
    ldp     q2, q3, [sp, #320]
    ldp     q4, q5, [sp, #352]
    ldp     q6, q7, [sp, #384]
    ldp     q8, q9, [sp, #416]
    ldp     q10, q11, [sp, #448]
    ldp     q12, q13, [sp, #480]
    ldp     q14, q15, [sp, #512]
    ldp     q16, q17, [sp, #544]
    ldp     q18, q19, [sp, #576]
    ldp     q20, q21, [sp, #608]
    ldp     q22, q23, [sp, #640]
    ldp     q24, q25, [sp, #672]
    ldp     q26, q27, [sp, #704]
    ldp     q28, q29, [sp, #736]
    ldp     q30, q31, [sp, #768]
    ldr     x1, [sp, #272]
    msr     fpsr, x1
    ldp     x1, x2, [sp, #256]
    msr     spsr_el1, x1
    msr     fpcr, x2
    ldp     x30, x1, [sp, #240]
    msr     elr_el1, x1
    ldp     x28, x29, [sp, #224]
    ldp     x26, x27, [sp, #208]
    ldp     x24, x25, [sp, #192]
    ldp     x22, x23, [sp, #176]
    ldp     x20, x21, [sp, #160]
    ldp     x18, x19, [sp, #144]
    ldp     x16, x17, [sp, #128]
    ldp     x14, x15, [sp, #112]
    ldp     x12, x13, [sp, #96]
    ldp     x10, x11, [sp, #80]
    ldp     x8, x9, [sp, #64]
    ldp     x6, x7, [sp, #48]
    ldp     x4, x5, [sp, #32]
    ldp     x2, x3, [sp, #16]
    ldp     x0, x1, [sp, #0]
    add     sp, sp, #{frame_size}
    eret
"#,
    frame_size = const size_of::<TrapFrame>(),
    handler = sym trap_handler,
);

// SAFETY: the symbol is the label of the vector table in the assembly above.
// Only its address is used; the type `u8` is a placeholder.
unsafe extern "C" {
    static __exception_vectors: u8;
}

/// Vector numbers as passed by the `VECTOR` macro.
const SYNC_CURRENT_EL: u64 = 4;
const IRQ_CURRENT_EL: u64 = 5;

/// Timer interrupts handled since boot.
static TICKS: AtomicU64 = AtomicU64::new(0);

pub fn ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}

/// Points `VBAR_EL1` at the vector table.
pub fn install_vectors() {
    let table = (&raw const __exception_vectors).addr();
    // SAFETY: `__exception_vectors` is the 2 KiB-aligned table of sixteen
    // entries defined in the assembly above. It is part of `.text`, so it
    // stays in place. Every entry leads to `__trap_entry`, which saves the
    // whole interrupted context and restores it before `eret`; the Rust
    // handler changes only the saved x0, and only for an `svc`.
    unsafe { cpu::set_vector_table(table) };
}

fn read_esr() -> u64 {
    let esr: u64;
    // SAFETY: `ESR_EL1` is readable at EL1 and the read has no side effects.
    unsafe {
        asm!("mrs {}, esr_el1", out(reg) esr, options(nomem, nostack, preserves_flags));
    }
    esr
}

fn read_far() -> u64 {
    let far: u64;
    // SAFETY: `FAR_EL1` is readable at EL1 and the read has no side effects.
    unsafe {
        asm!("mrs {}, far_el1", out(reg) far, options(nomem, nostack, preserves_flags));
    }
    far
}

/// Called by `__trap_entry` for every exception.
///
/// `frame` is the register save area on the stack of the interrupted code.
/// The assembly passes its address and does not touch it while this
/// function runs, so the exclusive reference is sound. Changes to the frame
/// become the register values of the interrupted code after `eret`.
extern "C" fn trap_handler(vector: u64, frame: &mut TrapFrame) {
    match vector {
        SYNC_CURRENT_EL => handle_sync(frame),
        IRQ_CURRENT_EL => handle_irq(),
        _ => fatal(vector, frame),
    }
}

fn handle_sync(frame: &mut TrapFrame) {
    let esr = read_esr();
    match svc_number(esr) {
        // For an SVC, `ELR_EL1` already points at the instruction after
        // `svc`, so returning continues the caller. SPEC 4.1 (M4) defines
        // the result: the immediate comes back in x0.
        Some(number) => frame.x[0] = u64::from(number),
        None => fatal(SYNC_CURRENT_EL, frame),
    }
}

fn handle_irq() {
    let controller = board::interrupt_controller();
    let Some(intid) = controller.claim() else {
        return;
    };
    let is_tick = intid == board::TIMER_INTID;
    if is_tick {
        board::timer().acknowledge();
        TICKS.fetch_add(1, Ordering::Relaxed);
    }
    // Finish the interrupt *before* a possible task switch. The task that
    // runs next may not come back here for a long time, and the controller
    // delivers no further timer interrupt until this one is completed.
    controller.complete(intid);
    if is_tick {
        task::on_tick();
    }
}

fn fatal(vector: u64, frame: &TrapFrame) -> ! {
    let esr = read_esr();
    let class: ExceptionClass = exception_class(esr);
    panic!(
        "unexpected exception: vector {vector}, {class:?}, esr {esr:#x}, elr {:#x}, far {:#x}",
        frame.elr,
        read_far(),
    );
}
