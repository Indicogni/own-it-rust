//! LESSON 07 — Capstone: a bounded record service
//! Run: cargo test --test chapter18 l07_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider:
//! 1. What happens if an attacker declares an oversized payload?
//! 2. Can one frame contain arbitrary non-UTF-8 bytes?
//!
//! IMPLEMENT the tasks; keep the tests.
//! EXPLAIN: Explain framing, length validation before allocation, error
//! propagation, and when the response is flushed.
//! COMPILER LAB (examples/ch18_scratch.rs): Write a reader that hands out one byte
//! per call and run handle_one through it, so the request header arrives in two
//! pieces. Then connect handle_one to the lesson 8 listener instead of slices.

use std::io::{self, Read, Write};
/// A. Read one u16-LE length-prefixed frame. Reject lengths above max with
/// InvalidData before allocating/reading its body. Require all declared bytes.
fn receive<R: Read>(mut input: R, max: u16) -> io::Result<Vec<u8>> {
    let mut header = [0u8; 2];
    input.read_exact(&mut header)?;
    let len = u16::from_le_bytes(header);
    // Decide before the buffer exists: the peer chose this number.
    if len > max {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "declared frame length is above the limit",
        ));
    }
    let mut body = vec![0u8; usize::from(len)];
    input.read_exact(&mut body)?;
    Ok(body)
}
/// B. Receive one frame, reverse its bytes (not Unicode characters), send the
/// same framing, then flush. No output may be written if receiving fails.
fn handle_one<R: Read, W: Write>(input: R, mut output: W, max: u16) -> io::Result<()> {
    // Nothing is written until the whole request has arrived and passed the
    // checks: `?` returns before the first write.
    let mut body = receive(input, max)?;
    body.reverse();
    // The body came from a u16 header, so its length fits a u16 again.
    let len = u16::try_from(body.len()).expect("received length fits in u16");
    output.write_all(&len.to_le_bytes())?;
    output.write_all(&body)?;
    output.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::within;

    // The tests run your code through `within`, so a hand-written read loop
    // that keeps asking a finished reader for more fails instead of hanging.
    const RECEIVE: &str = "receive (does a hand-written read loop stop at end of input?)";
    const HANDLE: &str = "handle_one (does a hand-written read loop stop at end of input?)";

    #[test]
    fn a_receive_rejects_an_oversized_length_before_reading_the_body() {
        within(5, RECEIVE, || {
            let mut input = &b"\x05\0hello"[..];
            assert_eq!(
                receive(&mut input, 4).unwrap_err().kind(),
                io::ErrorKind::InvalidData,
                "a declared length of 5 is above max = 4"
            );
            assert_eq!(input, b"hello", "only the header may be consumed");
            let mut huge = &b"\xff\xff"[..];
            assert_eq!(
                receive(&mut huge, 100).unwrap_err().kind(),
                io::ErrorKind::InvalidData,
                "the length is checked first, so the missing body is never noticed"
            );
        });
    }
    #[test]
    fn a_receive_validates_the_length_before_allocating() {
        // The header declares 65535 bytes and the limit is 100.
        let (kind, largest) = within(5, RECEIVE, || {
            let (result, largest) = crate::largest_allocation(|| receive(&b"\xff\xff"[..], 100));
            (result.unwrap_err().kind(), largest)
        });
        assert_eq!(kind, io::ErrorKind::InvalidData);
        assert!(
            largest < 4096,
            "rejecting an oversized frame allocated {largest} bytes in one piece: \
             compare the declared length with max before you create the body buffer"
        );
    }
    #[test]
    fn a_receive_requires_every_declared_byte() {
        within(5, RECEIVE, || {
            assert_eq!(
                receive(&b"\x02\0hi"[..], 2).unwrap(),
                b"hi",
                "max is inclusive"
            );
            for bytes in [&b"\x03\0ab"[..], &b"\x03"[..], &b""[..]] {
                assert_eq!(
                    receive(bytes, 8).unwrap_err().kind(),
                    io::ErrorKind::UnexpectedEof,
                    "{bytes:?}"
                );
            }
        });
    }
    #[test]
    fn b_service_transforms_one_frame_and_retains_next() {
        within(5, HANDLE, || {
            let mut input = &b"\x03\0a\xffbTAIL"[..];
            let mut out = vec![];
            handle_one(&mut input, &mut out, 8).unwrap();
            assert_eq!(out, b"\x03\0b\xffa", "same framing, payload bytes reversed");
            assert_eq!(input, b"TAIL", "bytes after the frame stay in the reader");
        });
    }
    #[test]
    fn b_invalid_request_produces_no_response() {
        within(5, HANDLE, || {
            for (bytes, kind) in [
                (&b"\x05\0"[..], io::ErrorKind::InvalidData),
                (&b"\x02\0x"[..], io::ErrorKind::UnexpectedEof),
            ] {
                let mut out = vec![];
                assert_eq!(handle_one(bytes, &mut out, 4).unwrap_err().kind(), kind);
                assert!(out.is_empty(), "a failed request must not produce output");
            }
        });
    }
    #[test]
    fn b_empty_frame_is_valid() {
        within(5, HANDLE, || {
            let mut out = vec![];
            handle_one(&b"\0\0"[..], &mut out, 0).unwrap();
            assert_eq!(out, b"\0\0", "an empty payload still gets a header");
        });
    }
    #[test]
    fn b_response_survives_short_writes_and_is_flushed_once() {
        struct Narrow {
            bytes: Vec<u8>,
            flushed: usize,
        }
        impl Write for Narrow {
            fn write(&mut self, b: &[u8]) -> io::Result<usize> {
                let n = b.len().min(1);
                self.bytes.extend_from_slice(&b[..n]);
                Ok(n)
            }
            fn flush(&mut self) -> io::Result<()> {
                self.flushed += 1;
                Ok(())
            }
        }
        let w = within(5, HANDLE, || {
            let mut w = Narrow {
                bytes: vec![],
                flushed: 0,
            };
            // "é!" is the three bytes C3 A9 21.
            handle_one(&b"\x03\0\xc3\xa9!"[..], &mut w, 8).unwrap();
            w
        });
        assert_eq!(
            w.bytes,
            [3, 0, b'!', 0xa9, 0xc3],
            "bytes reversed, not chars"
        );
        assert_eq!(w.flushed, 1, "flush exactly once, after the whole response");
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. Two complete frames sit in a byte slice. One read into a 64-byte
        // buffer: how many bytes does it return, and how many are left?
        let mut wire: &[u8] = b"\x01\0a\x01\0b";
        let mut buf = [0u8; 64];
        let n = wire.read(&mut buf).unwrap();
        let p1: (usize, usize) = (6, 0);
        assert_eq!(p1, (n, wire.len()), "P1: a read does not stop at a frame");

        // P2. The largest length a 2-byte header can declare, and the length
        // declared by the header bytes 00 01 in this little-endian format.
        let p2: (u16, u16) = (65535, 256);
        assert_eq!(
            p2,
            (
                u16::from_le_bytes([0xff, 0xff]),
                u16::from_le_bytes([0x00, 0x01])
            ),
            "P2: header values"
        );

        // P3. Reverse the bytes of "é!" (C3 A9 21): is the result still UTF-8?
        // And what does reversing its chars give?
        let mut bytes = "é!".as_bytes().to_vec();
        bytes.reverse();
        let by_chars: String = "é!".chars().rev().collect();
        let p3: (bool, &str) = (false, "!é");
        assert_eq!(
            p3,
            (String::from_utf8(bytes).is_ok(), by_chars.as_str()),
            "P3: bytes are not characters"
        );
    }
}
