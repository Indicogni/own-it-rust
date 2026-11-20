//! LESSON 01 — A safe invariant boundary
//! Run: cargo test --test chapter23 l01_
//! Read first: "Lesson 01" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. Does `unsafe` switch off borrow checking around the unsafe operation?
//! 2. Is every sequence of ASCII bytes valid UTF-8, including the byte zero?
//!
//! IMPLEMENT the tasks below. Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: State the private invariant, identify each unsafe
//! operation, and explain why all public operations preserve the invariant.
//!
//! COMPILER / TRANSFER LAB (optional): Put a copy of `Ascii` in a module and
//! try to build `Ascii(vec![200])` from outside that module. Read the error
//! about the private field. Then compare the checked `String::from_utf8` with
//! the unchecked conversion on valid input only; never call the unchecked
//! function on invalid data.
//! Use examples/ch23_scratch.rs for this, and leave it compiling afterward.

#[derive(Debug)]
struct Ascii(Vec<u8>); // invariant: every byte <= 0x7f
impl Ascii {
    /// A. Validate once; return the original bytes unchanged on rejection.
    fn new(bytes: Vec<u8>) -> Result<Self, Vec<u8>> {
        // `is_ascii` only borrows the bytes, so the same Vec can be moved into
        // the wrapper or handed back: no copy on either path.
        if bytes.is_ascii() {
            Ok(Self(bytes))
        } else {
            Err(bytes)
        }
    }
    /// B. Borrow the text without allocation. Practice a minimal unsafe block
    /// with a SAFETY comment deriving UTF-8 validity from the private invariant.
    fn as_str(&self) -> &str {
        // SAFETY: `from_utf8_unchecked` requires valid UTF-8. The invariant of
        // `Ascii` says every byte is <= 0x7f, and every such byte is a complete
        // one-byte UTF-8 sequence. The invariant holds because the field is
        // private, `new` checks it, and `lowercase` maps ASCII to ASCII.
        unsafe { std::str::from_utf8_unchecked(&self.0) }
    }
    fn lowercase(&mut self) {
        self.0.make_ascii_lowercase();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    #[test]
    fn a_b_validation_and_mutation_preserve_invariant() {
        let mut s = Ascii::new(b"A\0Z".to_vec()).unwrap();
        assert_eq!(s.as_str(), "A\0Z");
        s.lowercase();
        assert_eq!(s.as_str(), "a\0z");
        assert_eq!(
            s.as_str().as_ptr(),
            s.0.as_ptr(),
            "as_str must borrow the stored bytes, not build a new string"
        );
        assert_eq!(Ascii::new(vec![255]).unwrap_err(), [255]);
        assert_eq!(Ascii::new(vec![]).unwrap().as_str(), "");
    }
    #[test]
    fn a_boundary_bytes() {
        assert_eq!(Ascii::new(vec![0x7f]).unwrap().as_str(), "\u{7f}");
        assert_eq!(
            Ascii::new(vec![b'o', b'k', 0x80]).unwrap_err(),
            [b'o', b'k', 0x80]
        );
        let e_acute = "é".as_bytes().to_vec();
        assert_eq!(
            Ascii::new(e_acute).unwrap_err(),
            "é".as_bytes(),
            "valid UTF-8 is not enough"
        );
    }
    #[test]
    fn a_rejection_returns_the_same_buffer() {
        let bad = vec![b'o', b'k', 0xff];
        let p = bad.as_ptr();
        let (result, allocations) = count(|| Ascii::new(bad));
        let back = result.unwrap_err();
        assert_eq!(back.as_ptr(), p, "give back the caller's Vec");
        assert_eq!(
            allocations, 0,
            "a rejected Vec is returned as it is: no copy, no new buffer"
        );
    }
    #[test]
    fn a_b_construction_and_borrowing_do_not_allocate() {
        let bytes = b"plain ascii".to_vec();
        let p = bytes.as_ptr();
        let (s, allocations) = count(|| Ascii::new(bytes).unwrap());
        assert_eq!(allocations, 0, "validate in place; do not copy");
        let (len, allocations) = count(|| s.as_str().len());
        assert_eq!(
            (len, allocations),
            (11, 0),
            "(length, allocations): as_str borrows, so it never allocates"
        );
        assert_eq!(
            s.as_str().as_ptr(),
            p,
            "the wrapper must keep the caller's buffer, and as_str must point into it"
        );
    }
    #[test]
    fn b_technique_as_str_contains_the_unsafe_step() {
        let body = crate::source::body_of(include_str!("l01_ascii.rs"), "fn as_str");
        assert!(
            body.contains("unsafe"),
            "task B asks for an unsafe conversion that relies on the invariant. A checked \
             `str::from_utf8(..).unwrap()` gives the same text, but it validates the bytes a \
             second time and skips the point of the lesson"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    /// In every failure message, `left` is your prediction and `right` is what
    /// the program computed.
    #[test]
    fn p_predictions() {
        // P1. Which of these byte strings are ASCII?
        let p1: (bool, bool, bool) = (true, true, false);
        assert_eq!(
            p1,
            (
                b"A\0Z".is_ascii(),
                [0x7f_u8].is_ascii(),
                [0x80_u8].is_ascii()
            ),
            "P1: the ASCII range"
        );

        // P2. String::from_utf8 rejects [0xC3]. Does the error hand back the
        // same heap buffer it was given?
        let bytes = vec![0xC3_u8];
        let p = bytes.as_ptr();
        let back = String::from_utf8(bytes).unwrap_err().into_bytes();
        let p2: bool = true;
        assert_eq!(p2, back.as_ptr() == p, "P2: what a rejection returns");

        // P3. Size of the Ascii wrapper, in bytes, on a 64-bit target.
        let p3: usize = 24;
        assert_eq!(p3, size_of::<Ascii>(), "P3: newtype size");

        // P4. ASCII lowercasing of "ÀB".
        let mut text = String::from("ÀB");
        text.make_ascii_lowercase();
        let p4: &str = "Àb";
        assert_eq!(p4, text, "P4: make_ascii_lowercase");
    }
}
