//! Driver for the Arm PL011 UART.
//!
//! Register offsets and bit positions are from the "PrimeCell UART (PL011)
//! Technical Reference Manual" (Arm DDI 0183), chapter 3.

use core::mem::offset_of;

use kcore::reg::{pl011_baud_divisors, set_field};

use super::Console;
use crate::mmio::{Mmio, read_reg, write_reg};

/// The register block. Every register is 32 bits wide.
#[repr(C)]
pub struct Registers {
    /// 0x00 data register: a write sends a byte, a read receives one.
    dr: u32,
    /// 0x04 receive status / error clear.
    rsr_ecr: u32,
    _reserved0: [u32; 4],
    /// 0x18 flag register (read only).
    fr: u32,
    _reserved1: u32,
    /// 0x20 IrDA low-power counter (unused).
    ilpr: u32,
    /// 0x24 integer part of the baud rate divisor.
    ibrd: u32,
    /// 0x28 fractional part of the baud rate divisor.
    fbrd: u32,
    /// 0x2c line control.
    lcr_h: u32,
    /// 0x30 control.
    cr: u32,
    /// 0x34 interrupt FIFO level select.
    ifls: u32,
    /// 0x38 interrupt mask set/clear.
    imsc: u32,
    /// 0x3c raw interrupt status.
    ris: u32,
    /// 0x40 masked interrupt status.
    mis: u32,
    /// 0x44 interrupt clear (write only).
    icr: u32,
}

// The compiler checks the struct against the data sheet. A wrong padding
// array becomes a build error instead of a silent write to the wrong
// register.
const _: () = {
    assert!(offset_of!(Registers, fr) == 0x18);
    assert!(offset_of!(Registers, ibrd) == 0x24);
    assert!(offset_of!(Registers, lcr_h) == 0x2c);
    assert!(offset_of!(Registers, cr) == 0x30);
    assert!(offset_of!(Registers, icr) == 0x44);
};

/// FR: the UART is still shifting out a byte.
const FR_BUSY: u32 = 1 << 3;
/// FR: the transmit FIFO is full.
const FR_TXFF: u32 = 1 << 5;
/// CR: UART enable.
const CR_UARTEN: u32 = 1 << 0;
/// CR: transmit enable.
const CR_TXE: u32 = 1 << 8;
/// CR: receive enable.
const CR_RXE: u32 = 1 << 9;
/// ICR: all eleven interrupt-clear bits.
const ICR_ALL: u32 = 0x7ff;

const BAUD: u32 = 115_200;

pub struct Pl011 {
    regs: Mmio<Registers>,
    clock_hz: u32,
}

impl Pl011 {
    /// # Safety
    ///
    /// `base` must be the address of a PL011 that no other code drives, and
    /// `clock_hz` must be the frequency of its reference clock (`UARTCLK`).
    pub const unsafe fn new(base: usize, clock_hz: u32) -> Self {
        Self {
            // SAFETY: forwarded from this function's own contract.
            regs: unsafe { Mmio::new(base) },
            clock_hz,
        }
    }

    /// Programs 115200 baud, 8 data bits, no parity, 1 stop bit, FIFOs on.
    ///
    /// The order follows the manual: disable the UART, wait for the current
    /// byte, program the divisors, then write the line control register
    /// (that write latches the divisors), then enable.
    pub fn init(&self) {
        write_reg!(self.regs, cr, 0);
        self.flush();
        write_reg!(self.regs, icr, ICR_ALL);

        let (integer, fraction) = pl011_baud_divisors(self.clock_hz, BAUD);
        write_reg!(self.regs, ibrd, integer);
        write_reg!(self.regs, fbrd, fraction);

        // LCR_H: WLEN (bits 6:5) = 0b11 for 8 data bits, FEN (bit 4) = 1.
        let line_control = set_field(set_field(0, 6, 5, 0b11), 4, 4, 1);
        write_reg!(self.regs, lcr_h, line_control as u32);

        // The kernel polls; no UART interrupt may reach the controller.
        write_reg!(self.regs, imsc, 0);
        write_reg!(self.regs, cr, CR_UARTEN | CR_TXE | CR_RXE);
    }
}

impl Console for Pl011 {
    fn name(&self) -> &'static str {
        "pl011"
    }

    fn write_byte(&self, byte: u8) {
        while read_reg!(self.regs, fr) & FR_TXFF != 0 {
            core::hint::spin_loop();
        }
        write_reg!(self.regs, dr, u32::from(byte));
    }

    fn flush(&self) {
        while read_reg!(self.regs, fr) & FR_BUSY != 0 {
            core::hint::spin_loop();
        }
    }
}
