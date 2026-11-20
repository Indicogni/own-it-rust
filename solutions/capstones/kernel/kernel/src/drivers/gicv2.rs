//! Driver for the Arm Generic Interrupt Controller, architecture version 2.
//!
//! Register offsets are from the "ARM Generic Interrupt Controller
//! Architecture Specification, version 2.0" (Arm IHI 0048), chapter 4.
//!
//! The GIC has two register blocks. The *distributor* is shared by all
//! CPUs: it knows every interrupt line, whether it is enabled and how
//! urgent it is. The *CPU interface* exists once per CPU: it hands the most
//! urgent pending interrupt to that CPU and is told when the handler is
//! done.

use core::mem::offset_of;

use kcore::reg::{gic_bit_slot, gic_byte_slot, set_field};

use super::InterruptController;
use crate::mmio::{Mmio, read_reg, write_reg};

#[repr(C)]
pub struct Distributor {
    /// 0x000 control: bit 0 enables forwarding to the CPU interfaces.
    ctlr: u32,
    /// 0x004 type (read only).
    typer: u32,
    /// 0x008 implementer identification (read only).
    iidr: u32,
    _reserved0: [u32; 29],
    /// 0x080 interrupt group, one bit per interrupt.
    igroupr: [u32; 32],
    /// 0x100 set-enable, one bit per interrupt; writing 1 enables.
    isenabler: [u32; 32],
    /// 0x180 clear-enable; writing 1 disables.
    icenabler: [u32; 32],
    /// 0x200 set-pending.
    ispendr: [u32; 32],
    /// 0x280 clear-pending.
    icpendr: [u32; 32],
    /// 0x300 set-active.
    isactiver: [u32; 32],
    /// 0x380 clear-active.
    icactiver: [u32; 32],
    /// 0x400 priority, one byte per interrupt; a lower value is more urgent.
    ipriorityr: [u32; 256],
    /// 0x800 target CPUs, one byte per interrupt (a bit mask of CPUs).
    itargetsr: [u32; 256],
}

#[repr(C)]
pub struct CpuInterface {
    /// 0x00 control: bit 0 enables signalling to this CPU.
    ctlr: u32,
    /// 0x04 priority mask: only interrupts more urgent than this pass.
    pmr: u32,
    /// 0x08 binary point.
    bpr: u32,
    /// 0x0c interrupt acknowledge: reading it claims the pending interrupt.
    iar: u32,
    /// 0x10 end of interrupt: writing an id back finishes its handling.
    eoir: u32,
}

const _: () = {
    assert!(offset_of!(Distributor, igroupr) == 0x080);
    assert!(offset_of!(Distributor, isenabler) == 0x100);
    assert!(offset_of!(Distributor, ipriorityr) == 0x400);
    assert!(offset_of!(Distributor, itargetsr) == 0x800);
    assert!(offset_of!(CpuInterface, iar) == 0x0c);
    assert!(offset_of!(CpuInterface, eoir) == 0x10);
};

/// Ids 0..=31 are private to one CPU (SGIs and PPIs); shared peripheral
/// interrupts (SPIs) start at 32.
const FIRST_SPI: u32 = 32;
/// Ids from 1020 up are special; 1023 means "nothing is pending".
const FIRST_SPECIAL_INTID: u32 = 1020;
/// The priority given to every interrupt this kernel enables. It only has
/// to be more urgent (numerically lower) than the mask below.
const DEFAULT_PRIORITY: u64 = 0xa0;
/// Lowest urgency: lets every priority through.
const PRIORITY_MASK_ALL: u32 = 0xff;

pub struct GicV2 {
    dist: Mmio<Distributor>,
    cpu: Mmio<CpuInterface>,
}

impl GicV2 {
    /// # Safety
    ///
    /// The two addresses must be the distributor and the CPU interface of a
    /// GICv2 that no other code drives.
    pub const unsafe fn new(distributor: usize, cpu_interface: usize) -> Self {
        Self {
            // SAFETY: forwarded from this function's own contract.
            dist: unsafe { Mmio::new(distributor) },
            // SAFETY: as above.
            cpu: unsafe { Mmio::new(cpu_interface) },
        }
    }
}

impl InterruptController for GicV2 {
    fn init(&self) {
        // Configure with forwarding off, then switch both halves on.
        write_reg!(self.dist, ctlr, 0);
        write_reg!(self.cpu, ctlr, 0);
        write_reg!(self.cpu, pmr, PRIORITY_MASK_ALL);
        write_reg!(self.cpu, bpr, 0);
        write_reg!(self.cpu, ctlr, 1);
        write_reg!(self.dist, ctlr, 1);
    }

    fn enable(&self, intid: u32) {
        let (index, shift) = gic_byte_slot(intid);
        let priorities = u64::from(read_reg!(self.dist, ipriorityr[index]));
        let priorities = set_field(priorities, shift + 7, shift, DEFAULT_PRIORITY);
        write_reg!(self.dist, ipriorityr[index], priorities as u32);

        if intid >= FIRST_SPI {
            // Route shared interrupts to CPU 0. The target bytes of private
            // interrupts are read-only.
            let targets = u64::from(read_reg!(self.dist, itargetsr[index]));
            let targets = set_field(targets, shift + 7, shift, 0b1);
            write_reg!(self.dist, itargetsr[index], targets as u32);
        }

        // Set-enable registers ignore zero bits, so no read is needed.
        let (index, mask) = gic_bit_slot(intid);
        write_reg!(self.dist, isenabler[index], mask);
    }

    fn claim(&self) -> Option<u32> {
        // Bits 9:0 hold the interrupt id. (Bits 12:10 name the sending CPU
        // of a software-generated interrupt; this kernel uses none.)
        let intid = read_reg!(self.cpu, iar) & 0x3ff;
        (intid < FIRST_SPECIAL_INTID).then_some(intid)
    }

    fn complete(&self, intid: u32) {
        write_reg!(self.cpu, eoir, intid);
    }
}
