//! LESSON 09 — Capstone: a Unicode-aware report parser
//! Run: cargo test --test chapter17 l09_
//! Read first: "Lesson 09" in CONCEPTS.md (same directory).
//!
//! PREDICT before running: Predict parsing, rejection, byte/scalar counts, and output for
//! non-ASCII names. Decide whether a user-perceived character limit is in scope.
//!
//! IMPLEMENT the documented tasks, then keep the tests and add an edge case.
//! EXPLAIN afterward: Why are byte offsets safe at separator boundaries? What errors should
//! a display-oriented parser preserve rather than ignore?
//!
//! COMPILER / MANUAL LAB (optional; use examples/ch17_scratch.rs):
//! Extend the format with escaped separators and compare a small parser to the isolated
//! regex lab. Explain why this task does not implement Unicode normalization, grapheme
//! segmentation, or bidirectional layout.
//!
//! TRANSFER: invent a new edge case, predict it, then add a check. The capstone is
//! complete when its tests and explanation are done. The lab is optional.

#[derive(Debug, PartialEq, Eq)]
struct Record {
    name: String,
    value: u16,
}
/// A. Parse one line of the form NAME=VALUE. Whitespace (any Unicode
/// whitespace) is allowed around the whole line and around both parts, and is
/// not part of the name or the value. After trimming, the name must not be
/// empty and the value must be accepted by `str::parse::<u16>`. A line without
/// '=' or with more than one '=' is malformed.
/// Return Err("invalid record") for every malformed input.
fn parse_record(line: &str) -> Result<Record, &'static str> {
    const INVALID: &str = "invalid record";
    // With "a=1=2" the value part is "1=2", which is not a number, so an extra
    // '=' is rejected by the parse below.
    let (name, value) = line.trim().split_once('=').ok_or(INVALID)?;
    let name = name.trim();
    if name.is_empty() {
        return Err(INVALID);
    }
    let value = value.trim().parse().map_err(|_| INVALID)?;
    Ok(Record {
        name: name.to_owned(),
        value,
    })
}
/// B. Go through the lines of `text` (LF or CRLF line endings). Skip a line
/// that is empty after trimming, and a comment: a line whose trimmed text
/// starts with '#'. Parse every other line with `parse_record`. Return the
/// valid records in their order, and the line numbers of the rejected lines.
/// Line numbers start at 1 and count every line of `text`, skipped ones too.
fn read_report(text: &str) -> (Vec<Record>, Vec<usize>) {
    let mut records = Vec::new();
    let mut rejected = Vec::new();
    // Number the lines before any of them is skipped, so that a reject keeps
    // its position in the source text.
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match parse_record(line) {
            Ok(record) => records.push(record),
            Err(_) => rejected.push(index + 1),
        }
    }
    (records, rejected)
}
/// C. Render each record as "NAME: VALUE (N scalars)\n", where N is the number
/// of Unicode scalars in the name. The word is always "scalars", also for 1.
/// An empty slice gives empty text.
fn render(records: &[Record]) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    for record in records {
        let scalars = record.name.chars().count();
        // `writeln!` appends to `out` directly; no temporary String per row.
        writeln!(out, "{}: {} ({scalars} scalars)", record.name, record.value)
            .expect("writing to a String cannot fail");
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_parser_preserves_unicode_and_validates_entire_record() {
        assert_eq!(
            parse_record(" é = 42 "),
            Ok(Record {
                name: "é".into(),
                value: 42
            })
        );
        for input in [
            "=1",
            "a=65536",
            "a=-1",
            "a=1=2",
            "missing",
            "a=",
            " \u{2003}=1",
        ] {
            assert_eq!(parse_record(input), Err("invalid record"), "{input}");
        }
    }
    #[test]
    fn b_batch_parser_reports_original_lines() {
        let (rows, bad) = read_report("# title\n é=2\n\ninvalid\n🦀=3\na=1=2\n");
        assert_eq!(
            rows,
            vec![
                Record {
                    name: "é".into(),
                    value: 2
                },
                Record {
                    name: "🦀".into(),
                    value: 3
                }
            ]
        );
        assert_eq!(bad, [4, 6]);
        let (rows, bad) = read_report("  # indented comment\r\nx = 1\r\n\r\n#=2\n");
        assert_eq!(
            rows,
            [Record {
                name: "x".into(),
                value: 1
            }]
        );
        assert!(bad.is_empty());
        assert_eq!(read_report(""), (vec![], vec![]));
    }
    #[test]
    fn c_output_uses_scalar_counts_and_exact_line_endings() {
        assert_eq!(
            render(&[
                Record {
                    name: "e\u{301}".into(),
                    value: 5
                },
                Record {
                    name: "🦀".into(),
                    value: 0
                }
            ]),
            "e\u{301}: 5 (2 scalars)\n🦀: 0 (1 scalars)\n"
        );
        assert_eq!(render(&[]), "");
    }
}
