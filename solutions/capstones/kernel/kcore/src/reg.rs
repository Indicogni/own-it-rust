//! Pure arithmetic for hardware registers.
//!
//! Nothing here touches hardware. The drivers in the kernel binary compute a
//! value with these functions and then perform one volatile write, so the
//! error-prone part (bit positions and rounding) is tested on the host.

/// A mask with bits `lo ..= hi` set. Requires `lo <= hi <= 63`.
const fn mask(hi: u32, lo: u32) -> u64 {
    assert!(
        lo <= hi && hi <= 63,
        "bit range must satisfy lo <= hi <= 63"
    );
    (u64::MAX >> (63 - hi)) & (u64::MAX << lo)
}

/// Extracts bits `lo ..= hi` of `value`, shifted down to bit 0.
pub const fn get_field(value: u64, hi: u32, lo: u32) -> u64 {
    (value & mask(hi, lo)) >> lo
}

/// Returns `value` with bits `lo ..= hi` replaced by `field`. Bits of
/// `field` that do not fit in the range are dropped; all other bits of
/// `value` are kept.
pub const fn set_field(value: u64, hi: u32, lo: u32, field: u64) -> u64 {
    let mask = mask(hi, lo);
    (value & !mask) | ((field << lo) & mask)
}

/// The PL011 baud rate divisor as `(UARTIBRD, UARTFBRD)`.
///
/// The divisor is `uartclk_hz / (16 * baud)`. The device takes it as a
/// 16-bit integer part and a 6-bit fraction (units of 1/64), rounded to the
/// nearest unit. Working in units of 1/64 keeps everything in integers:
/// `64 * clk / (16 * baud)` is `4 * clk / baud`.
pub const fn pl011_baud_divisors(uartclk_hz: u32, baud: u32) -> (u32, u32) {
    assert!(baud > 0, "baud rate must not be zero");
    let baud = baud as u64;
    let in_64ths = (4 * uartclk_hz as u64 + baud / 2) / baud;
    ((in_64ths >> 6) as u32, (in_64ths & 0x3f) as u32)
}

/// Where the one-bit-per-interrupt GIC registers (`GICD_ISENABLERn`,
/// `GICD_ICENABLERn`, ...) keep the bit for `intid`: `(n, mask)`.
pub const fn gic_bit_slot(intid: u32) -> (usize, u32) {
    ((intid / 32) as usize, 1 << (intid % 32))
}

/// Where the one-byte-per-interrupt GIC registers (`GICD_IPRIORITYRn`,
/// `GICD_ITARGETSRn`) keep the byte for `intid`: `(n, shift)`.
pub const fn gic_byte_slot(intid: u32) -> (usize, u32) {
    ((intid / 4) as usize, 8 * (intid % 4))
}

/// The number of counter ticks between two timer interrupts.
pub const fn timer_interval(counter_hz: u64, tick_hz: u64) -> u64 {
    assert!(tick_hz > 0, "tick rate must not be zero");
    counter_hz / tick_hz
}

/// The cause of a synchronous exception: the `EC` field of `ESR_EL1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExceptionClass {
    /// EC 0x00: the CPU could not classify the cause.
    Unknown,
    /// EC 0x15: an `svc` instruction executed in AArch64 state.
    Svc,
    /// EC 0x20: instruction abort taken from a lower exception level.
    InstructionAbortLower,
    /// EC 0x21: instruction abort taken from the current exception level.
    InstructionAbortSame,
    /// EC 0x24: data abort taken from a lower exception level.
    DataAbortLower,
    /// EC 0x25: data abort taken from the current exception level.
    DataAbortSame,
    /// EC 0x3c: a `brk` instruction executed in AArch64 state.
    Brk,
    /// Any other value of the 6-bit field.
    Other(u8),
}

/// Decodes bits 31..=26 of an `ESR_EL1` value.
pub const fn exception_class(esr: u64) -> ExceptionClass {
    match get_field(esr, 31, 26) as u8 {
        0x00 => ExceptionClass::Unknown,
        0x15 => ExceptionClass::Svc,
        0x20 => ExceptionClass::InstructionAbortLower,
        0x21 => ExceptionClass::InstructionAbortSame,
        0x24 => ExceptionClass::DataAbortLower,
        0x25 => ExceptionClass::DataAbortSame,
        0x3c => ExceptionClass::Brk,
        other => ExceptionClass::Other(other),
    }
}

/// The 16-bit immediate of the `svc` instruction that caused `esr`, or
/// `None` when the exception is not an SVC.
pub const fn svc_number(esr: u64) -> Option<u16> {
    match exception_class(esr) {
        ExceptionClass::Svc => Some(get_field(esr, 15, 0) as u16),
        _ => None,
    }
}
