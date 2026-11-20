//! The QEMU `virt` machine: where each device lives and which driver owns it.
//!
//! This is the only file that knows addresses and interrupt numbers. The
//! values come from the table `base_memmap` in QEMU's `hw/arm/virt.c` and
//! from `include/hw/arm/bsa.h` (timer interrupts) and are listed in SPEC 2.

use crate::drivers::generic_timer::GenericTimer;
use crate::drivers::gicv2::GicV2;
use crate::drivers::pl011::Pl011;
use crate::drivers::{Console, InterruptController, Timer};

const GIC_DISTRIBUTOR: usize = 0x0800_0000;
const GIC_CPU_INTERFACE: usize = 0x0801_0000;
const UART0: usize = 0x0900_0000;
/// The fixed 24 MHz clock that feeds the UART on this machine.
const UART0_CLOCK_HZ: u32 = 24_000_000;

/// The EL1 virtual timer is private peripheral interrupt 11. Private
/// peripheral interrupts start at id 16, so the GIC calls it 27.
pub const TIMER_INTID: u32 = 16 + 11;

/// Timer interrupts per second.
pub const TICK_HZ: u64 = 100;

// SAFETY: 0x0900_0000 is the first PL011 of the `virt` machine and 24 MHz is
// its reference clock. This static is the only driver for that device.
static UART: Pl011 = unsafe { Pl011::new(UART0, UART0_CLOCK_HZ) };

// SAFETY: with `gic-version=2` these are the distributor and CPU interface
// of the machine's GIC. This static is the only driver for that device.
static GIC: GicV2 = unsafe { GicV2::new(GIC_DISTRIBUTOR, GIC_CPU_INTERFACE) };

static TIMER: GenericTimer = GenericTimer::new();

/// Makes the console usable. Must run before the first `print!`.
pub fn init_console() {
    UART.init();
}

pub fn console() -> &'static dyn Console {
    &UART
}

pub fn timer() -> &'static dyn Timer {
    &TIMER
}

pub fn interrupt_controller() -> &'static dyn InterruptController {
    &GIC
}
