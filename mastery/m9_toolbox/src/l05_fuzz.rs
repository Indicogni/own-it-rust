//! LESSON 05 — Fuzzing and sanitizers
//! Run: cargo test --manifest-path mastery/m9_toolbox/Cargo.toml l05_
//! Read first: CONCEPTS.md, "Lesson 05 · Fuzzing and sanitizers".
//! Labs 5.1 to 5.4 in LABS.md run the real fuzzer on this lesson's code.
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: a fuzz target only calls the parser
//! and ignores the result. Which kinds of bugs can it find, and which never?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! EXPLAIN afterward: Explain what "coverage-guided" means, and why a round-trip
//! check finds bugs that "it did not crash" cannot.
//! JUDGMENT: Fuzz every function that takes bytes from outside: parsers, decoders,
//! deserializers. Start with a bytes-in target (cheap), and add a structure-aware
//! target when most random inputs are rejected early.
//!
//! The code under test is a query-string codec, as in `name=J%C3%BCrgen&city=New+York`:
//!
//! - `encode`: the bytes `A-Z a-z 0-9 - _ . ~` stay as they are, a space becomes
//!   `+`, and every other byte of the UTF-8 text becomes `%XX` (two upper-case hex
//!   digits).
//! - `decode`: `+` becomes a space, `%XX` (hex digits in either case) becomes one
//!   byte, every other character is copied. A `%` that is not followed by two hex
//!   digits is an error, and so is a result that is not UTF-8.

use arbitrary::{Arbitrary, Unstructured};

/// Why a text could not be decoded or parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// A `%` at this byte offset is not followed by two hex digits.
    BadEscape(usize),
    /// The decoded bytes are not valid UTF-8.
    InvalidUtf8,
    /// A `key=value` pair has no `=`.
    MissingEquals,
}

/// The codec as it shipped, with two bugs. Supplied; do not edit. The labs fuzz
/// this module, and the tests check that your recorded findings really break it.
pub mod buggy {
    use super::DecodeError;

    pub fn encode(text: &str) -> String {
        let mut out = String::new();
        for byte in text.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'+' => {
                    out.push(char::from(byte));
                }
                b' ' => out.push('+'),
                _ => out.push_str(&format!("%{byte:02X}")),
            }
        }
        out
    }

    pub fn decode(text: &str) -> Result<String, DecodeError> {
        let mut out = Vec::new();
        let bytes = text.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                b'+' => {
                    out.push(b' ');
                    i += 1;
                }
                b'%' => {
                    if i + 3 > bytes.len() {
                        return Err(DecodeError::BadEscape(i));
                    }
                    let hex = &text[i + 1..i + 3];
                    let byte =
                        u8::from_str_radix(hex, 16).map_err(|_| DecodeError::BadEscape(i))?;
                    out.push(byte);
                    i += 3;
                }
                other => {
                    out.push(other);
                    i += 1;
                }
            }
        }
        String::from_utf8(out).map_err(|_| DecodeError::InvalidUtf8)
    }
}

/// A. The repaired encoder. It must be the exact inverse of [`decode`] for every
/// `&str`: `decode(&encode(s)) == Ok(s)`.
pub fn encode(text: &str) -> String {
    todo!("Implement this task")
}

/// A. The repaired decoder. It must return `Ok` or `Err` for every `&str` and never
/// panic. `BadEscape` carries the byte offset of the offending `%`.
pub fn decode(text: &str) -> Result<String, DecodeError> {
    todo!("Implement this task")
}

/// A query: `key=value` pairs in order. Keys may repeat.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Query {
    pub pairs: Vec<(String, String)>,
}

/// Writes a query as text with the given encoder. Supplied.
pub fn format_query(query: &Query, encode: fn(&str) -> String) -> String {
    let pairs: Vec<String> = query
        .pairs
        .iter()
        .map(|(key, value)| format!("{}={}", encode(key), encode(value)))
        .collect();
    pairs.join("&")
}

/// Reads a query from text with the given decoder. Supplied.
pub fn parse_query(
    text: &str,
    decode: fn(&str) -> Result<String, DecodeError>,
) -> Result<Query, DecodeError> {
    let mut pairs = Vec::new();
    if !text.is_empty() {
        for pair in text.split('&') {
            let (key, value) = pair.split_once('=').ok_or(DecodeError::MissingEquals)?;
            pairs.push((decode(key)?, decode(value)?));
        }
    }
    Ok(Query { pairs })
}

/// The characters keys and values are built from in fuzzing. It mixes harmless
/// letters with every character the codec treats specially, and with characters
/// that take two and three bytes in UTF-8.
pub const ALPHABET: [char; 12] = ['a', 'F', '0', '7', ' ', '+', '%', '&', '=', '~', 'é', '€'];

/// B. Structure-aware input: builds a [`Query`] from the fuzzer's bytes.
///
/// - 0 to 4 pairs. Each key and each value has 0 to 6 characters, every one of
///   them taken from [`ALPHABET`].
/// - The same bytes always give the same query.
/// - Running out of bytes is not an error: `Unstructured` hands out zeros when it
///   is empty, so short inputs give small queries. Never return `Err` yourself.
/// - Different bytes must reach different shapes: across many inputs, queries of
///   every size from 0 to 4 pairs and every character of the alphabet must occur.
impl<'a> Arbitrary<'a> for Query {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        todo!("Implement this task")
    }
}

/// An encoder and a decoder that belong together.
#[derive(Clone, Copy)]
pub struct Codec {
    pub encode: fn(&str) -> String,
    pub decode: fn(&str) -> Result<String, DecodeError>,
}

/// The repaired codec (your task A).
pub const FIXED: Codec = Codec { encode, decode };

/// The codec as it shipped.
pub const BUGGY: Codec = Codec {
    encode: buggy::encode,
    decode: buggy::decode,
};

/// C. The body of the structure-aware fuzz target `fuzz/fuzz_targets/roundtrip.rs`.
///
/// Format `query` with the codec's encoder, parse the text back with its decoder,
/// and panic (for example with `assert_eq!`) unless the result is `Ok` and equal to
/// `query`. A fuzzer reports a panic as a finding. Returning normally means "this
/// input is fine".
pub fn fuzz_roundtrip(query: &Query, codec: Codec) {
    todo!("Implement this task")
}

/// The body of the bytes-in fuzz target `fuzz/fuzz_targets/decode.rs`. Supplied.
/// It can only find panics: it does not look at what `decode` returns.
pub fn fuzz_decode(data: &[u8], codec: Codec) {
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = (codec.decode)(text);
    }
}

/// LAB 5.1. The minimised input on which `cargo fuzz run decode` crashes: the bytes
/// that `cargo fuzz tmin` prints under "Output of `std::fmt::Debug`".
pub fn lab_decode_crash() -> &'static [u8] {
    todo!("LAB 5.1: paste the bytes of the minimised crashing input")
}

/// LAB 5.2. A value for which `cargo fuzz run roundtrip` reports a failed round
/// trip, minimised.
pub fn lab_roundtrip_counterexample() -> &'static str {
    todo!("LAB 5.2: paste the minimised counterexample")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guess;
    use std::collections::BTreeSet;
    use std::panic::catch_unwind;

    /// Deterministic "random" bytes, so every run of the tests sees the same inputs.
    fn noise(seed: u64, len: usize) -> Vec<u8> {
        let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
        (0..len)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                (state >> 32) as u8
            })
            .collect()
    }

    /// Every text of up to `max` characters over [`ALPHABET`].
    fn all_texts(max: usize) -> Vec<String> {
        let mut texts = vec![String::new()];
        let mut start = 0;
        for _ in 0..max {
            let end = texts.len();
            for i in start..end {
                for c in ALPHABET {
                    let mut longer = texts[i].clone();
                    longer.push(c);
                    texts.push(longer);
                }
            }
            start = end;
        }
        texts
    }

    #[test]
    fn a_codec_examples() {
        crate::support::within(30, "a_codec_examples", || {
            assert_eq!(encode("New York"), "New+York");
            assert_eq!(encode("a-b_c.d~e"), "a-b_c.d~e");
            assert_eq!(encode("J\u{fc}rgen"), "J%C3%BCrgen");
            assert_eq!(encode("50%"), "50%25");
            assert_eq!(encode("a=b&c"), "a%3Db%26c");
            assert_eq!(encode(""), "");
            assert_eq!(decode("New+York"), Ok("New York".to_owned()));
            assert_eq!(decode("J%C3%BCrgen"), Ok("J\u{fc}rgen".to_owned()));
            assert_eq!(decode("%c3%bc"), Ok("\u{fc}".to_owned()), "lower-case hex");
            assert_eq!(decode(""), Ok(String::new()));
            assert_eq!(decode("ab%"), Err(DecodeError::BadEscape(2)));
            assert_eq!(decode("ab%4"), Err(DecodeError::BadEscape(2)));
            assert_eq!(decode("%zz"), Err(DecodeError::BadEscape(0)));
            assert_eq!(
                decode("%+1"),
                Err(DecodeError::BadEscape(0)),
                "`+1` is not hex"
            );
            assert_eq!(decode("%FF"), Err(DecodeError::InvalidUtf8));
        });
    }

    #[test]
    fn a_decode_never_panics_on_short_texts() {
        crate::support::within(60, "a_decode_never_panics_on_short_texts", || {
            // Technique: small-scope exhaustive testing. Every text of up to four
            // characters over the alphabet (22 621 texts) goes through `decode`.
            for text in all_texts(4) {
                let result = catch_unwind(|| decode(&text));
                assert!(
                    result.is_ok(),
                    "decode({text:?}) panicked; it must return Ok or Err"
                );
            }
        });
    }

    #[test]
    fn a_encode_then_decode_gives_the_text_back() {
        crate::support::within(60, "a_encode_then_decode_gives_the_text_back", || {
            for text in all_texts(4) {
                assert_eq!(
                    decode(&encode(&text)),
                    Ok(text.clone()),
                    "round trip failed for {text:?} (encoded: {:?})",
                    encode(&text)
                );
            }
        });
    }

    #[test]
    fn b_arbitrary_respects_the_limits() {
        for seed in 0..500 {
            for len in [0, 1, 2, 7, 40, 200] {
                let bytes = noise(seed, len);
                let query = Query::arbitrary(&mut Unstructured::new(&bytes))
                    .expect("running out of bytes is not an error");
                assert!(query.pairs.len() <= 4, "at most 4 pairs: {query:?}");
                for (key, value) in &query.pairs {
                    for text in [key, value] {
                        assert!(text.chars().count() <= 6, "at most 6 characters: {text:?}");
                        assert!(
                            text.chars().all(|c| ALPHABET.contains(&c)),
                            "only characters of ALPHABET: {text:?}"
                        );
                    }
                }
                let again = Query::arbitrary(&mut Unstructured::new(&bytes)).unwrap();
                assert_eq!(query, again, "the same bytes must give the same query");
            }
        }
        assert_eq!(
            Query::arbitrary(&mut Unstructured::new(&[])).unwrap(),
            Query::default(),
            "no bytes at all give the smallest query"
        );
    }

    #[test]
    fn b_arbitrary_reaches_every_shape() {
        // Technique: a generator that always builds the same shape, or never picks
        // the special characters, makes the fuzzer blind however long it runs.
        let mut sizes = BTreeSet::new();
        let mut characters = BTreeSet::new();
        let mut distinct = BTreeSet::new();
        for seed in 0..500 {
            let bytes = noise(seed, 64);
            let query = Query::arbitrary(&mut Unstructured::new(&bytes)).unwrap();
            sizes.insert(query.pairs.len());
            for (key, value) in &query.pairs {
                characters.extend(key.chars());
                characters.extend(value.chars());
            }
            distinct.insert(format!("{query:?}"));
        }
        assert_eq!(
            sizes,
            BTreeSet::from([0, 1, 2, 3, 4]),
            "every number of pairs from 0 to 4 must occur"
        );
        assert_eq!(
            characters,
            BTreeSet::from(ALPHABET),
            "every character of ALPHABET must occur"
        );
        assert!(
            distinct.len() >= 300,
            "500 different inputs gave only {} different queries",
            distinct.len()
        );
    }

    fn sample_queries() -> Vec<Query> {
        (0..400)
            .map(|seed| {
                let bytes = noise(seed, 64);
                Query::arbitrary(&mut Unstructured::new(&bytes)).unwrap()
            })
            .collect()
    }

    #[test]
    fn c_harness_accepts_the_fixed_codec() {
        for query in sample_queries() {
            fuzz_roundtrip(&query, FIXED);
        }
        fuzz_roundtrip(&Query::default(), FIXED);
    }

    #[test]
    fn c_harness_reports_a_wrong_codec() {
        // Technique: this codec never panics and never returns Err. It is simply
        // wrong: a literal `+` comes back as a space. Only a harness that compares
        // the result with the input can report that.
        let quiet_but_wrong = Codec {
            encode: buggy::encode,
            decode,
        };
        let found = sample_queries()
            .iter()
            .any(|query| catch_unwind(|| fuzz_roundtrip(query, quiet_but_wrong)).is_err());
        assert!(
            found,
            "the harness accepted a codec that changes `+` into a space. \
             (Panic messages printed above this line are expected once it works.)"
        );

        let decode_fails = Codec {
            encode,
            decode: |_| Err(DecodeError::InvalidUtf8),
        };
        let one_pair = Query {
            pairs: vec![("a".to_owned(), "b".to_owned())],
        };
        assert!(
            catch_unwind(|| fuzz_roundtrip(&one_pair, decode_fails)).is_err(),
            "a decoder that rejects text the encoder produced is a finding too"
        );
    }

    /// LABS 5.1 and 5.2 (need nightly and cargo-fuzz; see LABS.md). Run with:
    /// `cargo test --manifest-path mastery/m9_toolbox/Cargo.toml lab_l05 -- --ignored`
    #[test]
    #[ignore = "LAB: needs cargo-fuzz; see LABS.md"]
    fn lab_l05_decode_crash_is_real_and_minimal() {
        let bytes = lab_decode_crash();
        let input = std::str::from_utf8(bytes)
            .expect("the decode target only passes valid UTF-8 on, so the crash input is UTF-8");
        assert!(
            catch_unwind(|| buggy::decode(input)).is_err(),
            "buggy::decode({input:?}) does not panic, so this is not the crash"
        );
        assert!(
            bytes.len() <= 4,
            "{bytes:?} is {} bytes long; `cargo fuzz tmin` gets this crash down to 4",
            bytes.len()
        );
        assert!(
            matches!(decode(input), Err(DecodeError::BadEscape(_))),
            "the repaired decoder must reject {input:?} with BadEscape"
        );
    }

    #[test]
    #[ignore = "LAB: needs cargo-fuzz; see LABS.md"]
    fn lab_l05_roundtrip_counterexample_is_real_and_minimal() {
        let value = lab_roundtrip_counterexample();
        let query = Query {
            pairs: vec![(String::new(), value.to_owned())],
        };
        assert!(
            catch_unwind(|| fuzz_decode(value.as_bytes(), BUGGY)).is_ok(),
            "this input crashes the decoder; that is lab 5.1, not 5.2"
        );
        assert_ne!(
            parse_query(&format_query(&query, buggy::encode), buggy::decode),
            Ok(query.clone()),
            "the shipped codec round-trips {value:?} correctly, so this is not a counterexample"
        );
        assert!(
            value.chars().count() <= 2,
            "{value:?} is not minimal; one character is enough"
        );
        assert_eq!(
            parse_query(&format_query(&query, encode), decode),
            Ok(query),
            "the repaired codec must round-trip {value:?}"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    ///
    /// `Unstructured` wraps the fuzzer's bytes and hands them out piece by piece.
    #[test]
    fn p_predictions() {
        // P1. Two bytes are left, and a `u32` needs four. `arbitrary` does not fail:
        // it fills the missing bytes with zeros and reads little-endian.
        let mut u = Unstructured::new(&[0x01, 0x02]);
        let p1: u32 = guess();
        assert_eq!(
            p1,
            u32::arbitrary(&mut u).unwrap(),
            "P1: short input gives small values, not errors"
        );

        // P2. How many bytes are left after that call?
        let p2: usize = guess();
        assert_eq!(p2, u.len(), "P2: the call used up what was there");

        // P3. No bytes at all: which number does `int_in_range(10..=20)` return?
        let mut empty = Unstructured::new(&[]);
        let p3: u8 = guess();
        assert_eq!(
            p3,
            empty.int_in_range(10..=20).unwrap(),
            "P3: exhausted input gives the start of the range"
        );

        // P4. `fuzz_decode` gets the bytes FF FE with the shipped codec, whose
        // decoder panics on some inputs. Does this call panic?
        let p4: bool = guess();
        assert_eq!(
            p4,
            catch_unwind(|| fuzz_decode(&[0xFF, 0xFE], BUGGY)).is_err(),
            "P4: bytes that are not UTF-8 never reach the decoder in this target"
        );
    }
}
