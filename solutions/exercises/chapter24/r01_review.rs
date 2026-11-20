//! REVIEW 01 — Spaced retrieval: Chapters 18, 22, and 23 inside Chapter 24
//! Run: cargo test --test chapter24 r01_
//!
//! Do this WITHOUT reopening Chapters 18, 22, or 23 or their hints. Struggling
//! to recall is what makes the knowledge stick; look things up only after a
//! real attempt, and note in NOTES.md anything you had to look up.
//!
//! Each task mixes an earlier idea (uninitialized memory, macro control flow,
//! byte-level I/O) with this chapter's foreign-boundary rules.

use std::ffi::{c_char, c_int, c_uint};
use std::io::{self, Read, Write};

use sensor::{AVAILABLE, Sample, foreign_fill};

// Supplied mock library: keep it unchanged.
mod sensor {
    use std::ffi::c_int;
    use std::num::NonZero;

    pub const AVAILABLE: usize = 5;

    /// One reading. Only the library can make a `Sample`: its field is private
    /// to this module, and it has no constructor, no `Default` and no `Clone`.
    /// Zero is not a valid reading, so an all-zero `Sample` does not exist.
    #[repr(transparent)]
    #[derive(Debug, PartialEq)]
    pub struct Sample(NonZero<c_int>);

    impl Sample {
        pub fn value(&self) -> c_int {
            self.0.get()
        }
    }

    // Writes min(cap, AVAILABLE) readings (10, 20, 30, ...) to the start of
    // `out` and returns how many it wrote.
    // # Safety: out is valid for writing `cap` Samples. No pointer is retained.
    pub unsafe extern "C" fn foreign_fill(out: *mut Sample, cap: usize) -> usize {
        let n = cap.min(AVAILABLE);
        for (i, value) in (0..n).zip((10..).step_by(10)) {
            // The readings are never zero; stopping early keeps this function
            // free of panics, which must not cross an extern "C" boundary.
            let Some(value) = NonZero::new(value) else {
                return i;
            };
            // SAFETY: i < n <= cap, so the slot is inside the caller's buffer.
            unsafe { out.add(i).write(Sample(value)) };
        }
        n
    }
}

/// A. (Ch23 uninitialized memory + Ch24 out-buffers) Allocate ONE Vec with
/// capacity `max`, let `foreign_fill` write into its spare capacity, then set
/// the length to the reported count, after checking that it is <= max.
/// You cannot fill the buffer with placeholder values first, because there is
/// no way to make a `Sample` yourself. Never expose an element that the
/// library did not write.
fn read_samples(max: usize) -> Vec<Sample> {
    let mut samples = Vec::with_capacity(max);
    // SAFETY: the Vec owns room for `max` Samples starting at as_mut_ptr, and
    // foreign_fill writes at most `max` of them and keeps no pointer.
    let written = unsafe { foreign_fill(samples.as_mut_ptr(), max) };
    // Never trust a count from the other side of the boundary without a check.
    assert!(
        written <= max,
        "the library reported more than it was offered"
    );
    // SAFETY: the first `written` elements were initialized by foreign_fill,
    // and `written` is within the capacity.
    unsafe { samples.set_len(written) };
    samples
}

/// B. (Ch22 macro control flow + Ch24 status codes) Evaluate a c_int status
/// expression exactly once. If it is non-zero, `return Err(status)` from the
/// ENCLOSING function; otherwise the macro evaluates to `()`. Like `?` for C.
macro_rules! ffi_try {
    ($status:expr) => {{
        // Binding first evaluates the expression exactly once.
        let status: c_int = $status;
        if status != 0 {
            return Err(status);
        }
    }};
}

#[repr(C)]
#[derive(Debug, PartialEq)]
struct Reading {
    kind: c_char,
    count: c_uint,
    value: c_int,
}

/// C. (Ch18 I/O + Ch24 layout) Write `r` as a fixed 9-byte little-endian
/// record: kind (1 byte), count (4), value (4). Do not dump the struct's
/// memory: it has padding and the target's byte order. Propagate I/O errors.
fn write_record(out: &mut impl Write, r: &Reading) -> io::Result<()> {
    let mut bytes = [0_u8; 9];
    bytes[0] = r.kind.to_le_bytes()[0];
    bytes[1..5].copy_from_slice(&r.count.to_le_bytes());
    bytes[5..9].copy_from_slice(&r.value.to_le_bytes());
    out.write_all(&bytes)
}

/// C. Read one 9-byte record written by `write_record`. A short input is an
/// error (UnexpectedEof), never a partial Reading.
fn read_record(input: &mut impl Read) -> io::Result<Reading> {
    let mut bytes = [0_u8; 9];
    input.read_exact(&mut bytes)?;
    let [kind, c0, c1, c2, c3, v0, v1, v2, v3] = bytes;
    Ok(Reading {
        kind: c_char::from_le_bytes([kind]),
        count: c_uint::from_le_bytes([c0, c1, c2, c3]),
        value: c_int::from_le_bytes([v0, v1, v2, v3]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    use std::cell::RefCell;
    use std::io::Cursor;

    fn values(samples: &[Sample]) -> Vec<c_int> {
        samples.iter().map(Sample::value).collect()
    }

    #[test]
    fn a_samples_come_from_the_library() {
        assert_eq!(values(&read_samples(3)), [10, 20, 30]);
        assert_eq!(
            values(&read_samples(8)),
            [10, 20, 30, 40, 50],
            "the length is what the library reported, not the capacity you offered"
        );
        assert!(read_samples(0).is_empty());
    }

    #[test]
    fn a_samples_use_one_allocation() {
        let (samples, allocations) = count(|| read_samples(64));
        assert_eq!(
            samples.len(),
            AVAILABLE,
            "the length must be the count that foreign_fill returned"
        );
        assert!(
            samples.capacity() >= 64,
            "the library must write into a Vec that was created with capacity `max`"
        );
        assert!(
            allocations <= 1,
            "one buffer; got {allocations} allocations"
        );
    }

    fn step(log: &RefCell<Vec<&'static str>>, name: &'static str, status: c_int) -> c_int {
        log.borrow_mut().push(name);
        status
    }

    fn transaction(
        log: &RefCell<Vec<&'static str>>,
        write_status: c_int,
    ) -> Result<&'static str, c_int> {
        ffi_try!(step(log, "open", 0));
        ffi_try!(step(log, "write", write_status));
        ffi_try!(step(log, "close", 0));
        Ok("done")
    }

    #[test]
    fn b_success_runs_every_step_once() {
        let log = RefCell::new(Vec::new());
        assert_eq!(transaction(&log, 0), Ok("done"));
        assert_eq!(*log.borrow(), ["open", "write", "close"]);
    }

    #[test]
    fn b_failure_returns_early_from_the_caller() {
        let log = RefCell::new(Vec::new());
        assert_eq!(transaction(&log, -5), Err(-5));
        assert_eq!(*log.borrow(), ["open", "write"], "close never runs");
        let log = RefCell::new(Vec::new());
        assert_eq!(transaction(&log, 3), Err(3), "any non-zero status fails");
    }

    #[test]
    fn c_record_bytes_are_explicit_little_endian() {
        let r = Reading {
            kind: 0x41,
            count: 0x0102_0304,
            value: -2,
        };
        let mut bytes = Vec::new();
        write_record(&mut bytes, &r).unwrap();
        assert_eq!(bytes, [0x41, 4, 3, 2, 1, 0xfe, 0xff, 0xff, 0xff]);
        assert_ne!(bytes.len(), size_of::<Reading>(), "no padding on the wire");
        assert_eq!(read_record(&mut Cursor::new(bytes)).unwrap(), r);
    }

    // A hand-written loop around `read` or `write` never ends when the call
    // keeps returning Ok(0), so these two checks run under a time limit.
    #[test]
    fn c_errors_propagate_in_both_directions() {
        let short = practice::within(5, "read_record on a 3-byte input", || {
            read_record(&mut Cursor::new([1_u8, 2, 3])).unwrap_err()
        });
        assert_eq!(
            short.kind(),
            io::ErrorKind::UnexpectedEof,
            "a short input is an error, not a partial Reading"
        );
        let full = practice::within(5, "write_record into a 4-byte buffer", || {
            let r = Reading {
                kind: 0,
                count: 0,
                value: 0,
            };
            let mut tiny = [0_u8; 4];
            write_record(&mut &mut tiny[..], &r).unwrap_err()
        });
        assert_eq!(
            full.kind(),
            io::ErrorKind::WriteZero,
            "a writer that cannot take all 9 bytes must give an error"
        );
    }

    #[test]
    fn c_writing_into_a_reserved_buffer_does_not_allocate() {
        let r = Reading {
            kind: 1,
            count: 2,
            value: 3,
        };
        let mut out = Vec::with_capacity(64);
        let (result, allocations) = count(|| write_record(&mut out, &r));
        assert!(result.is_ok());
        assert_eq!(out.len(), 9);
        assert_eq!(allocations, 0, "no temporary buffers");
    }

    /// P. Chapters 18–23 recall: predict each value BEFORE running.
    #[test]
    fn p_predictions() {
        // P1. How many times is "kept" dropped by the end of this block?
        let log = RefCell::new(Vec::new());
        struct Noisy<'a>(&'a RefCell<Vec<&'static str>>);
        impl Drop for Noisy<'_> {
            fn drop(&mut self) {
                self.0.borrow_mut().push("kept");
            }
        }
        {
            let _kept = std::mem::ManuallyDrop::new(Noisy(&log));
        }
        let p1: usize = 0;
        assert_eq!(p1, log.borrow().len(), "P1: ManuallyDrop");

        // P2. How many token trees are in `f(a, b) [c]`?
        macro_rules! count_tts {
            ($($t:tt)*) => { 0 $(+ { let _ = stringify!($t); 1 })* };
        }
        let p2: i32 = 3;
        assert_eq!(p2, count_tts!(f(a, b)[c]), "P2: token trees");

        // P3. Write "abc", seek back to 1, write "Z". What does the buffer hold?
        let mut cursor = Cursor::new(Vec::new());
        cursor.write_all(b"abc").unwrap();
        cursor.set_position(1);
        cursor.write_all(b"Z").unwrap();
        let p3: &[u8] = b"aZc";
        assert_eq!(p3, cursor.get_ref().as_slice(), "P3: Cursor overwrites");
    }
}
