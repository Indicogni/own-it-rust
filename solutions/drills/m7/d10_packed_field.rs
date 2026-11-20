//! DRILL M7-10 · A reference into a packed struct
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0793
//!
//! Run `cargo drill m7`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: at which address does `value` start, and what does a `&u32` promise about its address?

// FIX: in a packed struct `value` starts at offset 1. A `&u32` must point to
// an address that is a multiple of 4, so the compiler rejects the reference
// (E0793). Copy the field out instead. When a pointer is really needed, `&raw
// const` with `read_unaligned` is the tool.
// A record of a file format, without padding.
#[repr(C, packed)] // KEEP
struct Record {
    tag: u8,
    value: u32,
}

fn main() {
    let record = Record {
        tag: 1,
        value: 0x0102_0304,
    };
    assert_eq!(size_of::<Record>(), 5); // KEEP
    let value = record.value; // a copy, not a reference
    let total = u64::from(record.tag) + u64::from(value);
    assert_eq!(total, 0x0102_0305); // KEEP
}
