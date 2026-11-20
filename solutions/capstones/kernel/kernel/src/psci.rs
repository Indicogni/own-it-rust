//! Power control through PSCI, the Power State Coordination Interface.
//!
//! On real boards PSCI is implemented by firmware at a higher exception
//! level. On the QEMU `virt` machine QEMU itself answers the calls. A guest
//! that starts at EL1 reaches it with the `hvc` instruction; the function
//! number goes in x0 as defined by "Arm Power State Coordination Interface"
//! (Arm DEN 0022), section 5.

use core::arch::asm;

use crate::cpu;

/// `SYSTEM_OFF`: function 8 of the 32-bit PSCI range.
const PSCI_SYSTEM_OFF: u64 = 0x8400_0008;

/// Switches the machine off. Under QEMU the emulator exits with status 0.
pub fn system_off() -> ! {
    // SAFETY: `hvc #0` with a PSCI function id in x0 is the calling
    // convention the platform documents for a guest at EL1. `SYSTEM_OFF`
    // takes no arguments and does not return. The call may change x0..x3,
    // which are declared as clobbered.
    unsafe {
        asm!(
            "hvc #0",
            inout("x0") PSCI_SYSTEM_OFF => _,
            lateout("x1") _,
            lateout("x2") _,
            lateout("x3") _,
            options(nomem, nostack),
        );
    }
    // Only reached if the platform has no PSCI.
    loop {
        cpu::wait_for_interrupt();
    }
}
