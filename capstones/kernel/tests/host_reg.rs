//! Host acceptance tests for `kcore::reg` (SPEC 5.5).
//!
//! ```text
//! cargo test --test host_reg
//! ```
//!
//! Every expected value below can be checked by hand against the document
//! that SPEC 2 links for the register in question.

use kcore::reg::{
    ExceptionClass, exception_class, get_field, gic_bit_slot, gic_byte_slot, pl011_baud_divisors,
    set_field, svc_number, timer_interval,
};

/// SPEC 5.5: every function in `reg` is a `const fn`, so drivers can
/// compute register values at compile time. This function compiles only if
/// that is true.
const fn computed_in_a_const_fn() -> [u64; 11] {
    let line_control = set_field(set_field(0, 6, 5, 0b11), 4, 4, 1);
    let (integer, fraction) = pl011_baud_divisors(24_000_000, 115_200);
    let (bit_register, bit_mask) = gic_bit_slot(27);
    let (byte_register, byte_shift) = gic_byte_slot(27);
    let is_svc = matches!(exception_class(0x5600_0042), ExceptionClass::Svc);
    let number = match svc_number(0x5600_0042) {
        Some(number) => number,
        None => 0,
    };
    [
        line_control,
        get_field(line_control, 6, 5),
        integer as u64,
        fraction as u64,
        bit_register as u64,
        bit_mask as u64,
        byte_register as u64,
        byte_shift as u64,
        timer_interval(62_500_000, 100),
        is_svc as u64,
        number as u64,
    ]
}

#[test]
fn get_field_extracts_an_inclusive_bit_range() {
    assert_eq!(
        get_field(0b1011_0100, 5, 2),
        0b1101,
        "bits 5..=2 of 0b1011_0100"
    );
    assert_eq!(
        get_field(0x8000_0000_0000_0000, 63, 63),
        1,
        "a single bit: hi == lo"
    );
    assert_eq!(
        get_field(0xdead_beef_0123_4567, 63, 0),
        0xdead_beef_0123_4567,
        "the whole word"
    );
    assert_eq!(
        get_field(0x5600_0042, 31, 26),
        0x15,
        "the EC field of an ESR value"
    );
    assert_eq!(
        get_field(u64::MAX, 7, 0),
        0xff,
        "bits above the range are cut off"
    );
}

#[test]
fn set_field_replaces_only_the_named_bits() {
    assert_eq!(
        set_field(0, 6, 5, 0b11),
        0x60,
        "WLEN = 3 in PL011 UARTLCR_H"
    );
    assert_eq!(
        set_field(0xffff_ffff, 15, 8, 0xa0),
        0xffff_a0ff,
        "the old content of the range is cleared; the other bits stay"
    );
    assert_eq!(
        set_field(0, 3, 0, 0x1f),
        0xf,
        "a value wider than the range must not spill into bit 4"
    );
    assert_eq!(set_field(0, 63, 0, u64::MAX), u64::MAX, "the whole word");
    assert_eq!(
        set_field(u64::MAX, 63, 63, 0),
        u64::MAX >> 1,
        "clearing the top bit"
    );
}

#[test]
fn every_function_is_a_const_fn() {
    assert_eq!(
        computed_in_a_const_fn(),
        [0x70, 0b11, 13, 1, 0, 1 << 27, 6, 24, 625_000, 1, 0x42],
        "the values computed inside a const fn"
    );
}

#[test]
fn pl011_divisors_are_rounded_to_the_nearest_64th() {
    assert_eq!(
        pl011_baud_divisors(24_000_000, 115_200),
        (13, 1),
        "24 MHz / (16 * 115200) = 13.0208: integer 13, fraction 0.0208 * 64 = 1.33 -> 1"
    );
    assert_eq!(
        pl011_baud_divisors(4_000_000, 230_400),
        (1, 5),
        "4 MHz / (16 * 230400) = 1.085: integer 1, fraction 0.085 * 64 + 0.5 = 5.9 -> 5"
    );
    assert_eq!(
        pl011_baud_divisors(3_000_000, 115_200),
        (1, 40),
        "3 MHz / (16 * 115200) = 1.6276: integer 1, fraction 0.6276 * 64 = 40.17 -> 40"
    );
    assert_eq!(
        pl011_baud_divisors(48_000_000, 115_200),
        (26, 3),
        "48 MHz / (16 * 115200) = 26.0417: fraction 0.0417 * 64 = 2.67 rounds UP to 3"
    );
    assert_eq!(
        pl011_baud_divisors(4_000_000_000, 9_600),
        (26_041, 43),
        "a 4 GHz clock must not overflow 32-bit arithmetic"
    );
}

#[test]
fn gic_slots_for_bit_and_byte_registers() {
    assert_eq!(gic_bit_slot(0), (0, 1), "interrupt 0: register 0, bit 0");
    assert_eq!(
        gic_bit_slot(27),
        (0, 1 << 27),
        "the virtual timer (27): register 0, bit 27"
    );
    assert_eq!(
        gic_bit_slot(33),
        (1, 1 << 1),
        "the UART (33): register 1, bit 1"
    );
    assert_eq!(
        gic_bit_slot(1019),
        (31, 1 << 27),
        "the highest interrupt id"
    );

    assert_eq!(gic_byte_slot(0), (0, 0), "interrupt 0: register 0, byte 0");
    assert_eq!(
        gic_byte_slot(27),
        (6, 24),
        "27 = 6 * 4 + 3: register 6, top byte"
    );
    assert_eq!(
        gic_byte_slot(33),
        (8, 8),
        "33 = 8 * 4 + 1: register 8, second byte"
    );
}

#[test]
fn timer_interval_is_counter_ticks_per_tick() {
    assert_eq!(
        timer_interval(62_500_000, 100),
        625_000,
        "62.5 MHz counter, 100 Hz tick"
    );
    assert_eq!(
        timer_interval(1_000_000_000, 100),
        10_000_000,
        "1 GHz counter, 100 Hz tick"
    );
    assert_eq!(
        timer_interval(24_000_000, 1_000),
        24_000,
        "24 MHz counter, 1 kHz tick"
    );
}

#[test]
fn exception_class_decodes_bits_31_to_26() {
    let cases = [
        (0x0200_0000, ExceptionClass::Unknown),
        (0x5600_0042, ExceptionClass::Svc),
        (0x8200_0000, ExceptionClass::InstructionAbortLower),
        (0x8600_0000, ExceptionClass::InstructionAbortSame),
        (0x9200_0045, ExceptionClass::DataAbortLower),
        (0x9600_0045, ExceptionClass::DataAbortSame),
        (0xf200_0001, ExceptionClass::Brk),
        (0x1e00_0000, ExceptionClass::Other(0x07)),
        (0xbe00_0000, ExceptionClass::Other(0x2f)),
    ];
    for (esr, class) in cases {
        assert_eq!(
            exception_class(esr),
            class,
            "ESR {esr:#010x}: EC is bits 31..=26"
        );
    }
    assert_eq!(
        exception_class(0xffff_ffff_5600_0000),
        ExceptionClass::Svc,
        "bits above 31 are not part of the class"
    );
}

#[test]
fn svc_number_is_the_low_16_bits_of_an_svc_syndrome() {
    assert_eq!(svc_number(0x5600_0042), Some(0x42), "svc #0x42");
    assert_eq!(
        svc_number(0x5600_ffff),
        Some(0xffff),
        "the immediate is 16 bits wide"
    );
    assert_eq!(svc_number(0x5600_0000), Some(0), "svc #0");
    assert_eq!(svc_number(0x9600_0042), None, "a data abort is not an SVC");
    assert_eq!(svc_number(0xf200_0042), None, "a BRK is not an SVC");
}
