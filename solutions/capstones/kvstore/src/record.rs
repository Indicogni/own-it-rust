//! Keys, values and the record format of SPEC 5.
//!
//! Two kinds of bytes reach the store from outside: request lines from
//! clients and the log file from disk. Both are checked here, in one place,
//! and turned into [`Key`] and [`Value`]. The rest of the program takes
//! those types and never checks again.

use crate::crc32;

/// The first eight bytes of every log file (SPEC 5.1).
pub const MAGIC: [u8; 8] = *b"KVSTORE1";
/// The fixed part of a record: crc (4), kind (1), key length (2), value
/// length (4).
pub const HEADER_LEN: usize = 11;
/// The longest key, in bytes (SPEC 7).
pub const MAX_KEY: usize = 128;
/// The longest value, in bytes (SPEC 7).
pub const MAX_VALUE: usize = 4096;

const KIND_SET: u8 = 1;
const KIND_DEL: u8 = 2;

/// 1 to 128 bytes, each from `0x21` to `0x7E`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key<'a>(&'a [u8]);

impl<'a> Key<'a> {
    pub fn new(bytes: &'a [u8]) -> Option<Key<'a>> {
        let valid =
            (1..=MAX_KEY).contains(&bytes.len()) && bytes.iter().all(|b| (0x21..=0x7E).contains(b));
        valid.then_some(Key(bytes))
    }

    pub fn as_bytes(self) -> &'a [u8] {
        self.0
    }
}

/// At most 4096 bytes, none of them a line feed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Value<'a>(&'a [u8]);

impl<'a> Value<'a> {
    pub fn new(bytes: &'a [u8]) -> Option<Value<'a>> {
        let valid = bytes.len() <= MAX_VALUE && !bytes.contains(&b'\n');
        valid.then_some(Value(bytes))
    }

    pub fn as_bytes(self) -> &'a [u8] {
        self.0
    }
}

/// The size on disk of the record that stores a key of `key_len` bytes with
/// a value of `value_len` bytes.
pub fn encoded_len(key_len: usize, value_len: usize) -> u64 {
    (HEADER_LEN + key_len + value_len) as u64
}

/// Appends one record to `out`: a SET record if there is a value, a DEL
/// record if not.
pub fn encode(out: &mut Vec<u8>, key: Key<'_>, value: Option<Value<'_>>) {
    let key = key.as_bytes();
    let (kind, value) = match value {
        Some(value) => (KIND_SET, value.as_bytes()),
        None => (KIND_DEL, &[][..]),
    };
    // The limits of `Key` and `Value` make both conversions lossless.
    let key_len = (key.len() as u16).to_le_bytes();
    let value_len = (value.len() as u32).to_le_bytes();

    let start = out.len();
    out.extend_from_slice(&[0; 4]); // the checksum, filled in below
    out.push(kind);
    out.extend_from_slice(&key_len);
    out.extend_from_slice(&value_len);
    out.extend_from_slice(key);
    out.extend_from_slice(value);
    let crc = crc32::checksum(&[&out[start + 4..]]);
    out[start..start + 4].copy_from_slice(&crc.to_le_bytes());
}

/// The fields of a record header whose kind and lengths are allowed.
struct Header {
    crc: u32,
    is_set: bool,
    key_len: usize,
    value_len: usize,
}

fn header(bytes: &[u8; HEADER_LEN]) -> Option<Header> {
    let [c0, c1, c2, c3, kind, k0, k1, v0, v1, v2, v3] = *bytes;
    let key_len = usize::from(u16::from_le_bytes([k0, k1]));
    // Compare as `u32` first: on a 16-bit target the cast could not hold it.
    let value_len = u32::from_le_bytes([v0, v1, v2, v3]);
    let is_set = match kind {
        KIND_SET if value_len as u64 <= MAX_VALUE as u64 => true,
        KIND_DEL if value_len == 0 => false,
        _ => return None,
    };
    (1..=MAX_KEY).contains(&key_len).then_some(Header {
        crc: u32::from_le_bytes([c0, c1, c2, c3]),
        is_set,
        key_len,
        value_len: value_len as usize,
    })
}

/// First step of reading a record: how many bytes of key and value follow
/// this header? `None` if no valid record can start with these bytes.
///
/// The answer is at most `MAX_KEY + MAX_VALUE`, so a caller can size a
/// buffer from it without trusting the file.
pub fn body_len(bytes: &[u8; HEADER_LEN]) -> Option<usize> {
    header(bytes).map(|h| h.key_len + h.value_len)
}

/// Second step: checks the checksum and the bytes of key and value
/// (SPEC 5.4). `body` must have the length that [`body_len`] gave.
/// A SET record gives `Some(value)`, a DEL record gives `None`.
pub fn decode<'a>(
    bytes: &[u8; HEADER_LEN],
    body: &'a [u8],
) -> Option<(Key<'a>, Option<Value<'a>>)> {
    let header = header(bytes)?;
    if body.len() != header.key_len + header.value_len {
        return None;
    }
    if crc32::checksum(&[&bytes[4..], body]) != header.crc {
        return None;
    }
    let (key, value) = body.split_at(header.key_len);
    let key = Key::new(key)?;
    if header.is_set {
        Some((key, Some(Value::new(value)?)))
    } else {
        Some((key, None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(text: &str) -> Key<'_> {
        Key::new(text.as_bytes()).expect("test key")
    }

    fn value(text: &str) -> Value<'_> {
        Value::new(text.as_bytes()).expect("test value")
    }

    fn split(record: &[u8]) -> (&[u8; HEADER_LEN], &[u8]) {
        let (head, body) = record.split_at(HEADER_LEN);
        (head.try_into().unwrap(), body)
    }

    #[test]
    fn key_rules() {
        assert!(Key::new(b"").is_none());
        assert!(Key::new(b"a").is_some());
        assert!(Key::new(&[b'k'; MAX_KEY]).is_some());
        assert!(Key::new(&[b'k'; MAX_KEY + 1]).is_none());
        assert!(Key::new(b"a b").is_none());
        assert!(Key::new(b"a\x7f").is_none());
        assert!(Key::new(b"!~").is_some());
        assert!(Key::new(b"\xc3\xa9").is_none(), "bytes above 0x7E");
    }

    #[test]
    fn value_rules() {
        assert!(Value::new(b"").is_some());
        assert!(Value::new(&[0xFF; MAX_VALUE]).is_some());
        assert!(Value::new(&[b'v'; MAX_VALUE + 1]).is_none());
        assert!(Value::new(b"a\nb").is_none());
        assert!(Value::new(b"a\rb\0").is_some());
    }

    #[test]
    fn the_example_of_the_spec() {
        // SPEC 5.5: SET fruit apple, SET n (empty), DEL fruit.
        let mut log = MAGIC.to_vec();
        encode(&mut log, key("fruit"), Some(value("apple")));
        encode(&mut log, key("n"), Some(value("")));
        encode(&mut log, key("fruit"), None);
        #[rustfmt::skip]
        let expected: &[u8] = &[
            0x4B, 0x56, 0x53, 0x54, 0x4F, 0x52, 0x45, 0x31,
            0xB9, 0xE8, 0x3E, 0xB7, 0x01, 0x05, 0x00, 0x05, 0x00, 0x00, 0x00,
            0x66, 0x72, 0x75, 0x69, 0x74, 0x61, 0x70, 0x70, 0x6C, 0x65,
            0x1C, 0x98, 0xF5, 0xA5, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x6E,
            0x38, 0xD1, 0x90, 0x80, 0x02, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x66, 0x72, 0x75, 0x69, 0x74,
        ];
        assert_eq!(log, expected);
    }

    #[test]
    fn encode_then_decode() {
        let long_key = "k".repeat(MAX_KEY);
        let long_value = "v".repeat(MAX_VALUE);
        for (k, v) in [
            ("a", Some("")),
            ("a", Some("1")),
            ("a", None),
            (&long_key[..], Some(&long_value[..])),
        ] {
            let mut record = Vec::new();
            encode(&mut record, key(k), v.map(value));
            assert_eq!(
                record.len() as u64,
                encoded_len(k.len(), v.map_or(0, str::len))
            );
            let (head, body) = split(&record);
            assert_eq!(body_len(head), Some(body.len()));
            assert_eq!(decode(head, body), Some((key(k), v.map(value))));
        }
    }

    #[test]
    fn encode_appends() {
        let mut log = b"earlier".to_vec();
        encode(&mut log, key("k"), Some(value("v")));
        assert_eq!(&log[..7], b"earlier");
        let (head, body) = split(&log[7..]);
        assert!(decode(head, body).is_some());
    }

    #[test]
    fn every_single_byte_change_is_rejected() {
        let mut record = Vec::new();
        encode(&mut record, key("key"), Some(value("value")));
        for at in 0..record.len() {
            let mut bad = record.clone();
            bad[at] ^= 0x10;
            let (head, body) = split(&bad);
            // A changed length field can make the body the wrong size, which
            // a reader notices before it gets as far as `decode`.
            let accepted = body_len(head) == Some(body.len()) && decode(head, body).is_some();
            assert!(!accepted, "a change in byte {at} was accepted");
        }
    }

    /// A record with a correct checksum over fields of our choice.
    fn forged(kind: u8, key_len: u16, value_len: u32, body: &[u8]) -> Vec<u8> {
        let mut rest = vec![kind];
        rest.extend_from_slice(&key_len.to_le_bytes());
        rest.extend_from_slice(&value_len.to_le_bytes());
        rest.extend_from_slice(body);
        let mut record = crc32::checksum(&[&rest]).to_le_bytes().to_vec();
        record.extend_from_slice(&rest);
        record
    }

    #[test]
    fn a_correct_checksum_is_not_enough() {
        let cases = [
            ("kind 0", forged(0, 1, 0, b"a")),
            ("kind 3", forged(3, 1, 1, b"ab")),
            ("empty key", forged(1, 0, 1, b"v")),
            ("key too long", forged(1, 129, 0, &[b'k'; 129])),
            ("value too long", forged(1, 1, 4097, &[b'v'; 4098])),
            ("value length in a DEL", forged(2, 1, 1, b"av")),
            ("space in the key", forged(1, 3, 1, b"a bv")),
            ("line feed in the value", forged(1, 1, 3, b"av\nw")),
        ];
        for (what, record) in cases {
            let (head, body) = split(&record);
            let accepted = body_len(head) == Some(body.len()) && decode(head, body).is_some();
            assert!(!accepted, "{what} was accepted");
        }
        // The same helper does produce records that pass.
        let good = forged(1, 1, 1, b"av");
        let (head, body) = split(&good);
        assert!(decode(head, body).is_some());
    }

    #[test]
    fn absurd_lengths_are_refused_from_the_header_alone() {
        let huge = forged(1, 1, u32::MAX, b"");
        let (head, _) = split(&huge);
        assert_eq!(body_len(head), None);
        assert_eq!(
            body_len(&[0; HEADER_LEN]),
            None,
            "a run of zero bytes is not a record"
        );
    }
}
