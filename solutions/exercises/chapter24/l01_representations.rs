//! LESSON 01 — C representations
//! Run: cargo test --test chapter24 l01_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: Does repr(C) make a Rust String field C-compatible? Is C long guaranteed the same size as Rust i64 on every target?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! EXPLAIN: Explain primitive aliases, field layout, padding, and why a raw struct byte dump is not a portable wire format.
//! COMPILER LAB (examples/ch24_scratch.rs): Compare c_int/c_long/c_char sizes to Rust primitives on this platform, then inspect the same C struct with a tiny C sizeof/offsetof program in the lab.

use std::ffi::{c_char, c_int, c_uint};
#[repr(C)]
#[derive(Debug, PartialEq)]
struct Reading {
    kind: c_char,
    count: c_uint,
    value: c_int,
}
/// A. Construct a C-layout record using C aliases. No casts or assumptions about
/// signed char are required: accept exactly the field types in the signature.
fn reading(kind: c_char, count: c_uint, value: c_int) -> Reading {
    Reading { kind, count, value }
}
/// B. Return size, alignment, and the value field's byte offset for this target.
fn layout() -> (usize, usize, usize) {
    // Measured, never hard-coded: the numbers belong to the target.
    (
        size_of::<Reading>(),
        align_of::<Reading>(),
        std::mem::offset_of!(Reading, value),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::c_longlong;
    #[test]
    fn a_field_types_and_values_preserved() {
        assert_eq!(
            reading(1, 7, -9),
            Reading {
                kind: 1,
                count: 7,
                value: -9
            }
        );
        // 0x7f fits c_char whether it is signed (x86-64) or unsigned (ARM Linux).
        let extremes = reading(0x7f, c_uint::MAX, c_int::MIN);
        assert_eq!(
            (extremes.kind, extremes.count, extremes.value),
            (0x7f, c_uint::MAX, c_int::MIN)
        );
    }
    #[test]
    fn b_layout_accounts_for_alignment() {
        let (size, align, offset) = layout();
        assert_eq!(
            size,
            std::mem::size_of::<Reading>(),
            "first number: the size of Reading in bytes, padding included"
        );
        assert_eq!(
            align,
            std::mem::align_of::<Reading>(),
            "second number: the alignment of Reading in bytes"
        );
        assert_eq!(
            offset,
            std::mem::offset_of!(Reading, value),
            "third number: how many bytes into Reading the `value` field starts"
        );
        assert_eq!(
            offset % std::mem::align_of::<c_int>(),
            0,
            "a c_int field always starts at a multiple of c_int's alignment"
        );
        assert!(
            size >= offset + std::mem::size_of::<c_int>(),
            "the last field must fit inside the struct"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. Sizes of c_char, c_int, and c_longlong on Linux, macOS, and Windows.
        // (c_long is deliberately absent: it is 8 bytes on 64-bit Unix, 4 on Windows.)
        let p1: (usize, usize, usize) = (1, 4, 8);
        assert_eq!(
            p1,
            (
                size_of::<c_char>(),
                size_of::<c_int>(),
                size_of::<c_longlong>()
            ),
            "P1: C type aliases"
        );

        #[repr(C)]
        struct Loose {
            a: u8,
            b: u64,
            c: u8,
        }
        #[repr(C)]
        struct Tight {
            b: u64,
            a: u8,
            c: u8,
        }
        // P2. repr(C) keeps declaration order. Sizes of Loose and Tight?
        let p2: (usize, usize) = (24, 16);
        assert_eq!(
            p2,
            (size_of::<Loose>(), size_of::<Tight>()),
            "P2: field order and padding"
        );

        #[repr(C)]
        enum CStatus {
            Ok,
            Failed,
        }
        #[repr(u8)]
        enum ByteStatus {
            Ok,
            Failed,
        }
        // P3. Size of a fieldless repr(C) enum, and of a repr(u8) one.
        let p3: (usize, usize) = (4, 1);
        assert_eq!(
            p3,
            (size_of::<CStatus>(), size_of::<ByteStatus>()),
            "P3: enum representations"
        );
    }
}
