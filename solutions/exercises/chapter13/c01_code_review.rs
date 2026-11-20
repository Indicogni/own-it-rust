//! CODE REVIEW 01 — A teammate's pull request
//! Run: cargo test --test chapter13 c01_
//! Then: cargo clippy --test chapter13
//!
//! This is the kind of code you will be asked to review at work. It compiles,
//! and some behavior tests pass. Your job:
//! 1. Before running anything, read the code and write a review in NOTES.md:
//!    list every bug, every utility trait that is the wrong choice, and every
//!    unnecessary allocation.
//! 2. Run the tests. Compare what they found with your review.
//! 3. Fix it. You MAY change parameter types, return types, and which
//!    conversion trait is implemented where the doc comment says so; the
//!    tests are written to accept the better signature.
//! 4. Run clippy and resolve its warnings for this file.

use std::borrow::Cow;

/// A TCP port, 1..=65535.
#[derive(Debug, PartialEq, Eq)]
struct Port(u16);

/// Parse a port from text such as " 8080 ". Bad input (not a number, out of
/// range, or 0) must be reported to the caller, never panic. You may change
/// which conversion trait is implemented.
impl TryFrom<&str> for Port {
    type Error = PortError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        // Parsing into u16 rejects text that is not a number and values above 65535.
        match text.trim().parse::<u16>() {
            Ok(0) => Err(PortError::Zero),
            Ok(number) => Ok(Port(number)),
            Err(error) => Err(PortError::NotAPort(error)),
        }
    }
}

/// Why a piece of text is not a valid port.
#[derive(Debug, PartialEq, Eq)]
enum PortError {
    /// Port 0 is reserved.
    Zero,
    /// The text is not a number, or the number does not fit in 16 bits.
    NotAPort(std::num::ParseIntError),
}

/// Convert Windows line endings ("\r\n") to Unix ones ("\n"). Other text is
/// unchanged. You may change the return type.
fn unix_newlines(text: &str) -> Cow<'_, str> {
    // Only the pair "\r\n" changes. Text without it is returned borrowed.
    if text.contains("\r\n") {
        Cow::Owned(text.replace("\r\n", "\n"))
    } else {
        Cow::Borrowed(text)
    }
}

/// Total Unicode scalars across all names. You may change the parameter type.
fn total_chars(names: &[String]) -> usize {
    names.iter().map(|name| name.chars().count()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::alloc::count;
    use std::hint::black_box;

    #[test]
    // `Port::try_from` compiles with the starting `From` impl and with a `TryFrom` impl.
    #[allow(clippy::unnecessary_fallible_conversions)]
    fn a_ports_parse_and_report_bad_input() {
        assert_eq!(Port::try_from(" 8080 ").ok(), Some(Port(8080)));
        assert_eq!(Port::try_from("65535").ok(), Some(Port(65_535)));
        assert!(Port::try_from("http").is_err());
        assert!(Port::try_from("65536").is_err());
        assert!(Port::try_from("0").is_err());
        assert!(Port::try_from("").is_err());
    }

    #[test]
    fn b_only_crlf_pairs_change() {
        assert_eq!(unix_newlines("a\r\nb\r\n"), "a\nb\n");
        assert_eq!(
            unix_newlines("a\rb\r\n"),
            "a\rb\n",
            "a '\\r' that is not followed by '\\n' is ordinary text and must stay"
        );
        assert_eq!(unix_newlines("plain\ntext"), "plain\ntext");
        assert_eq!(unix_newlines(""), "");
    }

    #[test]
    fn b_unchanged_text_is_not_copied() {
        let text = "already\nunix\n".repeat(10);
        let (out, allocations) = count(|| black_box(unix_newlines(black_box(&text))));
        assert_eq!(out, text);
        assert_eq!(
            allocations, 0,
            "borrow the input when nothing changes; do not build a new String"
        );
    }

    #[test]
    fn c_total_counts_scalars() {
        let names = vec!["Zoë".to_string(), "李".to_string(), String::new()];
        assert_eq!(total_chars(&names), 4);
        assert_eq!(total_chars(&Vec::<String>::new()), 0);
    }

    #[test]
    fn c_total_reads_names_without_copying_them() {
        let names: Vec<String> = (0..50).map(|i| format!("name{i}")).collect();
        let (total, allocations) = count(|| total_chars(black_box(&names)));
        assert_eq!(total, 10 * 5 + 40 * 6);
        assert_eq!(
            allocations, 0,
            "counting characters needs no copies; read each name through a reference"
        );
    }
}
