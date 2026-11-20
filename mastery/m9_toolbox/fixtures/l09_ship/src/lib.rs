//! Counts the lines, words and bytes of text.

/// Line, word and byte counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// Number of `\n` characters.
    pub lines: usize,
    /// Number of whitespace-separated words.
    pub words: usize,
    /// Number of bytes.
    pub bytes: usize,
}

/// Counts `text`.
///
/// ```
/// let counts = shipit::count("one two\nthree\n");
/// assert_eq!((counts.lines, counts.words, counts.bytes), (2, 3, 14));
/// ```
pub fn count(text: &str) -> Counts {
    Counts {
        lines: text.bytes().filter(|byte| *byte == b'\n').count(),
        words: text.split_whitespace().count(),
        bytes: text.len(),
    }
}
