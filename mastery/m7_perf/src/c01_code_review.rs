//! CODE REVIEW 01 — A pull request titled "make it faster"
//! Run: cargo test --manifest-path mastery/m7_perf/Cargo.toml c01_
//! Then: cargo clippy --manifest-path mastery/m7_perf/Cargo.toml --all-targets
//!
//! A teammate tuned three functions and wrote in the description: "chunked
//! loop, buffer reuse, same output". The code compiles, it looks reasonable,
//! and some of its tests pass. Your job:
//! 1. Before running anything, read the code and write a review in NOTES.md:
//!    list every bug, every place where the "optimization" does not do what
//!    its comment says, and every error that is lost.
//! 2. Run the tests. Compare what they found with your review.
//! 3. Fix it. You MAY change parameter types where the doc comment says so;
//!    the tests are written to accept the better signature.
//! 4. Run clippy and resolve its warnings for this file.

use std::io::{self, Write};

/// Add up `data` as little-endian 32-bit words, wrapping around. When the
/// length is not a multiple of four, the last word is made of the remaining
/// bytes followed by zero bytes. Keep this signature.
pub fn checksum_words(data: &[u8]) -> u32 {
    let mut sum = 0u32;
    // Faster: whole words, and no bounds checks in the loop.
    for word in data.chunks_exact(4) {
        sum = sum.wrapping_add(u32::from_le_bytes([word[0], word[1], word[2], word[3]]));
    }
    sum
}

/// Render every group as its numbers joined by `+`: `[1, 2, 3]` gives
/// `"1+2+3"`, and an empty group gives `""`. You may change the parameter
/// type.
pub fn render_groups(groups: &Vec<Vec<u32>>) -> Vec<String> {
    let mut rendered = Vec::new();
    // Faster: reuse one buffer instead of allocating for every group.
    let mut buffer = String::new();
    for i in 0..groups.len() {
        for j in 0..groups[i].len() {
            if j > 0 {
                buffer.push('+');
            }
            buffer.push_str(&groups[i][j].to_string());
        }
        rendered.push(buffer.clone());
    }
    rendered
}

/// Write one line `key=value` per pair to `out`, in order. `out` is slow:
/// every call of its `write` costs a system call. Every error of `out` must
/// reach the caller. Keep this signature.
pub fn export<W: Write>(out: W, pairs: &[(String, u64)]) -> io::Result<()> {
    let mut out = out;
    for (key, value) in pairs {
        let line = format!("{}={}\n", key.clone(), value);
        out.write_all(line.as_bytes())?;
    }
    // Flushing is best effort.
    let _ = out.flush();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::l07_io::Sink;
    use crate::support::alloc::count;

    fn pairs(n: u64) -> Vec<(String, u64)> {
        (0..n).map(|i| (format!("key{i}"), i * 3)).collect()
    }

    #[test]
    fn checksum_of_whole_words() {
        assert_eq!(checksum_words(&[]), 0);
        assert_eq!(checksum_words(&[1, 0, 0, 0]), 1);
        assert_eq!(checksum_words(&[1, 0, 0, 0, 0, 1, 0, 0]), 257);
        assert_eq!(
            checksum_words(&[0xFF, 0xFF, 0xFF, 0xFF, 2, 0, 0, 0]),
            1,
            "the sum wraps around"
        );
    }

    #[test]
    fn checksum_includes_the_last_partial_word() {
        assert_eq!(
            checksum_words(&[1, 0, 0, 0, 5]),
            6,
            "the fifth byte is a word of its own: 5 followed by three zero bytes"
        );
        assert_eq!(checksum_words(&[7]), 7);
        assert_eq!(checksum_words(&[0, 1, 0]), 256);
        assert_eq!(checksum_words(&[1, 0, 0, 0, 0, 0, 2]), 1 + (2 << 16));
    }

    /// Owned groups. Passing `&groups(..)` compiles whether `render_groups`
    /// takes `&Vec<Vec<u32>>` or a slice.
    fn groups(list: &[&[u32]]) -> Vec<Vec<u32>> {
        list.iter().map(|group| group.to_vec()).collect()
    }

    #[test]
    fn one_group_is_rendered() {
        assert_eq!(render_groups(&groups(&[&[1, 2, 3]])), ["1+2+3"]);
        assert_eq!(render_groups(&groups(&[&[42]])), ["42"]);
        assert!(render_groups(&groups(&[])).is_empty());
    }

    #[test]
    fn every_group_is_rendered_on_its_own() {
        let groups = groups(&[&[1, 2], &[], &[30], &[4, 5, 6]]);
        assert_eq!(
            render_groups(&groups),
            ["1+2", "", "30", "4+5+6"],
            "each result must hold its own group only"
        );
    }

    #[test]
    fn rendering_allocates_per_group_not_per_number() {
        let groups: Vec<Vec<u32>> = (0..100).map(|g| (g..g + 20).collect()).collect();
        let (rendered, allocations) = count(|| render_groups(&groups));
        assert_eq!(rendered.len(), 100);
        assert!(
            allocations <= 100 + 16,
            "render_groups made {allocations} allocations for 100 groups of 20 numbers; the \
             budget is one allocation per group plus 16. Two things cost more than that: a \
             String for each number (`to_string`), and a result String that starts empty \
             and grows step by step. Format the numbers into the reused buffer, then copy \
             the finished text once"
        );
    }

    #[test]
    fn export_writes_every_pair() {
        let mut sink = Sink::new();
        export(&mut sink, &pairs(3)).unwrap();
        assert_eq!(sink.text(), "key0=0\nkey1=3\nkey2=6\n");
        let mut empty = Sink::new();
        export(&mut empty, &[]).unwrap();
        assert_eq!(empty.text(), "");
    }

    #[test]
    fn export_batches_its_writes() {
        let pairs = pairs(1_000);
        let mut sink = Sink::new();
        sink.data.reserve(100_000);
        sink.accepted.reserve(100_000);
        let (result, allocations) = count(|| export(&mut sink, &pairs));
        result.unwrap();
        let bytes = sink.data.len();
        assert!(
            sink.writes <= bytes / 4096 + 2,
            "export called `write` {} times for {bytes} bytes; the budget is {}",
            sink.writes,
            bytes / 4096 + 2
        );
        assert!(
            allocations <= 2,
            "export made {allocations} allocations for 1,000 pairs; no line needs a String \
             of its own, and no key needs a copy"
        );
    }

    #[test]
    fn export_reports_a_failed_flush() {
        let mut sink = Sink {
            fail_flush: true,
            ..Sink::new()
        };
        assert!(
            export(&mut sink, &pairs(3)).is_err(),
            "`out.flush()` failed, so the data may not have arrived; the caller must hear \
             about it"
        );
        let mut sink = Sink {
            fail_write: Some((1, io::ErrorKind::BrokenPipe)),
            ..Sink::new()
        };
        assert!(export(&mut sink, &pairs(3)).is_err());
    }
}
