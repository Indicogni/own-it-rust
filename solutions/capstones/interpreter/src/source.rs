//! Source text, spans and diagnostics (SPEC section 11).
//!
//! A [`Span`] is a pair of byte offsets. Lines and columns are computed only
//! when a diagnostic is printed, which happens at most once per run, so no
//! line table is kept.

/// A half-open range of byte offsets into one source text.
///
/// Offsets are `u32` so that a span is 8 bytes: every expression carries
/// one. The parser refuses sources that do not fit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    /// The span from byte `start` up to, not including, byte `end`.
    pub fn new(start: usize, end: usize) -> Span {
        // `parser::parse` rejects sources longer than `u32::MAX` bytes, so
        // the fallback is never used.
        let narrow = |offset: usize| u32::try_from(offset).unwrap_or(u32::MAX);
        Span {
            start: narrow(start),
            end: narrow(end),
        }
    }

    /// The span from the start of `self` to the end of `other`.
    pub fn to(self, other: Span) -> Span {
        Span {
            start: self.start,
            end: other.end,
        }
    }
}

/// One error: where it is and what it says.
///
/// The same type serves syntax errors, name errors and runtime errors. Which
/// of the three it is decides the exit status, and that is recorded by the
/// caller (see `session::Failure`), not here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub span: Span,
    pub message: String,
}

impl Diagnostic {
    pub fn new(span: Span, message: impl Into<String>) -> Diagnostic {
        Diagnostic {
            span,
            message: message.into(),
        }
    }
}

/// A named source text: a script file, or everything typed into the REPL.
#[derive(Clone, Copy, Debug)]
pub struct Source<'a> {
    pub name: &'a str,
    pub text: &'a str,
}

impl Source<'_> {
    /// The three lines of SPEC 11.2, each ending in a line feed:
    ///
    /// ```text
    /// main.tn:3:9: error: division by zero
    ///     let x = 10 / (5 - 5);
    ///             ^^^^^^^^^^^^
    /// ```
    pub fn render(&self, diagnostic: &Diagnostic) -> String {
        let text = self.text;
        // A span always lies on character boundaries of the text it was made
        // from. Clamping keeps this function total for any other input.
        let start = floor_boundary(text, diagnostic.span.start as usize);
        let end = floor_boundary(text, diagnostic.span.end as usize).max(start);

        let line_start = text[..start].rfind('\n').map_or(0, |i| i + 1);
        let line_number = text[..line_start].matches('\n').count() + 1;
        let rest = &text[line_start..];
        let line = &rest[..rest.find('\n').unwrap_or(rest.len())];
        let line = line.strip_suffix('\r').unwrap_or(line);
        let line_end = line_start + line.len();

        let column = text[line_start..start].chars().count() + 1;
        // Carets cover the part of the span on its first line, at least one.
        let marked = text[start..end.clamp(start, line_end.max(start))]
            .chars()
            .count()
            .max(1);

        // A tab is shown as one space, so that a column is one cell and the
        // carets line up whatever the tab width of the terminal is.
        let shown = line.replace('\t', " ");
        format!(
            "{name}:{line_number}:{column}: error: {message}\n    {shown}\n    {pad}{carets}\n",
            name = self.name,
            message = diagnostic.message,
            pad = " ".repeat(column - 1),
            carets = "^".repeat(marked),
        )
    }
}

/// The largest character boundary of `text` that is not after `offset`.
fn floor_boundary(text: &str, offset: usize) -> usize {
    let mut offset = offset.min(text.len());
    while !text.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(text: &str, start: usize, end: usize) -> String {
        Source { name: "t.tn", text }.render(&Diagnostic::new(Span::new(start, end), "boom"))
    }

    #[test]
    fn first_line_first_column() {
        assert_eq!(
            render("abc", 0, 1),
            "t.tn:1:1: error: boom\n    abc\n    ^\n"
        );
    }

    #[test]
    fn later_line_and_wider_span() {
        let text = "one\ntwo three\nfour";
        let start = text.find("three").unwrap();
        assert_eq!(
            render(text, start, start + 5),
            "t.tn:2:5: error: boom\n    two three\n        ^^^^^\n"
        );
    }

    #[test]
    fn columns_count_characters_not_bytes() {
        let text = "\"żółw\" @";
        let at = text.find('@').unwrap();
        assert_eq!(
            render(text, at, at + 1),
            "t.tn:1:8: error: boom\n    \"żółw\" @\n           ^\n"
        );
    }

    #[test]
    fn tabs_are_shown_as_single_spaces() {
        assert_eq!(
            render("\t\tx", 2, 3),
            "t.tn:1:3: error: boom\n      x\n      ^\n"
        );
    }

    #[test]
    fn a_span_over_several_lines_is_marked_to_the_end_of_its_first_line() {
        let text = "f(1,\n  2)";
        assert_eq!(
            render(text, 0, text.len()),
            "t.tn:1:1: error: boom\n    f(1,\n    ^^^^\n"
        );
    }

    #[test]
    fn carriage_returns_are_not_part_of_the_line() {
        let text = "a\r\nbb\r\n";
        assert_eq!(
            render(text, 3, 5),
            "t.tn:2:1: error: boom\n    bb\n    ^^\n"
        );
    }

    #[test]
    fn end_of_input_after_a_final_newline_is_an_empty_line() {
        let text = "x\n";
        assert_eq!(render(text, 2, 2), "t.tn:2:1: error: boom\n    \n    ^\n");
    }

    #[test]
    fn end_of_input_without_a_final_newline_is_one_past_the_line() {
        assert_eq!(
            render("xy", 2, 2),
            "t.tn:1:3: error: boom\n    xy\n      ^\n"
        );
    }

    #[test]
    fn spans_outside_the_text_do_not_panic() {
        assert_eq!(render("é", 1, 99), "t.tn:1:1: error: boom\n    é\n    ^\n");
    }
}
