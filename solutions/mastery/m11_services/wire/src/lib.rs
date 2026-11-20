//! LESSON 07, part 1 — A binary protocol as a shared crate: framing and encoding
//! Read first: CONCEPTS.md, "Lesson 07". Start after: lesson 01; chapter 18 helps.
//! Run: cargo test --manifest-path mastery/m11_services/Cargo.toml -p beacon_wire
//!
//! This crate is the single definition of the "beacon" protocol. The sender and
//! the receiver are different programs (part 2 of this lesson, and later the
//! capstone's agent and server), and both depend on this crate, so they cannot
//! disagree about the bytes. The crate does no I/O at all: it turns messages into
//! bytes and bytes into messages. That keeps it usable from async code, from
//! blocking code, and from tests.
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: TCP delivers a stream of bytes, not
//! messages. One `read` may return half a message, or three and a half. What must
//! a decoder return in each case?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! EXPLAIN afterward: Which check in `decode_frame` protects memory, which detects
//! damage, and which allows the protocol to change later? Why must the length be
//! checked before the payload has arrived? What does the checksum not protect
//! against?
//! JUDGMENT: Write a protocol by hand when you control both ends and care about
//! bytes or about exact framing rules, as here. Otherwise prefer a format with a
//! schema and tooling: Protocol Buffers (`prost`) when other languages take part,
//! `postcard` for Rust-only messages through serde, and plain JSON
//! lines when a human will read the traffic. gRPC (`tonic`) adds a full RPC layer
//! on top; CONCEPTS.md has a short overview. A CRC detects accidental damage only.
//! It gives no protection against a deliberate change; that needs TLS or a MAC.
//!
//! # The frame
//!
//! ```text
//! offset  size  field
//! 0       2     magic: the bytes 0x42 0x57 ("BW")
//! 2       1     version: 1
//! 3       1     kind: 1 = Hello, 2 = Batch, 3 = Ack
//! 4       4     payload length n, unsigned, big-endian; at most MAX_PAYLOAD
//! 8       n     payload (layout depends on kind, see `Message`)
//! 8+n     4     CRC-32 (IEEE) of bytes 0 .. 8+n, big-endian
//! ```
//!
//! # Payload building blocks
//!
//! - **varint**: an unsigned integer in 1 to 10 bytes. Each byte carries 7 bits,
//!   least significant group first; the top bit of a byte is 1 when more bytes
//!   follow. 0 is `00`, 127 is `7f`, 128 is `80 01`, 300 is `ac 02`.
//! - **zigzag**: a signed integer mapped to an unsigned one so that small
//!   magnitudes stay small: 0 → 0, -1 → 1, 1 → 2, -2 → 3, ... and then written as
//!   a varint.
//! - **string**: a varint byte length, then that many bytes of UTF-8.

// Starter bodies are `todo!()`, so their parameters stay unused until you
// implement them, and clippy cannot yet see that the `&mut Vec` parameters will
// be pushed to.
#![allow(dead_code, unused_variables, clippy::ptr_arg)]

/// The first two bytes of every frame.
pub const MAGIC: [u8; 2] = *b"BW";
/// The protocol version that this crate speaks.
pub const VERSION: u8 = 1;
/// Bytes before the payload.
pub const HEADER_LEN: usize = 8;
/// Bytes after the payload.
pub const TRAILER_LEN: usize = 4;
/// Largest payload a frame may carry.
pub const MAX_PAYLOAD: usize = 64 * 1024;

/// One measurement: a time in milliseconds since the Unix epoch, and a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(test, derive(serde::Serialize))]
pub struct Sample {
    pub ts_ms: u64,
    pub value: i64,
}

/// The messages of protocol version 1.
///
/// Payload layouts:
///
/// - `Hello` (kind 1): string `agent`.
/// - `Batch` (kind 2): varint `seq`, string `metric`, varint sample count, then
///   per sample a varint time and a zigzag varint `value`. The time of the first
///   sample is written in full. Every later time is written as the difference to
///   the sample before it. Times never decrease inside a batch.
/// - `Ack` (kind 3): varint `seq`.
///
/// A payload must be used up exactly: bytes left over after the last field are an
/// error.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(test, derive(serde::Serialize))]
pub enum Message {
    /// The first message on a connection: who is sending.
    Hello { agent: String },
    /// Samples of one metric. `seq` identifies the batch in the answering `Ack`.
    Batch {
        seq: u64,
        metric: String,
        samples: Vec<Sample>,
    },
    /// The receiver has stored batch `seq` durably.
    Ack { seq: u64 },
}

/// Everything that can be wrong with bytes that claim to be a frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WireError {
    /// The first two bytes are not [`MAGIC`]: this is not our protocol, or the
    /// stream is out of step.
    BadMagic([u8; 2]),
    /// A version this crate does not speak.
    UnsupportedVersion(u8),
    /// The payload is (or claims to be) longer than [`MAX_PAYLOAD`].
    TooLarge { len: usize },
    /// The checksum in the trailer does not match the bytes.
    BadChecksum { expected: u32, found: u32 },
    /// The frame is intact, but this version has no message of that kind.
    UnknownKind(u8),
    /// The payload ends in the middle of a field.
    Truncated,
    /// A varint is longer than 10 bytes or does not fit in 64 bits.
    BadVarint,
    /// A string is not valid UTF-8.
    BadUtf8,
    /// The payload has bytes after its last field.
    TrailingBytes(usize),
    /// A batch whose times decrease cannot be encoded.
    TimesNotSorted,
}

impl std::fmt::Display for WireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WireError::BadMagic(bytes) => write!(f, "bad magic bytes {bytes:02x?}"),
            WireError::UnsupportedVersion(v) => write!(f, "unsupported protocol version {v}"),
            WireError::TooLarge { len } => {
                write!(
                    f,
                    "payload of {len} bytes exceeds the limit of {MAX_PAYLOAD}"
                )
            }
            WireError::BadChecksum { expected, found } => {
                write!(
                    f,
                    "checksum mismatch: computed {expected:08x}, frame says {found:08x}"
                )
            }
            WireError::UnknownKind(kind) => write!(f, "unknown message kind {kind}"),
            WireError::Truncated => f.write_str("payload ends in the middle of a field"),
            WireError::BadVarint => f.write_str("malformed variable-length integer"),
            WireError::BadUtf8 => f.write_str("string is not valid UTF-8"),
            WireError::TrailingBytes(n) => write!(f, "{n} unexpected bytes after the last field"),
            WireError::TimesNotSorted => f.write_str("sample times decrease inside a batch"),
        }
    }
}

impl std::error::Error for WireError {}

/// A. Append `value` to `out` as a varint (see the crate header).
pub fn put_varint(out: &mut Vec<u8>, value: u64) {
    let mut rest = value;
    while rest >= 0x80 {
        out.push((rest as u8 & 0x7f) | 0x80);
        rest >>= 7;
    }
    out.push(rest as u8);
}

/// A. Read one varint from the front of `input` and advance `input` past it.
/// `Truncated` if the input ends before the final byte; `BadVarint` if the value
/// needs more than 64 bits (an eleventh byte, or a tenth byte above 1).
pub fn get_varint(input: &mut &[u8]) -> Result<u64, WireError> {
    let mut value = 0u64;
    for index in 0..10 {
        let Some((&byte, rest)) = input.split_first() else {
            return Err(WireError::Truncated);
        };
        *input = rest;
        let bits = u64::from(byte & 0x7f);
        // The tenth byte may only carry the single remaining bit.
        if index == 9 && bits > 1 {
            return Err(WireError::BadVarint);
        }
        value |= bits << (7 * index);
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(WireError::BadVarint)
}

/// A. Map a signed value to an unsigned one: 0, -1, 1, -2, 2, ... become
/// 0, 1, 2, 3, 4, ...
pub fn zigzag(value: i64) -> u64 {
    ((value << 1) ^ (value >> 63)) as u64
}

/// A. The inverse of [`zigzag`].
pub fn unzigzag(value: u64) -> i64 {
    ((value >> 1) as i64) ^ -((value & 1) as i64)
}

fn put_string(out: &mut Vec<u8>, text: &str) {
    put_varint(out, text.len() as u64);
    out.extend_from_slice(text.as_bytes());
}

fn get_string(input: &mut &[u8]) -> Result<String, WireError> {
    let len = get_varint(input)?;
    // Compare as u64 first: a hostile length must not be cast down and wrap.
    if len > input.len() as u64 {
        return Err(WireError::Truncated);
    }
    let (bytes, rest) = input.split_at(len as usize);
    *input = rest;
    String::from_utf8(bytes.to_vec()).map_err(|_| WireError::BadUtf8)
}

impl Message {
    /// The kind byte of this message.
    pub fn kind(&self) -> u8 {
        match self {
            Message::Hello { .. } => 1,
            Message::Batch { .. } => 2,
            Message::Ack { .. } => 3,
        }
    }

    /// B. Append the payload of this message to `out` (layouts: see [`Message`]).
    /// The only possible error is `TimesNotSorted`; in that case `out` may hold a
    /// partial payload and the caller discards it.
    pub fn encode_payload(&self, out: &mut Vec<u8>) -> Result<(), WireError> {
        match self {
            Message::Hello { agent } => put_string(out, agent),
            Message::Batch {
                seq,
                metric,
                samples,
            } => {
                put_varint(out, *seq);
                put_string(out, metric);
                put_varint(out, samples.len() as u64);
                let mut previous = 0u64;
                for sample in samples {
                    // Delta encoding: regular sampling gives small differences,
                    // and small numbers are short varints.
                    let delta = sample
                        .ts_ms
                        .checked_sub(previous)
                        .ok_or(WireError::TimesNotSorted)?;
                    put_varint(out, delta);
                    put_varint(out, zigzag(sample.value));
                    previous = sample.ts_ms;
                }
            }
            Message::Ack { seq } => put_varint(out, *seq),
        }
        Ok(())
    }

    /// B. Decode a payload of the given kind. Never panics and never allocates
    /// more than the payload justifies, whatever the bytes are: a sample count of
    /// four billion in a ten-byte payload is `Truncated`, not a huge `Vec`.
    /// Sample times are sums of the differences in the payload; a sum that does
    /// not fit in 64 bits is `BadVarint`, like any other number that is too large.
    pub fn decode_payload(kind: u8, payload: &[u8]) -> Result<Message, WireError> {
        let mut input = payload;
        let message = match kind {
            1 => Message::Hello {
                agent: get_string(&mut input)?,
            },
            2 => {
                let seq = get_varint(&mut input)?;
                let metric = get_string(&mut input)?;
                let count = get_varint(&mut input)?;
                // Every sample takes at least two bytes, so the rest of the
                // payload bounds how many there can be. Do not trust `count`
                // for the allocation.
                if count > (input.len() / 2) as u64 {
                    return Err(WireError::Truncated);
                }
                let mut samples = Vec::with_capacity(count as usize);
                let mut time = 0u64;
                for _ in 0..count {
                    let delta = get_varint(&mut input)?;
                    time = time.checked_add(delta).ok_or(WireError::BadVarint)?;
                    let value = unzigzag(get_varint(&mut input)?);
                    samples.push(Sample { ts_ms: time, value });
                }
                Message::Batch {
                    seq,
                    metric,
                    samples,
                }
            }
            3 => Message::Ack {
                seq: get_varint(&mut input)?,
            },
            other => return Err(WireError::UnknownKind(other)),
        };
        if !input.is_empty() {
            return Err(WireError::TrailingBytes(input.len()));
        }
        Ok(message)
    }
}

/// C. Append one complete frame for `message` to `out`: header, payload, CRC.
/// A payload over [`MAX_PAYLOAD`] is `TooLarge`. On any error, `out` is left
/// exactly as it was, so a caller can keep using its buffer.
pub fn encode_frame(message: &Message, out: &mut Vec<u8>) -> Result<(), WireError> {
    let start = out.len();
    out.extend_from_slice(&MAGIC);
    out.push(VERSION);
    out.push(message.kind());
    out.extend_from_slice(&[0; 4]); // the length, filled in below
    let result = message.encode_payload(out).and_then(|()| {
        let len = out.len() - start - HEADER_LEN;
        if len > MAX_PAYLOAD {
            return Err(WireError::TooLarge { len });
        }
        out[start + 4..start + HEADER_LEN].copy_from_slice(&(len as u32).to_be_bytes());
        let crc = crc32fast::hash(&out[start..]);
        out.extend_from_slice(&crc.to_be_bytes());
        Ok(())
    });
    if result.is_err() {
        out.truncate(start);
    }
    result
}

/// C. Try to decode one frame from the front of `buf`, which holds bytes received
/// so far.
///
/// - `Ok(Some((message, used)))`: a complete, valid frame; it occupied the first
///   `used` bytes of `buf`. More frames may follow.
/// - `Ok(None)`: so far so good, but the frame is not complete. Call again when
///   more bytes have arrived.
/// - `Err(_)`: these bytes can never become a valid frame. The connection is
///   useless from here on, because the frame boundaries are lost.
///
/// Check each header field **as soon as its bytes are there**: magic after 2
/// bytes, version after 3, the length after 8. A sender that announces a 4 GiB
/// payload must be refused when the header arrives, not after you have waited for
/// (and buffered) 4 GiB. Verify the checksum before you decode the payload.
pub fn decode_frame(buf: &[u8]) -> Result<Option<(Message, usize)>, WireError> {
    if buf.len() >= 2 && buf[..2] != MAGIC {
        return Err(WireError::BadMagic([buf[0], buf[1]]));
    }
    if buf.len() >= 3 && buf[2] != VERSION {
        return Err(WireError::UnsupportedVersion(buf[2]));
    }
    if buf.len() < HEADER_LEN {
        return Ok(None);
    }
    let len = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]) as usize;
    if len > MAX_PAYLOAD {
        return Err(WireError::TooLarge { len });
    }
    let total = HEADER_LEN + len + TRAILER_LEN;
    if buf.len() < total {
        return Ok(None);
    }
    let (body, trailer) = buf[..total].split_at(HEADER_LEN + len);
    let expected = crc32fast::hash(body);
    let found = u32::from_be_bytes([trailer[0], trailer[1], trailer[2], trailer[3]]);
    if expected != found {
        return Err(WireError::BadChecksum { expected, found });
    }
    let message = Message::decode_payload(buf[3], &body[HEADER_LEN..])?;
    Ok(Some((message, total)))
}

/// Placeholder for a prediction. Replace the whole call (`crate::guess()`) with
/// your answer before running the lesson, for example `let p1: usize = 5;`.
#[track_caller]
pub fn guess<T>() -> T {
    panic!("PREDICT: replace `guess()` with your written prediction, then run again")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs a test body on a helper thread, and fails with a message if the body
    /// has not finished after ten seconds (for example because a loop in the
    /// code under test never ends) instead of hanging the whole test run.
    #[track_caller]
    fn within_10s(what: &str, body: impl FnOnce() + Send + 'static) {
        use std::sync::mpsc::{self, RecvTimeoutError};
        let (done_tx, done) = mpsc::channel();
        std::thread::spawn(move || {
            body();
            let _ = done_tx.send(());
        });
        match done.recv_timeout(std::time::Duration::from_secs(10)) {
            Ok(()) => {}
            Err(RecvTimeoutError::Timeout) => {
                panic!("{what} was still running after 10 s: look for a loop that never ends")
            }
            Err(RecvTimeoutError::Disconnected) => {
                panic!("{what} failed; see the panic message above")
            }
        }
    }

    fn varint(value: u64) -> Vec<u8> {
        let mut out = Vec::new();
        put_varint(&mut out, value);
        out
    }

    fn frame(message: &Message) -> Vec<u8> {
        let mut out = Vec::new();
        encode_frame(message, &mut out).expect("encodable message");
        out
    }

    fn hello() -> Message {
        Message::Hello {
            agent: "pump-7".to_owned(),
        }
    }

    /// `count` samples, one per second, with values that wander a little.
    fn batch(seq: u64, count: u64) -> Message {
        Message::Batch {
            seq,
            metric: "inlet_pressure_kpa".to_owned(),
            samples: (0..count)
                .map(|i| Sample {
                    ts_ms: 1_800_000_000_000 + i * 1000,
                    value: 2500 + (i as i64 * 37) % 23 - 11,
                })
                .collect(),
        }
    }

    /// A frame with a correct header and checksum around an arbitrary payload.
    fn raw_frame(kind: u8, payload: &[u8]) -> Vec<u8> {
        let mut out = vec![MAGIC[0], MAGIC[1], VERSION, kind];
        out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        out.extend_from_slice(payload);
        let crc = crc32fast::hash(&out);
        out.extend_from_slice(&crc.to_be_bytes());
        out
    }

    #[test]
    fn a_varints_match_the_documented_bytes() {
        within_10s(
            "the test `a_varints_match_the_documented_bytes`",
            move || {
                assert_eq!(varint(0), [0x00]);
                assert_eq!(varint(1), [0x01]);
                assert_eq!(varint(127), [0x7f]);
                assert_eq!(varint(128), [0x80, 0x01]);
                assert_eq!(varint(300), [0xac, 0x02]);
                assert_eq!(varint(16_383), [0xff, 0x7f]);
                assert_eq!(varint(16_384), [0x80, 0x80, 0x01]);
                assert_eq!(varint(u64::MAX).len(), 10);
                assert_eq!(varint(u64::MAX)[9], 0x01);
            },
        )
    }

    #[test]
    fn a_varints_round_trip_and_advance_the_input() {
        within_10s(
            "the test `a_varints_round_trip_and_advance_the_input`",
            move || {
                for value in [
                    0,
                    1,
                    127,
                    128,
                    255,
                    300,
                    65_535,
                    1 << 32,
                    u64::MAX - 1,
                    u64::MAX,
                ] {
                    let mut bytes = varint(value);
                    bytes.extend_from_slice(&[0xaa, 0xbb]);
                    let mut input = bytes.as_slice();
                    assert_eq!(get_varint(&mut input), Ok(value), "value {value}");
                    assert_eq!(
                        input,
                        [0xaa, 0xbb],
                        "the input must point just past the varint"
                    );
                }
            },
        )
    }

    #[test]
    fn a_bad_varints_are_errors_not_panics() {
        within_10s(
            "the test `a_bad_varints_are_errors_not_panics`",
            move || {
                let mut empty: &[u8] = &[];
                assert_eq!(get_varint(&mut empty), Err(WireError::Truncated));
                let mut unfinished: &[u8] = &[0x80, 0x80];
                assert_eq!(get_varint(&mut unfinished), Err(WireError::Truncated));
                let mut eleven: &[u8] = &[0xff; 11];
                assert_eq!(get_varint(&mut eleven), Err(WireError::BadVarint));
                // Ten bytes, but the last one carries bits beyond the 64th.
                let mut too_big: &[u8] =
                    &[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x02];
                assert_eq!(
                    get_varint(&mut too_big),
                    Err(WireError::BadVarint),
                    "a value that does not fit in u64 must not be silently cut off"
                );
            },
        )
    }

    #[test]
    fn a_zigzag_keeps_small_magnitudes_small() {
        within_10s(
            "the test `a_zigzag_keeps_small_magnitudes_small`",
            move || {
                let pairs = [
                    (0, 0),
                    (-1, 1),
                    (1, 2),
                    (-2, 3),
                    (2, 4),
                    (-64, 127),
                    (64, 128),
                ];
                for (signed, unsigned) in pairs {
                    assert_eq!(zigzag(signed), unsigned, "zigzag({signed})");
                    assert_eq!(unzigzag(unsigned), signed, "unzigzag({unsigned})");
                }
                for value in [i64::MIN, i64::MIN + 1, -1_000_000, 1_000_000, i64::MAX] {
                    assert_eq!(unzigzag(zigzag(value)), value);
                }
                assert_eq!(zigzag(i64::MIN), u64::MAX);
                assert_eq!(
                    varint(zigzag(-1)).len(),
                    1,
                    "-1 must take one byte, not ten"
                );
            },
        )
    }

    #[test]
    fn b_payloads_match_the_documented_layout() {
        within_10s(
            "the test `b_payloads_match_the_documented_layout`",
            move || {
                let mut out = Vec::new();
                hello().encode_payload(&mut out).unwrap();
                assert_eq!(out, b"\x06pump-7");

                let mut out = Vec::new();
                Message::Ack { seq: 300 }.encode_payload(&mut out).unwrap();
                assert_eq!(out, [0xac, 0x02]);

                let small = Message::Batch {
                    seq: 5,
                    metric: "t".to_owned(),
                    samples: vec![
                        Sample {
                            ts_ms: 1000,
                            value: -1,
                        },
                        Sample {
                            ts_ms: 1010,
                            value: 2,
                        },
                        Sample {
                            ts_ms: 1010,
                            value: 0,
                        },
                    ],
                };
                let mut out = Vec::new();
                small.encode_payload(&mut out).unwrap();
                assert_eq!(
                    out,
                    [
                        0x05, // seq
                        0x01, b't', // metric
                        0x03, // three samples
                        0xe8, 0x07, 0x01, // time 1000 in full, value -1
                        0x0a, 0x04, // +10 ms, value 2
                        0x00, 0x00, // +0 ms, value 0
                    ]
                );
                assert_eq!(Message::decode_payload(2, &out), Ok(small));
            },
        )
    }

    #[test]
    fn b_messages_round_trip() {
        within_10s("the test `b_messages_round_trip`", move || {
            let extremes = Message::Batch {
                seq: u64::MAX,
                metric: "température °C".to_owned(),
                samples: vec![
                    Sample {
                        ts_ms: 0,
                        value: i64::MIN,
                    },
                    Sample {
                        ts_ms: 0,
                        value: i64::MAX,
                    },
                    Sample {
                        ts_ms: u64::MAX,
                        value: 0,
                    },
                ],
            };
            let empty = Message::Batch {
                seq: 0,
                metric: String::new(),
                samples: vec![],
            };
            for message in [
                hello(),
                Message::Ack { seq: 0 },
                batch(9, 50),
                extremes,
                empty,
            ] {
                let mut payload = Vec::new();
                message.encode_payload(&mut payload).unwrap();
                assert_eq!(
                    Message::decode_payload(message.kind(), &payload),
                    Ok(message)
                );
            }
        })
    }

    #[test]
    fn b_unsorted_times_cannot_be_encoded() {
        within_10s("the test `b_unsorted_times_cannot_be_encoded`", move || {
            let unsorted = Message::Batch {
                seq: 1,
                metric: "t".to_owned(),
                samples: vec![
                    Sample {
                        ts_ms: 2000,
                        value: 1,
                    },
                    Sample {
                        ts_ms: 1999,
                        value: 1,
                    },
                ],
            };
            assert_eq!(
                unsorted.encode_payload(&mut Vec::new()),
                Err(WireError::TimesNotSorted)
            );
        })
    }

    #[test]
    fn b_hostile_payloads_are_errors_not_panics() {
        within_10s(
            "the test `b_hostile_payloads_are_errors_not_panics`",
            move || {
                assert_eq!(
                    Message::decode_payload(9, &[]),
                    Err(WireError::UnknownKind(9))
                );
                assert_eq!(
                    Message::decode_payload(0, &[1]),
                    Err(WireError::UnknownKind(0))
                );
                // Hello: the string claims 6 bytes, 3 are present.
                assert_eq!(
                    Message::decode_payload(1, b"\x06pum"),
                    Err(WireError::Truncated)
                );
                assert_eq!(
                    Message::decode_payload(1, &[2, 0xff, 0xfe]),
                    Err(WireError::BadUtf8)
                );
                assert_eq!(
                    Message::decode_payload(3, &[1, 2]),
                    Err(WireError::TrailingBytes(1))
                );
                assert_eq!(Message::decode_payload(3, &[]), Err(WireError::Truncated));
                // A string length close to u64::MAX must not wrap around or allocate.
                let mut huge_string = varint(u64::MAX - 3);
                huge_string.extend_from_slice(b"abc");
                assert_eq!(
                    Message::decode_payload(1, &huge_string),
                    Err(WireError::Truncated)
                );
                // Batch: seq 1, metric "", and a sample count of four billion with no samples.
                let mut lying = vec![0x01, 0x00];
                put_varint(&mut lying, 4_000_000_000);
                assert_eq!(
                    Message::decode_payload(2, &lying),
                    Err(WireError::Truncated),
                    "the count must be checked against the bytes that are really there"
                );
                // The same with a count so large that reserving memory for it cannot work.
                let mut absurd = vec![0x01, 0x00];
                put_varint(&mut absurd, 1 << 60);
                assert_eq!(
                    Message::decode_payload(2, &absurd),
                    Err(WireError::Truncated),
                    "never size an allocation from a number that the peer sent"
                );
                // Two samples whose time differences add up to more than u64::MAX.
                let mut overflow = vec![0x01, 0x00, 0x02];
                put_varint(&mut overflow, u64::MAX);
                overflow.push(0);
                put_varint(&mut overflow, 1);
                overflow.push(0);
                assert_eq!(
                    Message::decode_payload(2, &overflow),
                    Err(WireError::BadVarint)
                );
            },
        )
    }

    #[test]
    fn c_frames_have_the_documented_bytes() {
        within_10s("the test `c_frames_have_the_documented_bytes`", move || {
            let bytes = frame(&Message::Ack { seq: 300 });
            assert_eq!(&bytes[..2], b"BW");
            assert_eq!(bytes[2], 1, "version");
            assert_eq!(bytes[3], 3, "kind");
            assert_eq!(&bytes[4..8], [0, 0, 0, 2], "payload length, big-endian");
            assert_eq!(&bytes[8..10], [0xac, 0x02]);
            assert_eq!(
                &bytes[10..],
                crc32fast::hash(&bytes[..10]).to_be_bytes(),
                "CRC-32 of header and payload, big-endian"
            );
            assert_eq!(bytes.len(), HEADER_LEN + 2 + TRAILER_LEN);
            assert_eq!(
                decode_frame(&bytes),
                Ok(Some((Message::Ack { seq: 300 }, 14)))
            );
        })
    }

    #[test]
    fn c_frames_append_and_decode_one_at_a_time() {
        within_10s(
            "the test `c_frames_append_and_decode_one_at_a_time`",
            move || {
                let mut stream = vec![];
                let messages = [hello(), batch(1, 3), Message::Ack { seq: 1 }];
                for message in &messages {
                    encode_frame(message, &mut stream).unwrap();
                }
                let mut rest = stream.as_slice();
                for message in &messages {
                    let (decoded, used) = decode_frame(rest)
                        .unwrap()
                        .expect("a complete frame is at the front");
                    assert_eq!(&decoded, message);
                    rest = &rest[used..];
                }
                assert!(rest.is_empty(), "every byte belongs to exactly one frame");
                assert_eq!(
                    decode_frame(rest),
                    Ok(None),
                    "an empty buffer needs more bytes"
                );
            },
        )
    }

    #[test]
    fn c_every_proper_prefix_asks_for_more_bytes() {
        within_10s(
            "the test `c_every_proper_prefix_asks_for_more_bytes`",
            move || {
                let bytes = frame(&batch(7, 4));
                for cut in 0..bytes.len() {
                    assert_eq!(
                        decode_frame(&bytes[..cut]),
                        Ok(None),
                        "the first {cut} of {} bytes are a valid beginning",
                        bytes.len()
                    );
                }
            },
        )
    }

    #[test]
    fn c_every_single_damaged_byte_is_noticed() {
        within_10s(
            "the test `c_every_single_damaged_byte_is_noticed`",
            move || {
                let original = batch(7, 4);
                let bytes = frame(&original);
                for position in 0..bytes.len() {
                    for flip in [0x01, 0x80, 0xff] {
                        let mut damaged = bytes.clone();
                        damaged[position] ^= flip;
                        let result = decode_frame(&damaged);
                        assert!(
                            !matches!(result, Ok(Some(_))),
                            "byte {position} xor {flip:#04x} went unnoticed: {result:?}"
                        );
                    }
                }
                // Damage in the payload or the checksum is a checksum error.
                let mut damaged = bytes.clone();
                damaged[HEADER_LEN + 1] ^= 0x10;
                assert!(matches!(
                    decode_frame(&damaged),
                    Err(WireError::BadChecksum { .. })
                ));
            },
        )
    }

    #[test]
    fn c_header_fields_are_checked_as_soon_as_they_arrive() {
        within_10s(
            "the test `c_header_fields_are_checked_as_soon_as_they_arrive`",
            move || {
                assert_eq!(decode_frame(b"GE"), Err(WireError::BadMagic(*b"GE")));
                assert_eq!(
                    decode_frame(b"GET / HTTP/1.1\r\n"),
                    Err(WireError::BadMagic(*b"GE")),
                    "another protocol on our port"
                );
                assert_eq!(
                    decode_frame(b"B"),
                    Ok(None),
                    "one byte cannot be judged yet"
                );
                assert_eq!(
                    decode_frame(&[b'B', b'W', 2]),
                    Err(WireError::UnsupportedVersion(2)),
                    "a version we do not speak is refused before anything else is read"
                );

                // Only a header, announcing a payload of almost 4 GiB.
                let announced = [b'B', b'W', 1, 2, 0xff, 0xff, 0xff, 0xf0];
                assert_eq!(
                    decode_frame(&announced),
                    Err(WireError::TooLarge { len: 0xffff_fff0 }),
                    "refuse an oversized frame when its header arrives; `Ok(None)` here \
             would make the receiver buffer gigabytes"
                );
                let just_over = [b'B', b'W', 1, 2, 0x00, 0x01, 0x00, 0x01];
                assert_eq!(
                    decode_frame(&just_over),
                    Err(WireError::TooLarge {
                        len: MAX_PAYLOAD + 1
                    })
                );
                let at_limit = [b'B', b'W', 1, 2, 0x00, 0x01, 0x00, 0x00];
                assert_eq!(
                    decode_frame(&at_limit),
                    Ok(None),
                    "exactly MAX_PAYLOAD is allowed"
                );
            },
        )
    }

    #[test]
    fn c_intact_frames_with_bad_contents_are_errors() {
        within_10s(
            "the test `c_intact_frames_with_bad_contents_are_errors`",
            move || {
                assert_eq!(
                    decode_frame(&raw_frame(9, &[])),
                    Err(WireError::UnknownKind(9)),
                    "the frame is intact, so the kind is the problem, not the checksum"
                );
                assert_eq!(
                    decode_frame(&raw_frame(3, &[1, 2])),
                    Err(WireError::TrailingBytes(1))
                );
                assert_eq!(
                    decode_frame(&raw_frame(1, b"\x09short")),
                    Err(WireError::Truncated)
                );
            },
        )
    }

    #[test]
    fn c_oversized_messages_are_refused_and_leave_the_buffer_alone() {
        within_10s(
            "the test `c_oversized_messages_are_refused_and_leave_the_buffer_alone`",
            move || {
                let huge = Message::Hello {
                    agent: "x".repeat(MAX_PAYLOAD),
                };
                let mut out = b"earlier frames".to_vec();
                assert!(matches!(
                    encode_frame(&huge, &mut out),
                    Err(WireError::TooLarge { len }) if len > MAX_PAYLOAD
                ));
                assert_eq!(
                    out, b"earlier frames",
                    "a failed encode must not leave half a frame"
                );

                let unsorted = Message::Batch {
                    seq: 1,
                    metric: "t".to_owned(),
                    samples: vec![Sample { ts_ms: 2, value: 0 }, Sample { ts_ms: 1, value: 0 }],
                };
                assert_eq!(
                    encode_frame(&unsorted, &mut out),
                    Err(WireError::TimesNotSorted)
                );
                assert_eq!(out, b"earlier frames");

                // The largest Hello that fits: 3 length bytes + the name = MAX_PAYLOAD.
                let largest = Message::Hello {
                    agent: "x".repeat(MAX_PAYLOAD - 3),
                };
                let bytes = frame(&largest);
                assert_eq!(bytes.len(), HEADER_LEN + MAX_PAYLOAD + TRAILER_LEN);
                assert_eq!(decode_frame(&bytes), Ok(Some((largest, bytes.len()))));
            },
        )
    }

    #[test]
    fn d_the_binary_form_is_much_smaller_than_json() {
        within_10s(
            "the test `d_the_binary_form_is_much_smaller_than_json`",
            move || {
                // Task D is this measurement: no code to write, but read the numbers.
                let message = batch(42, 100);
                let binary = frame(&message).len();
                let json = serde_json::to_vec(&message).unwrap().len();
                println!("100 samples: {binary} bytes as a frame, {json} bytes as JSON");
                assert!(
                    binary * 8 < json,
                    "delta-encoded varints should be at least 8 times smaller than JSON \
             here, got {binary} vs {json} bytes"
                );
                let per_sample = (binary - frame(&batch(42, 0)).len()) as f64 / 100.0;
                assert!(
                    per_sample < 4.5,
                    "a sample should cost about 4 bytes (2 for the 1000 ms step, 2 for \
             the value), got {per_sample}"
                );
            },
        )
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        within_10s("the test `p_predictions`", move || {
            // P1. How many bytes is the complete frame for `Ack { seq: 1 }`?
            let p1: usize = 13;
            assert_eq!(
                p1,
                frame(&Message::Ack { seq: 1 }).len(),
                "P1: smallest Ack"
            );

            // P2. And for `Ack { seq: 1_000_000 }`?
            let p2: usize = 15;
            assert_eq!(
                p2,
                frame(&Message::Ack { seq: 1_000_000 }).len(),
                "P2: a varint grows by one byte per 7 bits"
            );

            // P3. How many bytes does the varint for the value -3 take when it is
            // written (a) with zigzag, (b) by casting to u64 without zigzag?
            let p3: (usize, usize) = (1, 10);
            assert_eq!(
                p3,
                (varint(zigzag(-3)).len(), varint(-3i64 as u64).len()),
                "P3: why signed values need zigzag"
            );

            // P4. Two valid frames arrive glued together in one read, followed by the
            // first 5 bytes of a third. What does the FIRST call of `decode_frame` on
            // that buffer say it used?
            let mut buffer = frame(&Message::Ack { seq: 1 });
            buffer.extend_from_slice(&frame(&hello()));
            buffer.extend_from_slice(&frame(&hello())[..5]);
            let p4: usize = 13;
            let (_, used) = decode_frame(&buffer).unwrap().unwrap();
            assert_eq!(p4, used, "P4: one frame per call");
        })
    }
}
