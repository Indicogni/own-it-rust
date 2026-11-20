//! LESSON 04 — Seeking and binary formats
//! Run: cargo test --test chapter18 l04_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider:
//! 1. Does seek change a Cursor backing vector length immediately?
//! 2. Is transmuting a struct a portable file format?
//!
//! IMPLEMENT the tasks; keep the tests.
//! EXPLAIN: Explain why field sizes and byte order belong to a format contract.
//! COMPILER LAB (examples/ch18_scratch.rs): Use Cursor<Vec<u8>> and SeekFrom::End,
//! Current, Start. Then wrap it in BufReader and compare stream_position.

use std::io::{self, Read, Seek, SeekFrom};
/// A. Read the u32 little-endian record that starts at byte index * 4. Seek
/// from the start. Compute the offset with a checked multiplication: an
/// offset that overflows u64 is an InvalidInput error. Propagate seek and
/// read errors; a missing or partial record is UnexpectedEof.
fn read_record<R: Read + Seek>(input: &mut R, index: u64) -> io::Result<u32> {
    let offset = index
        .checked_mul(4)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "record offset overflows"))?;
    input.seek(SeekFrom::Start(offset))?;
    let mut bytes = [0u8; 4];
    // read_exact reports a missing or partial record as UnexpectedEof.
    input.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}
/// B. Read a single frame in the lesson 3 format: a u16 little-endian length,
/// then that many payload bytes. Return the payload. A truncated header or
/// payload is UnexpectedEof. Leave any following frame unread.
fn read_frame<R: Read>(mut input: R) -> io::Result<Vec<u8>> {
    let mut header = [0u8; 2];
    input.read_exact(&mut header)?;
    let mut payload = vec![0u8; usize::from(u16::from_le_bytes(header))];
    input.read_exact(&mut payload)?;
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::within;
    use std::io::{Cursor, Write};

    // The tests run your code through `within`, so a hand-written fill loop
    // that keeps asking a finished reader for more fails instead of hanging.
    const RECORD: &str = "read_record (does a hand-written read loop stop at end of input?)";
    const FRAME: &str = "read_frame (does a hand-written read loop stop at end of input?)";

    #[test]
    fn a_random_access_has_explicit_endianness() {
        within(5, RECORD, || {
            let mut c = std::io::Cursor::new([1, 0, 0, 0, 0x78, 0x56, 0x34, 0x12]);
            assert_eq!(
                read_record(&mut c, 1).unwrap(),
                0x12345678,
                "bytes 78 56 34 12 are 0x12345678 in little endian"
            );
            assert_eq!(read_record(&mut c, 0).unwrap(), 1);
            assert_eq!(
                read_record(&mut c, 2).unwrap_err().kind(),
                io::ErrorKind::UnexpectedEof,
                "record 2 would start at the end of the data"
            );
            assert_eq!(
                read_record(&mut c, u64::MAX).unwrap_err().kind(),
                io::ErrorKind::InvalidInput,
                "index * 4 overflows u64: report InvalidInput, do not wrap or panic"
            );
        });
    }
    #[test]
    fn a_partial_and_distant_records_are_eof() {
        within(5, RECORD, || {
            let mut c = Cursor::new(vec![1, 0, 0, 0, 9, 9]);
            assert_eq!(
                read_record(&mut c, 1).unwrap_err().kind(),
                io::ErrorKind::UnexpectedEof,
                "half a record is not a record"
            );
            assert_eq!(
                read_record(&mut c, 1 << 40).unwrap_err().kind(),
                io::ErrorKind::UnexpectedEof,
                "seeking far past the end succeeds; the read then finds nothing"
            );
            assert_eq!(read_record(&mut c, 0).unwrap(), 1, "seek from the start");
        });
    }
    #[test]
    fn b_framed_read_does_not_eat_next_message() {
        within(5, FRAME, || {
            let mut input = &b"\x02\0hi\0\0"[..];
            assert_eq!(read_frame(&mut input).unwrap(), b"hi");
            assert_eq!(input, b"\0\0", "the next frame must still be in the reader");
            assert!(
                read_frame(&mut input).unwrap().is_empty(),
                "a zero length is a valid, empty frame"
            );
            assert_eq!(
                read_frame(&b"\x02\0x"[..]).unwrap_err().kind(),
                io::ErrorKind::UnexpectedEof,
                "the header promised 2 bytes and only 1 arrived"
            );
        });
    }
    #[test]
    fn b_truncated_headers_and_little_endian_lengths() {
        within(5, FRAME, || {
            assert_eq!(
                read_frame(&b"\x02"[..]).unwrap_err().kind(),
                io::ErrorKind::UnexpectedEof,
                "half a header is not a header"
            );
            // Header bytes 01 02 mean 0x0201 = 513 in little endian. Read as
            // big endian they would mean 0x0102 = 258.
            let mut frame = vec![0x01, 0x02];
            frame.extend(std::iter::repeat_n(b'z', 513));
            frame.push(b'!');
            let mut input = &frame[..];
            assert_eq!(
                read_frame(&mut input).unwrap().len(),
                513,
                "header bytes 01 02 are the little-endian length 513"
            );
            assert_eq!(input, b"!", "exactly one frame may be consumed");
        });
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. SeekFrom::End(-1): the new position, then the byte read there.
        let mut c = Cursor::new(vec![1u8, 2, 3]);
        let pos = c.seek(SeekFrom::End(-1)).unwrap();
        let mut b = [0];
        c.read_exact(&mut b).unwrap();
        let p1: (u64, u8) = (2, 3);
        assert_eq!(p1, (pos, b[0]), "P1: seeking from the end");

        // P2. Seek past the end of a Cursor<Vec<u8>>, then write one byte: the
        // position, the length before the write, and the length after.
        let mut c = Cursor::new(vec![1u8, 2, 3]);
        let pos = c.seek(SeekFrom::Start(10)).unwrap();
        let before = c.get_ref().len();
        c.write_all(&[9]).unwrap();
        let p2: (u64, usize, usize) = (10, 3, 11);
        assert_eq!(
            p2,
            (pos, before, c.get_ref().len()),
            "P2: seeking past the end"
        );

        // P3. Seeking before the start: which error kind?
        let mut c = Cursor::new(vec![1u8]);
        let p3: io::ErrorKind = io::ErrorKind::InvalidInput;
        assert_eq!(
            p3,
            c.seek(SeekFrom::Current(-5)).unwrap_err().kind(),
            "P3: negative positions"
        );

        // P4. Byte order: decode two little-endian bytes; encode big-endian.
        let p4: (u16, [u8; 2]) = (0x1234, [0x12, 0x34]);
        assert_eq!(
            p4,
            (u16::from_le_bytes([0x34, 0x12]), 0x1234u16.to_be_bytes()),
            "P4: from_le_bytes / to_be_bytes"
        );
    }
}
