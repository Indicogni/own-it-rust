//! Support code for the tests of this module. You do not need to edit this file.
//!
//! Many answers in this module are words: a module path, a test name, a commit
//! subject. The tests compare a *digest* of your answer with the digest of the model
//! answer, so the answer cannot be read from the test.

use std::path::PathBuf;

/// A 64-bit FNV-1a digest of `answer`, ignoring case, surrounding white space, and
/// the amount of white space between words.
pub fn digest(answer: &str) -> u64 {
    let normalised = answer
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in normalised.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Fails the test unless `answer` has the digest `expected`. `what` names the
/// question in the failure message.
#[track_caller]
pub fn assert_answer(what: &str, answer: &str, expected: u64) {
    assert!(
        !answer.trim().is_empty(),
        "{what}: the answer is still empty"
    );
    assert!(
        digest(answer) == expected,
        "{what}: `{answer}` is not the model answer. Check the spelling and the \
         format the task asks for, then look at the code again."
    );
}

/// The directory of this package (`mastery/m10_contributing`).
pub fn package_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Reads a text file below the package directory. Line endings are normalised to
/// `\n`, so the checks behave the same on every platform.
#[track_caller]
pub fn read(relative: &str) -> String {
    let path = package_dir().join(relative);
    match std::fs::read_to_string(&path) {
        Ok(text) => text.replace("\r\n", "\n"),
        Err(error) => panic!("cannot read {}: {error}", path.display()),
    }
}

/// The body of the Markdown section whose heading line is exactly `heading` (for
/// example `"## Testing"`), up to the next heading of the same or a higher level.
/// Lines inside fenced code blocks are never treated as headings.
pub fn section<'a>(markdown: &'a str, heading: &str) -> Option<&'a str> {
    let level = heading.chars().take_while(|c| *c == '#').count();
    let mut start = None;
    let mut in_code = false;
    let mut offset = 0;
    for line in markdown.split_inclusive('\n') {
        let trimmed = line.trim_end();
        if trimmed.trim_start().starts_with("```") {
            in_code = !in_code;
        } else if !in_code {
            match start {
                None if trimmed.eq_ignore_ascii_case(heading) => {
                    start = Some(offset + line.len());
                }
                Some(from) => {
                    let hashes = trimmed.chars().take_while(|c| *c == '#').count();
                    if hashes > 0 && hashes <= level && trimmed[hashes..].starts_with(' ') {
                        return Some(markdown[from..offset].trim());
                    }
                }
                None => {}
            }
        }
        offset += line.len();
    }
    start.map(|from| markdown[from..].trim())
}

/// Fails the test unless the Markdown text has a non-empty section `heading` without
/// the word `TODO`. Returns the section body.
#[track_caller]
pub fn require_section<'a>(file: &str, markdown: &'a str, heading: &str) -> &'a str {
    let Some(body) = section(markdown, heading) else {
        panic!("{file}: the section `{heading}` is missing")
    };
    assert!(!body.is_empty(), "{file}: the section `{heading}` is empty");
    assert!(
        !body.contains("TODO"),
        "{file}: the section `{heading}` still contains a TODO"
    );
    body
}

/// The list items of a Markdown text: each `- ` bullet with its continuation lines,
/// joined into one line.
pub fn list_items(markdown: &str) -> Vec<String> {
    let mut items: Vec<String> = Vec::new();
    for line in markdown.lines() {
        let trimmed = line.trim();
        if let Some(first) = trimmed.strip_prefix("- ") {
            items.push(first.to_string());
        } else if trimmed.is_empty() || trimmed.starts_with('#') {
            // A blank line or a heading ends the current item.
            items.push(String::new());
        } else if let Some(item) = items.last_mut()
            && !item.is_empty()
        {
            item.push(' ');
            item.push_str(trimmed);
        }
    }
    items.retain(|item| !item.is_empty());
    items
}

/// Whether `text` mentions the issue number as `#N`, not followed by another digit.
pub fn mentions_issue(text: &str, number: u32) -> bool {
    let needle = format!("#{number}");
    text.match_indices(&needle).any(|(at, _)| {
        !text[at + needle.len()..]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit())
    })
}

/// The number of `#[test]` attributes in a source text.
pub fn count_tests(source: &str) -> usize {
    source.matches("#[test]").count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_ignores_case_and_spacing() {
        assert_eq!(
            digest("  Tasklane_Core::vars "),
            digest("tasklane_core::vars")
        );
        assert_eq!(digest("a  b"), digest("a b"));
        assert_ne!(digest("a b"), digest("ab"));
    }

    #[test]
    fn section_stops_at_the_next_heading_of_the_same_level() {
        let text = "# T\n\n## A\none\n\n### A1\ntwo\n\n```\n## not a heading\n```\n\n## B\nthree\n";
        assert_eq!(
            section(text, "## A"),
            Some("one\n\n### A1\ntwo\n\n```\n## not a heading\n```")
        );
        assert_eq!(section(text, "## B"), Some("three"));
        assert_eq!(section(text, "## C"), None);
    }

    #[test]
    fn list_items_join_wrapped_lines() {
        let text = "### Added\n\n- one\n  two (#3)\n- three\n\nnot an item\n";
        assert_eq!(list_items(text), ["one two (#3)", "three"]);
    }

    #[test]
    fn issue_mentions_need_the_exact_number() {
        assert!(mentions_issue("Fixes #14.", 14));
        assert!(!mentions_issue("Fixes #141.", 14));
        assert!(!mentions_issue("Fixes 14.", 14));
    }
}
