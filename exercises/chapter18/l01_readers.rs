//! LESSON 01 — Streaming bytes
//! Run: cargo test --test chapter18 l01_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider:
//! 1. Can read return fewer bytes than requested before EOF?
//! 2. Does read_to_string accept arbitrary bytes?
//!
//! IMPLEMENT the tasks; keep the tests.
//! EXPLAIN: Why use Read rather than File in the contract? Explain EOF versus
//! Interrupted.
//! COMPILER LAB (examples/ch18_scratch.rs): Implement a Read wrapper that returns
//! one byte per call. Compare read, read_exact, take, and chain.

use std::io::{self, Read, Write};
/// A. Copy all bytes with a fixed 4-byte scratch buffer. Retry Interrupted;
/// propagate all other errors. Use write_all, not one write per chunk.
fn copy_stream<R: Read, W: Write>(mut input: R, mut output: W) -> io::Result<u64> {
    todo!("A: implement the contract")
}
/// B. Read at most limit bytes without consuming the next byte.
fn prefix<R: Read>(input: &mut R, limit: u64) -> io::Result<Vec<u8>> {
    todo!("B: implement the contract")
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::{guess, within};

    // The tests run your code through `within`, so a loop that never ends
    // fails with a message after a few seconds instead of hanging the suite.
    const COPY: &str = "copy_stream (does the loop stop when read returns Ok(0), \
                        and retry only Interrupted?)";
    const PREFIX: &str = "prefix (does it stop at end of input?)";

    /// Hands out one byte per call, then EOF.
    struct Drip<'a>(&'a [u8]);
    impl Read for Drip<'_> {
        fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
            if self.0.is_empty() || b.is_empty() {
                return Ok(0);
            }
            b[0] = self.0[0];
            self.0 = &self.0[1..];
            Ok(1)
        }
    }

    #[test]
    fn a_copy_handles_binary_short_reads_and_interruptions() {
        struct Source {
            first: bool,
            rest: &'static [u8],
        }
        impl Read for Source {
            fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
                if self.first {
                    self.first = false;
                    return Err(io::ErrorKind::Interrupted.into());
                }
                if self.rest.is_empty() {
                    return Ok(0);
                }
                b[0] = self.rest[0];
                self.rest = &self.rest[1..];
                Ok(1)
            }
        }
        struct Broken;
        impl Read for Broken {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Err(io::ErrorKind::PermissionDenied.into())
            }
        }
        let (copied, out, from_empty, broken) = within(5, COPY, || {
            let mut out = Vec::new();
            let source = Source {
                first: true,
                rest: b"a\0\xffz",
            };
            let copied = copy_stream(source, &mut out).unwrap();
            let from_empty = copy_stream(&b""[..], Vec::new()).unwrap();
            let broken = copy_stream(Broken, Vec::new()).unwrap_err().kind();
            (copied, out, from_empty, broken)
        });
        assert_eq!(copied, 4, "the result counts every byte copied");
        assert_eq!(out, b"a\0\xffz", "bytes that are not text must survive");
        assert_eq!(from_empty, 0, "an empty input copies nothing");
        assert_eq!(
            broken,
            io::ErrorKind::PermissionDenied,
            "only Interrupted is retried; every other read error is returned"
        );
    }
    #[test]
    fn a_copy_uses_a_small_buffer_and_write_all() {
        /// Remembers the largest buffer it was asked to fill.
        struct Spy {
            data: &'static [u8],
            largest: usize,
        }
        impl Read for Spy {
            fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
                self.largest = self.largest.max(b.len());
                let n = b.len().min(self.data.len());
                b[..n].copy_from_slice(&self.data[..n]);
                self.data = &self.data[n..];
                Ok(n)
            }
        }
        /// Accepts one byte per write call.
        struct Narrow(Vec<u8>);
        impl Write for Narrow {
            fn write(&mut self, b: &[u8]) -> io::Result<usize> {
                let n = b.len().min(1);
                self.0.extend_from_slice(&b[..n]);
                Ok(n)
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let (copied, written, largest) = within(5, COPY, || {
            let mut spy = Spy {
                data: b"0123456789",
                largest: 0,
            };
            let mut out = Narrow(vec![]);
            let copied = copy_stream(&mut spy, &mut out).unwrap();
            (copied, out.0, spy.largest)
        });
        assert_eq!(copied, 10, "the result counts every byte copied");
        assert_eq!(written, b"0123456789", "short writes must be retried");
        assert_eq!(largest, 4, "use the fixed 4-byte scratch buffer");
    }
    #[test]
    fn a_copy_propagates_write_errors() {
        struct Closed;
        impl Write for Closed {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let kind = within(5, COPY, || {
            copy_stream(&b"data"[..], Closed).unwrap_err().kind()
        });
        assert_eq!(
            kind,
            io::ErrorKind::BrokenPipe,
            "a failed write ends the copy with that error"
        );
    }
    #[test]
    fn b_prefix_leaves_remainder() {
        within(5, PREFIX, || {
            let mut r = &b"abcdef"[..];
            assert_eq!(prefix(&mut r, 2).unwrap(), b"ab");
            assert_eq!(r, b"cdef", "the bytes after the limit stay in the reader");
            assert_eq!(prefix(&mut r, 0).unwrap(), b"");
            assert_eq!(prefix(&mut r, 10).unwrap(), b"cdef");
            assert!(r.is_empty(), "a limit past the end reads everything");
        });
    }
    #[test]
    fn b_prefix_survives_short_reads_and_huge_limits() {
        within(5, PREFIX, || {
            let mut d = Drip(b"xyz!");
            assert_eq!(prefix(&mut d, 3).unwrap(), b"xyz", "one read may be short");
            assert_eq!(d.0, b"!", "the byte after the limit must not be consumed");
            let mut r = &b"tail"[..];
            assert_eq!(
                prefix(&mut r, u64::MAX).unwrap(),
                b"tail",
                "don't preallocate the limit"
            );
        });
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. One read from a 6-byte slice into a 4-byte buffer: bytes read,
        // and bytes left in the slice.
        let mut r: &[u8] = b"abcdef";
        let mut buf = [0; 4];
        let n = r.read(&mut buf).unwrap();
        let p1: (usize, usize) = guess();
        assert_eq!(p1, (n, r.len()), "P1: a slice reader advances");

        // P2. At EOF: what does read return, and which error does read_exact give?
        let mut empty: &[u8] = b"";
        let p2: (usize, io::ErrorKind) = guess();
        assert_eq!(
            p2,
            (
                empty.read(&mut buf).unwrap(),
                empty.read_exact(&mut buf).unwrap_err().kind()
            ),
            "P2: EOF"
        );

        // P3. chain two readers, then take(3).
        let mut text = String::new();
        (&b"ab"[..])
            .chain(&b"cd"[..])
            .take(3)
            .read_to_string(&mut text)
            .unwrap();
        let p3: &str = guess();
        assert_eq!(p3, text, "P3: chain + take");

        // P4. read_to_string on bytes that are not UTF-8: which error kind?
        let mut sink = String::new();
        let kind = (&b"\xff"[..]).read_to_string(&mut sink).unwrap_err().kind();
        let p4: io::ErrorKind = guess();
        assert_eq!(p4, kind, "P4: text readers validate UTF-8");
    }
}
