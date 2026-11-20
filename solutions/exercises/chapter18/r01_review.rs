//! REVIEW 01 — Spaced retrieval: earlier chapters inside Chapter 18
//! Run: cargo test --test chapter18 r01_
//!
//! Do this WITHOUT reopening earlier chapters or their hints. Struggling to
//! recall is what makes the knowledge stick; look things up only after a real
//! attempt, and note in NOTES.md anything you had to look up.
//!
//! Each task mixes an earlier idea (Unicode scalars, map entries and checked
//! arithmetic, error enums with `?`) with this chapter's readers.

use std::collections::BTreeMap;
use std::fmt;
use std::io::{self, BufRead, Read};
use std::num::ParseIntError;

/// A. (Ch17 Unicode + Ch18 buffered lines) Return the 1-based number and text
/// of the line with the most Unicode scalars (ties go to the earliest), or
/// None for empty input. Line endings (LF or CRLF) are not part of a line.
/// Propagate read errors, including invalid UTF-8.
fn longest_line<R: BufRead>(input: R) -> io::Result<Option<(usize, String)>> {
    // (scalar count, line number, text) of the best line so far.
    let mut best: Option<(usize, usize, String)> = None;
    for (index, line) in input.lines().enumerate() {
        let line = line?;
        let scalars = line.chars().count();
        // Strictly greater, so the earliest of equal lines stays.
        if best.as_ref().is_none_or(|(most, _, _)| scalars > *most) {
            best = Some((scalars, index + 1, line));
        }
    }
    Ok(best.map(|(_, number, line)| (number, line)))
}

/// B. (Ch3 checked arithmetic + Ch16 entries + Ch18 BufRead) Each nonblank
/// line is "NAME COUNT", two fields separated by whitespace. Sum the counts
/// per name. Any other shape, a count that is not a u32, or a sum that would
/// overflow u32 is an InvalidData error. Propagate read errors.
fn tally<R: BufRead>(input: R) -> io::Result<BTreeMap<String, u32>> {
    let invalid = |what: &str| io::Error::new(io::ErrorKind::InvalidData, what.to_string());
    let mut totals = BTreeMap::new();
    for line in input.lines() {
        let line = line?;
        let mut fields = line.split_whitespace();
        let (name, count) = match (fields.next(), fields.next(), fields.next()) {
            (None, _, _) => continue, // a blank line
            (Some(name), Some(count), None) => (name, count),
            _ => return Err(invalid("expected NAME COUNT")),
        };
        let count: u32 = count.parse().map_err(|_| invalid("COUNT is not a u32"))?;
        let total = totals.entry(name.to_string()).or_insert(0u32);
        *total = total
            .checked_add(count)
            .ok_or_else(|| invalid("sum overflows u32"))?;
    }
    Ok(totals)
}

#[derive(Debug)]
enum LoadError {
    Io(io::Error),
    Parse { line: usize, source: ParseIntError },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Io(e) => write!(f, "read failed: {e}"),
            LoadError::Parse { line, source } => write!(f, "line {line}: {source}"),
        }
    }
}

/// C1. (Ch7 error sources) Expose the underlying error of either variant.
impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LoadError::Io(e) => Some(e),
            LoadError::Parse { source, .. } => Some(source),
        }
    }
}

/// C2. (Ch7 From for `?`) Let `?` turn an io::Error into a LoadError.
impl From<io::Error> for LoadError {
    fn from(err: io::Error) -> Self {
        LoadError::Io(err)
    }
}

/// C3. (Ch7 `?` + Ch18 Read) Read all of `input` as UTF-8 text holding one
/// number per line (trimmed; blank lines skipped) and parse each as u32.
/// Read and UTF-8 failures become LoadError::Io through `?`; a bad number is
/// LoadError::Parse with its 1-based line.
fn load_numbers<R: Read>(input: R) -> Result<Vec<u32>, LoadError> {
    let mut input = input;
    let mut text = String::new();
    // `?` converts the io::Error (also the InvalidData one for bad UTF-8)
    // through the From impl above.
    input.read_to_string(&mut text)?;
    let mut numbers = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let number = line.parse().map_err(|source| LoadError::Parse {
            line: index + 1,
            source,
        })?;
        numbers.push(number);
    }
    Ok(numbers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::within;
    use std::collections::VecDeque;
    use std::error::Error;

    // The tests run your code through `within`, so a read loop that never
    // notices the end of input fails with a message instead of hanging.
    const LOOP: &str = "(does a hand-written read loop stop at end of input?)";

    #[test]
    fn a_longest_line_counts_scalars() {
        within(5, &format!("longest_line {LOOP}"), || {
            let text = "ab\r\néééé\n🦀🦀🦀🦀\nabcd";
            assert_eq!(
                longest_line(text.as_bytes()).unwrap(),
                Some((2, "éééé".into()))
            );
            assert_eq!(longest_line(&b""[..]).unwrap(), None);
            assert_eq!(
                longest_line(&b"\n\n"[..]).unwrap(),
                Some((1, String::new()))
            );
            assert_eq!(
                longest_line(&b"ok\n\xff\n"[..]).unwrap_err().kind(),
                io::ErrorKind::InvalidData
            );
        });
    }

    #[test]
    fn b_tally_sums_per_name() {
        within(5, &format!("tally {LOOP}"), || {
            let totals = tally(&b"ann 3\nbo\t1\n\n  ann   4  \n"[..]).unwrap();
            assert_eq!(
                totals,
                BTreeMap::from([("ann".into(), 7), ("bo".into(), 1)])
            );
            assert!(tally(&b""[..]).unwrap().is_empty());
            for bad in ["ann", "ann 1 2", "ann x", "ann -1", "a 4294967295\na 1"] {
                assert_eq!(
                    tally(bad.as_bytes()).unwrap_err().kind(),
                    io::ErrorKind::InvalidData,
                    "{bad:?}"
                );
            }
            assert_eq!(tally(&b"a 4294967295"[..]).unwrap()["a"], u32::MAX);
        });
    }

    #[test]
    fn c_load_numbers_reports_line_and_source() {
        within(5, &format!("load_numbers {LOOP}"), || {
            assert_eq!(load_numbers(&b" 7\n\n42\n"[..]).unwrap(), [7, 42]);
            let err = load_numbers(&b"1\n\nx2\n"[..]).unwrap_err();
            assert!(matches!(err, LoadError::Parse { line: 3, .. }), "{err:?}");
            assert_eq!(err.to_string(), "line 3: invalid digit found in string");
            assert!(err.source().is_some(), "the ParseIntError is the source");
        });
    }

    #[test]
    fn c_io_errors_convert_through_question_mark() {
        within(5, &format!("load_numbers {LOOP}"), || {
            let err = load_numbers(&b"1\n\xff"[..]).unwrap_err();
            assert!(matches!(&err, LoadError::Io(e) if e.kind() == io::ErrorKind::InvalidData));
            let source = err.source().expect("the io::Error is the source");
            assert!(source.downcast_ref::<io::Error>().is_some());
        });
    }

    /// P. Earlier-chapter recall: predict each value BEFORE running.
    #[test]
    fn p_predictions() {
        // P1 (Ch17). Byte length of "héllo", and the String reversed by scalars.
        let p1: (usize, String) = (6, "olléh".to_string());
        assert_eq!(
            p1,
            ("héllo".len(), "héllo".chars().rev().collect()),
            "P1: bytes and scalars"
        );

        // P2 (Ch16). VecDeque after rotate_right(1).
        let mut q = VecDeque::from([1, 2, 3]);
        q.rotate_right(1);
        let p2: Vec<i32> = vec![3, 1, 2];
        assert_eq!(p2, Vec::from(q), "P2: rotation");

        // P3 (Ch15). skip_while then count; and a product.
        let p3: (usize, i32) = (2, 6);
        assert_eq!(
            p3,
            (
                [1, 2, 3, 4].iter().skip_while(|&&x| x < 3).count(),
                (1..=3).product()
            ),
            "P3: adapters and consumers"
        );

        // P4 (Ch11). Size of a trait-object reference and of a boxed slice,
        // in machine words.
        let p4: (usize, usize) = (2, 2);
        assert_eq!(
            p4,
            (
                size_of::<&dyn fmt::Debug>() / size_of::<usize>(),
                size_of::<Box<[u8]>>() / size_of::<usize>()
            ),
            "P4: fat pointers"
        );
    }
}
