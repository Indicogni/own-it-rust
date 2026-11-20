//! Milestones m15 (Chapter 15) and m18 (Chapter 18): processing whole inputs.
//!
//! Rules shared by every function here:
//!
//! - Lines are numbered from 1, counting every physical line.
//! - A line that is empty, only whitespace, or whose first non-whitespace
//!   character is `#` is *ignored*: it is neither a record nor an error.
//!   As everywhere in this format, whitespace means ASCII whitespace
//!   (`str::trim_ascii_start`). A line that holds only a no-break space is
//!   not blank: it is a malformed line.
//! - Every other line is either a record or a malformed line. Malformed lines
//!   never stop processing; they are counted and reported with their line
//!   number.

use crate::error::{ErrorKind, ParseError};
use crate::filter::Filter;
use crate::record::{Record, parse_record};
use crate::sink::Sink;
use std::io::{self, BufRead};
use std::ops::AddAssign;

/// The parsed records of a text, one item per line that is not ignored.
///
/// Errors carry their line number ([`ParseError::at_line`]).
#[derive(Debug, Clone)]
pub struct Records<'a> {
    lines: std::str::Lines<'a>,
    line_no: u64,
}

impl<'a> Records<'a> {
    /// Iterate over the records of `text`.
    pub fn new(text: &'a str) -> Self {
        Records {
            lines: text.lines(),
            line_no: 0,
        }
    }
}

/// m15 A. Yield `parse_record` results for the lines that are not ignored,
/// with line numbers on the errors. Records borrow from the text, not from
/// the iterator.
impl<'a> Iterator for Records<'a> {
    type Item = Result<Record<'a>, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let line = self.lines.next()?;
            self.line_no += 1;
            let n = self.line_no;
            let trimmed = line.trim_ascii_start();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            return Some(parse_record(line).map_err(|e| e.at_line(n)));
        }
    }
}

/// Counters for one run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tally {
    /// Lines that parsed as records.
    pub records: u64,
    /// Records that matched the filter (and went to the sink).
    pub matched: u64,
    /// Lines that were neither ignored nor records.
    pub malformed: u64,
}

/// m15 B. Add the other tally's counters to this one (used to merge the
/// results of several files).
impl AddAssign for Tally {
    fn add_assign(&mut self, other: Tally) {
        self.records += other.records;
        self.matched += other.matched;
        self.malformed += other.malformed;
    }
}

/// m15 C. The records of `text` that match `filter`, in order. Malformed
/// lines are skipped silently. Build this from iterator adapters on
/// [`Records`]; do not collect.
pub fn matching<'a>(text: &'a str, filter: &'a Filter) -> impl Iterator<Item = Record<'a>> + 'a {
    Records::new(text)
        .filter_map(Result::ok)
        .filter(move |r| filter.matches(r))
}

/// m15 D. The record with the largest `number(key)` value, ignoring
/// malformed lines and records where the field is missing or not a number.
/// On a tie, return the *first* such record. `None` if there is none.
///
/// Careful: `Iterator::max_by_key` returns the *last* maximum.
pub fn slowest<'a>(text: &'a str, key: &str) -> Option<Record<'a>> {
    Records::new(text)
        .filter_map(Result::ok)
        .filter_map(|r| Some((r.number(key).ok()??, r)))
        .min_by_key(|&(n, _)| std::cmp::Reverse(n))
        .map(|(_, r)| r)
}

/// m15 E. Run a whole text through the filter into the sink.
///
/// For each record: count it, and if it matches, count it as matched and pass
/// it to `sink.accept` (an error from the sink stops processing and is
/// returned). For each malformed line: count it and call `on_error` with the
/// error (which has its line number). Do not call `sink.finish`; the caller
/// may send several inputs to one sink.
pub fn process<S: Sink + ?Sized>(
    text: &str,
    filter: &Filter,
    sink: &mut S,
    on_error: &mut dyn FnMut(ParseError),
) -> io::Result<Tally> {
    let mut tally = Tally::default();
    for (line_no, line) in (1..).zip(text.lines()) {
        handle_line(line.as_bytes(), line_no, filter, sink, on_error, &mut tally)?;
    }
    Ok(tally)
}

/// The per-line work shared by every reader: ignore, parse, count, filter,
/// forward. `bytes` is one line without its line ending.
pub(crate) fn handle_line<S: Sink + ?Sized>(
    bytes: &[u8],
    line_no: u64,
    filter: &Filter,
    sink: &mut S,
    on_error: &mut dyn FnMut(ParseError),
    tally: &mut Tally,
) -> io::Result<()> {
    let line = match std::str::from_utf8(bytes) {
        Ok(line) => line,
        Err(e) => {
            // The prefix was just validated, so the lossy conversion borrows.
            let column = String::from_utf8_lossy(&bytes[..e.valid_up_to()])
                .chars()
                .count()
                + 1;
            tally.malformed += 1;
            on_error(ParseError::new(ErrorKind::InvalidUtf8, column).at_line(line_no));
            return Ok(());
        }
    };
    let trimmed = line.trim_ascii_start();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return Ok(());
    }
    match parse_record(line) {
        Ok(record) => {
            tally.records += 1;
            if filter.matches(&record) {
                tally.matched += 1;
                sink.accept(&record)?;
            }
        }
        Err(e) => {
            tally.malformed += 1;
            on_error(e.at_line(line_no));
        }
    }
    Ok(())
}

/// m18 A. [`process`] for any buffered reader, without reading everything
/// into memory.
///
/// Read one line at a time with `read_until(b'\n', ...)` into a single
/// `Vec<u8>` that you clear and reuse: no allocation per line. Strip the
/// `\n`, and a `\r` right before it. A line that is not valid UTF-8 is
/// malformed with kind [`crate::ErrorKind::InvalidUtf8`], and its column is
/// one past the characters of its longest valid prefix
/// (`Utf8Error::valid_up_to`). For valid UTF-8 input, the tally, the sink's
/// records, and the errors are exactly those of [`process`].
///
/// I/O errors from the reader are returned as they are.
///
/// Refactor: move the per-line work (ignore, parse, count, filter, forward)
/// into one private helper that both `process` and `process_reader` call.
pub fn process_reader<R: BufRead, S: Sink + ?Sized>(
    reader: R,
    filter: &Filter,
    sink: &mut S,
    on_error: &mut dyn FnMut(ParseError),
) -> io::Result<Tally> {
    let mut reader = reader;
    let mut tally = Tally::default();
    let mut buf = Vec::new();
    let mut line_no = 0u64;
    loop {
        buf.clear();
        if reader.read_until(b'\n', &mut buf)? == 0 {
            break;
        }
        line_no += 1;
        let mut bytes = &buf[..];
        if let Some(stripped) = bytes.strip_suffix(b"\n") {
            bytes = stripped.strip_suffix(b"\r").unwrap_or(stripped);
        }
        handle_line(bytes, line_no, filter, sink, on_error, &mut tally)?;
    }
    Ok(tally)
}
