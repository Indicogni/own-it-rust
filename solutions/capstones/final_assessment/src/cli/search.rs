//! Searching one input and formatting what it contributes to the output.

use std::fs;
use std::io::{self, Read, Write};

use rgx::Matcher;

use super::args::Mode;
use super::walk::{Input, Source};

/// The options that decide what a line contributes.
#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub mode: Mode,
    pub invert: bool,
    pub line_numbers: bool,
    /// Put `PATH:` before each line.
    pub show_names: bool,
}

/// Everything one input adds to the run. A worker thread fills it in; the
/// main thread prints it when the input's turn has come.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    /// Bytes for standard output, complete lines only.
    pub output: Vec<u8>,
    /// A message for standard error, without the `rgx: ` in front.
    pub error: Option<String>,
    /// Was at least one line selected?
    pub selected: bool,
}

impl Outcome {
    fn failed(label: &str, reason: impl std::fmt::Display) -> Outcome {
        Outcome {
            error: Some(format!("{label}: {reason}")),
            ..Outcome::default()
        }
    }
}

/// Reads and searches `input`.
pub fn search_input(input: &Input, settings: Settings, matcher: &mut Matcher<'_>) -> Outcome {
    let (bytes, must_be_text) = match &input.source {
        Source::Failed(reason) => return Outcome::failed(&input.label, reason),
        Source::File { path, named } => (fs::read(path), *named),
        Source::Stdin => {
            let mut bytes = Vec::new();
            (
                io::stdin().lock().read_to_end(&mut bytes).map(|_| bytes),
                true,
            )
        }
    };
    let bytes = match bytes {
        Ok(bytes) => bytes,
        Err(error) => return Outcome::failed(&input.label, error),
    };
    // Checking the bytes in place makes the `&str` for the whole file
    // without copying it; every line below is a slice of this one buffer.
    let text = match str::from_utf8(&bytes) {
        Ok(text) => text,
        Err(error) if must_be_text => {
            return Outcome::failed(
                &input.label,
                format_args!("invalid UTF-8 at byte {}", error.valid_up_to()),
            );
        }
        // A file that a directory walk came across and that is not text is
        // passed over without a word (SPEC 7.4).
        Err(_) => return Outcome::default(),
    };
    let mut output = Vec::new();
    let selected = search_text(text, &input.label, settings, matcher, &mut output);
    Outcome {
        output,
        error: None,
        selected,
    }
}

/// Appends to `output` what `text` contributes, and reports whether any
/// line was selected.
pub fn search_text(
    text: &str,
    label: &str,
    settings: Settings,
    matcher: &mut Matcher<'_>,
    output: &mut Vec<u8>,
) -> bool {
    let name = settings.show_names.then_some(label);
    let prefix = |output: &mut Vec<u8>, number: usize| {
        if let Some(name) = name {
            output.extend_from_slice(name.as_bytes());
            output.push(b':');
        }
        if settings.line_numbers {
            // Writing to a `Vec` cannot fail.
            let _ = write!(output, "{number}:");
        }
    };
    let mut selected: u64 = 0;
    for (number, line) in rgx::lines(text) {
        match settings.mode {
            Mode::OnlyMatching => {
                // One pass finds out both whether the line is selected and
                // what to print. Empty matches select but print nothing.
                let mut any = false;
                for found in matcher.find_iter(line) {
                    any = true;
                    if !found.is_empty() {
                        prefix(output, number);
                        output.extend_from_slice(&line.as_bytes()[found]);
                        output.push(b'\n');
                    }
                }
                selected += u64::from(any);
            }
            _ if matcher.is_match(line) == settings.invert => {}
            Mode::Lines => {
                selected += 1;
                prefix(output, number);
                output.extend_from_slice(line.as_bytes());
                output.push(b'\n');
            }
            Mode::Count => selected += 1,
            Mode::FilesWithMatches => {
                // One selected line settles it: the rest need not be read.
                selected += 1;
                break;
            }
        }
    }
    match settings.mode {
        Mode::Count => {
            if let Some(name) = name {
                output.extend_from_slice(name.as_bytes());
                output.push(b':');
            }
            let _ = writeln!(output, "{selected}");
        }
        Mode::FilesWithMatches if selected > 0 => {
            output.extend_from_slice(label.as_bytes());
            output.push(b'\n');
        }
        _ => {}
    }
    selected > 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use rgx::Regex;

    fn run(pattern: &str, settings: Settings, text: &str) -> (String, bool) {
        let regex = Regex::new(pattern).unwrap();
        let mut output = Vec::new();
        let selected = search_text(text, "f.txt", settings, &mut regex.matcher(), &mut output);
        (String::from_utf8(output).unwrap(), selected)
    }

    const PLAIN: Settings = Settings {
        mode: Mode::Lines,
        invert: false,
        line_numbers: false,
        show_names: false,
    };
    const TEXT: &str = "one fish\ntwo fish\nred\n\nblue fish";

    #[test]
    fn lines_with_every_prefix() {
        assert_eq!(run("o", PLAIN, TEXT), ("one fish\ntwo fish\n".into(), true));
        let numbered = Settings {
            line_numbers: true,
            ..PLAIN
        };
        assert_eq!(run("^$|red", numbered, TEXT).0, "3:red\n4:\n");
        let named = Settings {
            show_names: true,
            ..numbered
        };
        assert_eq!(run("blue", named, TEXT).0, "f.txt:5:blue fish\n");
        assert_eq!(run("green", named, TEXT), (String::new(), false));
    }

    #[test]
    fn inverted_counted_and_listed() {
        let inverted = Settings {
            invert: true,
            ..PLAIN
        };
        assert_eq!(run("fish", inverted, TEXT).0, "red\n\n");
        let count = Settings {
            mode: Mode::Count,
            ..PLAIN
        };
        assert_eq!(run("fish", count, TEXT), ("3\n".into(), true));
        assert_eq!(
            run(
                "cat",
                Settings {
                    show_names: true,
                    ..count
                },
                TEXT
            ),
            ("f.txt:0\n".into(), false)
        );
        let list = Settings {
            mode: Mode::FilesWithMatches,
            ..PLAIN
        };
        assert_eq!(run("fish", list, TEXT), ("f.txt\n".into(), true));
        assert_eq!(run("cat", list, TEXT), (String::new(), false));
    }

    #[test]
    fn only_matching_prints_non_empty_matches() {
        let only = Settings {
            mode: Mode::OnlyMatching,
            line_numbers: true,
            ..PLAIN
        };
        assert_eq!(
            run("[a-z]*sh|o", only, TEXT).0,
            "1:o\n1:fish\n2:o\n2:fish\n5:fish\n"
        );
        // `x*` matches the empty string in every line: all are selected,
        // nothing is printed.
        assert_eq!(run("x*", only, TEXT), (String::new(), true));
    }
}
