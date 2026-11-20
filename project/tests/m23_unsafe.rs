//! OPTIONAL MILESTONE m23 — A fast byte search, and when `unsafe` earns its place
//! (Chapters 23–24)
//! Run: cargo test --test m23_unsafe
//! Optional comparison: cargo test --test m23_unsafe -- --ignored c_
//! Measure: cargo test --release --test m23_unsafe -- --ignored --nocapture d_measure
//! Miri (from the project directory): cargo +nightly-2026-11-20 miri test --test m23_unsafe
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: how can eight bytes be loaded into a
//! `u64` without `unsafe`? Which raw reads would be out of bounds at the end of the
//! slice, and what makes an unaligned read legal?
//! IMPLEMENT `src/fast.rs`: `find_byte` in safe code, then `split_ascii`. Write a
//! `// SAFETY:` comment for every `unsafe` block before running anything.
//! OPTIONAL: `find_byte_unaligned`, the same search with raw pointer reads. Its tests
//! and the measurement are `#[ignore]`d so that the milestone is green without them.
//! Measure in release mode, then decide which version the program should call.
//! REFACTOR (optional): use the version you chose in `src/scan.rs` to find `=`, `"`,
//! and `\`. Every earlier milestone must stay green: `cargo test --tests`.
//! EXPLAIN afterward: your safety argument for `split_ascii`; what Miri checks and
//! what it cannot prove; and what your measurement says about `find_byte_unaligned`.

mod common;

use common::borrows_from;
#[allow(unused_imports)] // unused once every prediction is answered
use common::guess;
use loglens::fast::{find_byte, find_byte_unaligned, split_ascii};
use std::hint::black_box;
use std::time::{Duration, Instant};

type Search = fn(&[u8], u8) -> Option<usize>;

fn naive(haystack: &[u8], needle: u8) -> Option<usize> {
    haystack.iter().position(|&b| b == needle)
}

/// Every length up to 40, every alignment of the slice start, and the needle
/// absent or at every position, among bytes that differ from it in one bit.
fn agrees_with_a_plain_search(name: &str, find: Search) {
    let backing: Vec<u8> = (0..64).collect();
    for needle in [b'=', 0x00, 0x7f, 0x80, 0xff] {
        for offset in 0..8 {
            for len in 0..=40 {
                let mut hay: Vec<u8> = backing[..offset + len]
                    .iter()
                    .map(|&i| needle ^ (1 << (i % 8)))
                    .collect();
                let slice = &hay[offset..];
                assert_eq!(
                    find(slice, needle),
                    None,
                    "{name}: needle {needle:#x} is absent (offset {offset}, len {len})"
                );
                for pos in 0..len {
                    hay[offset + pos] = needle;
                    let slice = &hay[offset..];
                    assert_eq!(
                        find(slice, needle),
                        naive(slice, needle),
                        "{name}: needle {needle:#x}, offset {offset}, len {len}, pos {pos}"
                    );
                    hay[offset + pos] = needle ^ 1;
                }
            }
        }
    }
}

fn returns_the_first_match(name: &str, find: Search) {
    let hay = b"key=value=more==";
    assert_eq!(find(hay, b'='), Some(3), "{name}: the first of several");
    assert_eq!(find(&hay[4..], b'='), Some(5), "{name}");
    assert_eq!(find(b"", b'='), None, "{name}: empty input");
    let long = [b"abcdefgh".repeat(10), b"=".to_vec()].concat();
    assert_eq!(find(&long, b'='), Some(80), "{name}: a match in the tail");
    let two = [b"abcdefgh".repeat(3), b"ab=d=fgh".to_vec()].concat();
    assert_eq!(
        find(&two, b'='),
        Some(26),
        "{name}: two matches in one word; report the lower index"
    );
    let high: Vec<u8> = (0x80..=0xff).collect();
    assert_eq!(find(&high, 0x00), None, "{name}: no false match on 0x80");
    assert_eq!(find(&high, 0xff), Some(127), "{name}");
}

#[test]
fn a_find_byte_matches_a_plain_search() {
    agrees_with_a_plain_search("find_byte", find_byte);
}

#[test]
fn a_find_byte_returns_the_first_match() {
    returns_the_first_match("find_byte", find_byte);
}

#[test]
fn b_split_ascii() {
    assert_eq!(split_ascii("key=value=x", b'='), Some(("key", "value=x")));
    assert_eq!(split_ascii("é=ü", b'='), Some(("é", "ü")));
    assert_eq!(split_ascii("=", b'='), Some(("", "")));
    assert_eq!(split_ascii("ab=", b'='), Some(("ab", "")));
    assert_eq!(split_ascii("no equals", b'='), None);
    assert_eq!(split_ascii("", b' '), None);
    assert_eq!(split_ascii("🦀 crab", b' '), Some(("🦀", "crab")));
}

#[test]
fn b_split_ascii_borrows() {
    let s = String::from("ключ=значение");
    let (k, v) = split_ascii(&s, b'=').unwrap();
    assert!(
        borrows_from(&s, k) && borrows_from(&s, v),
        "both halves must be slices of the input, not copies"
    );
    assert_eq!(
        v.as_ptr(),
        s[9..].as_ptr(),
        "the second half starts right after the `=` (byte 9)"
    );
}

#[test]
#[should_panic(expected = "ASCII")]
fn b_split_ascii_rejects_non_ascii_bytes() {
    // 0xA9 is the second byte of "é": splitting there would break UTF-8.
    split_ascii("é", 0xa9);
}

#[test]
#[ignore = "optional comparison (m23 C): run with `-- --ignored c_`"]
fn c_unaligned_matches_a_plain_search() {
    agrees_with_a_plain_search("find_byte_unaligned", find_byte_unaligned);
}

#[test]
#[ignore = "optional comparison (m23 C): run with `-- --ignored c_`"]
fn c_unaligned_returns_the_first_match() {
    returns_the_first_match("find_byte_unaligned", find_byte_unaligned);
}

/// The best time of several rounds; each round searches every haystack once.
fn best_time(find: Search, haystacks: &[Vec<u8>], rounds: u32) -> Duration {
    (0..rounds)
        .map(|_| {
            let start = Instant::now();
            for hay in haystacks {
                black_box(find(black_box(hay), black_box(b'=')));
            }
            start.elapsed()
        })
        .min()
        .unwrap_or_default()
}

/// Not a check: it prints how long each version takes, so that the choice
/// between them rests on numbers from your machine. Run it in release mode.
#[test]
#[ignore = "a measurement: run with `--release -- --ignored --nocapture d_measure`"]
fn d_measure() {
    // Miri interprets every instruction, so give it a tiny workload.
    let (long_len, short_count, rounds) = if cfg!(miri) {
        (64, 4, 1)
    } else {
        (1 << 20, 20_000, 30)
    };
    // One long haystack without the needle (the worst case), and many
    // log-line-sized ones with the needle near the end (the common case).
    let long = vec![vec![b'a'; long_len]];
    let short: Vec<Vec<u8>> = (0..short_count)
        .map(|i| {
            let mut line = vec![b'a'; 24 + i % 40];
            let last = line.len() - 1;
            line[last] = b'=';
            line
        })
        .collect();
    let mut versions: Vec<(&str, Search)> = vec![("plain loop", naive), ("find_byte", find_byte)];
    // The optional version may still be a `todo!()`.
    if std::panic::catch_unwind(|| find_byte_unaligned(b"=", b'=')).is_ok() {
        versions.push(("find_byte_unaligned", find_byte_unaligned));
    } else {
        println!("find_byte_unaligned is not written yet; measuring the others");
    }
    if cfg!(debug_assertions) {
        println!("note: this is a debug build; only release numbers mean anything");
    }
    println!(
        "{:<22}{:>18}{:>24}",
        "", "1 MiB, no match", "20000 short lines"
    );
    for (name, find) in versions {
        assert_eq!(find(&long[0], b'='), None, "{name}");
        assert_eq!(find(&short[0], b'='), Some(23), "{name}");
        let (a, b) = (
            best_time(find, &long, rounds),
            best_time(find, &short, rounds),
        );
        println!("{name:<22}{a:>18.1?}{b:>24.1?}");
    }
}

/// P. Predict BEFORE running: replace each `guess()` with your answer.
/// The sizes assume a 64-bit target.
#[test]
fn p_predictions() {
    // P1. The size of Option<&u8>.
    let p1: usize = guess();
    assert_eq!(
        p1,
        size_of::<Option<&u8>>(),
        "P1: references are never null"
    );

    // P2. Eight bytes read as a little-endian u64.
    let p2: u64 = guess();
    assert_eq!(
        p2,
        u64::from_le_bytes(*b"\x01\0\0\0\0\0\0\0"),
        "P2: the first byte is the lowest"
    );

    // P3. Do all bytes of "é" have the high bit set?
    let p3: bool = guess();
    assert_eq!(
        p3,
        "é".as_bytes().iter().all(|b| b & 0x80 != 0),
        "P3: no byte of a multi-byte character is ASCII"
    );

    // P4. A byte repeated in every lane of a u64, in hexadecimal.
    let p4: u64 = guess();
    assert_eq!(
        p4,
        0x0101_0101_0101_0101u64 * 0x3d,
        "P4: the broadcast pattern"
    );

    // P5. Nineteen bytes as chunks of eight: how many chunks, how long a tail?
    let bytes = [0u8; 19];
    let (chunks, tail) = bytes.as_chunks::<8>();
    let p5: (usize, usize) = guess();
    assert_eq!(
        p5,
        (chunks.len(), tail.len()),
        "P5: whole arrays first, then what is left over"
    );
}
