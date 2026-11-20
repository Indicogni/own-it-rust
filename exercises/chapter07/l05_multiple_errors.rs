//! LESSON 05 — Multiple error types at a boundary
//! Run: cargo test --test chapter07 l05_
//! Read first: CONCEPTS.md, section "Lesson 05".
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. How can one Result carry both an I/O error and an integer parse error?
//! 2. What does Box<dyn Error> hide from the compiler that an enum would show?
//!
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! EXPLAIN afterward: Explain ? conversion into a boxed error and compare a
//! concrete error enum with a universal application-boundary error.
//!
//! OPTIONAL LAB: LABS.md, lab 4 (the anyhow crate in a separate package; it
//! needs network access once). Do not add dependencies to this package.

use std::error::Error;
use std::io::Read;
type AnyResult<T> = Result<T, Box<dyn Error>>;
/// A. Read the whole input with `Read::read_to_string`, then parse the trimmed
/// text as u32. Propagate both kinds of failure with ? into Box<dyn Error>;
/// do not turn them into strings. Because read_to_string checks the encoding
/// for you, input that is not valid UTF-8 comes back as an io::Error (not as
/// a FromUtf8Error), and the tests expect exactly that.
fn read_count(mut reader: impl Read) -> AnyResult<u32> {
    todo!("Implement this task")
}
/// B. For either success or error, return a stable category: "ok", "io",
/// "parse", or "other", recognizing concrete boxed error types by downcast.
/// "parse" means ParseIntError only.
fn category(result: &AnyResult<u32>) -> &'static str {
    todo!("Implement this task")
}
#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    use practice::guess;
    use practice::within;
    use std::io::{self, ErrorKind};
    use std::num::ParseIntError;
    struct Broken;
    impl Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::new(ErrorKind::PermissionDenied, "blocked"))
        }
    }
    /// Named in the failure message when a test below panics or never ends.
    const READING: &str =
        "read_count [if it never returns: a read loop must stop when read gives Ok(0)]";
    // The tests run inside `within`, so a read loop that never ends fails
    // after a few seconds instead of hanging the whole suite. All assertions
    // sit inside the closure: a Box<dyn Error> cannot be handed back to the
    // test thread, because it is not Send (lesson 09 explains why).
    #[test]
    fn a_boundary_preserves_sources() {
        within(5, READING, || {
            assert_eq!(read_count(" 17 ".as_bytes()).unwrap(), 17);
            assert!(
                read_count("bad".as_bytes())
                    .unwrap_err()
                    .is::<ParseIntError>(),
                "the boxed error must still be the ParseIntError, not a String made from it"
            );
            assert!(
                read_count(Broken).unwrap_err().is::<io::Error>(),
                "the boxed error must still be the reader's io::Error"
            );
            assert_eq!(read_count("4294967295\n".as_bytes()).unwrap(), u32::MAX);
            assert!(
                read_count("4294967296".as_bytes())
                    .unwrap_err()
                    .is::<ParseIntError>(),
                "u32 overflow is a ParseIntError too"
            );
        });
    }
    #[test]
    fn a_io_error_keeps_its_kind() {
        within(5, READING, || {
            let err = read_count(Broken).unwrap_err();
            let io = err
                .downcast_ref::<io::Error>()
                .expect("the reader's io::Error, boxed unchanged");
            assert_eq!(
                io.kind(),
                ErrorKind::PermissionDenied,
                "the original io::Error must be boxed, not a new one"
            );
            let invalid_utf8: &[u8] = &[b'1', 0xff];
            assert!(
                read_count(invalid_utf8).unwrap_err().is::<io::Error>(),
                "read_to_string reports invalid UTF-8 as an io::Error"
            );
        });
    }
    #[test]
    fn b_downcast_identifies_error_families() {
        within(5, READING, || {
            assert_eq!(category(&Ok(1)), "ok");
            assert_eq!(category(&read_count(Broken)), "io");
            assert_eq!(category(&read_count("bad".as_bytes())), "parse");
            let other: AnyResult<u32> = Err("custom boundary failure".into());
            assert_eq!(category(&other), "other");
            let invalid_utf8: &[u8] = &[0xc3];
            assert_eq!(
                category(&read_count(invalid_utf8)),
                "io",
                "invalid UTF-8 must arrive as an io::Error: read with read_to_string"
            );
            let float: AnyResult<u32> = Err(Box::new("x".parse::<f64>().unwrap_err()));
            assert_eq!(
                category(&float),
                "other",
                "a ParseFloatError is not a ParseIntError"
            );
        });
    }

    fn boxed(text: &str) -> AnyResult<u32> {
        Ok(text.parse::<u32>()?)
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. How many machine words does a Box<dyn Error> take on the stack?
        let p1: usize = guess();
        assert_eq!(
            p1 * size_of::<usize>(),
            size_of::<Box<dyn Error>>(),
            "P1: a trait-object pointer is fat"
        );

        // P2. Heap allocations made by `boxed` on success, and on a parse error.
        let (good, ok_allocations) = count(|| boxed("12"));
        let (bad, err_allocations) = count(|| boxed("x"));
        assert!(good.is_ok() && bad.is_err());
        let p2: (usize, usize) = guess();
        assert_eq!(
            p2,
            (ok_allocations, err_allocations),
            "P2: what ? into Box<dyn Error> costs"
        );

        // P3. Which io::ErrorKind does read_to_string report for bytes that
        // are not valid UTF-8?
        let mut text = String::new();
        let mut bytes: &[u8] = &[0xff];
        let error = bytes.read_to_string(&mut text).unwrap_err();
        let p3: ErrorKind = guess();
        assert_eq!(p3, error.kind(), "P3: invalid UTF-8 is an I/O error");
    }
}
