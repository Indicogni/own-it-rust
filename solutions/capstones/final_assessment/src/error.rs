//! The error for a pattern that cannot be compiled.

use std::fmt;

/// Why a pattern was rejected, and where.
///
/// [`Display`](fmt::Display) prints only the message of SPEC section 5,
/// without the position, so that a caller can lay out the position as it
/// likes. The `rgx` binary prints the pattern with a caret under it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    kind: ErrorKind,
    offset: usize,
}

/// The kinds of pattern error. Each has one fixed message (SPEC section 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ErrorKind {
    /// `(` without its `)`.
    MissingCloseParen,
    /// `)` without a `(` before it.
    UnmatchedCloseParen,
    /// `(?`: the extended group forms are not part of the language.
    UnsupportedGroupSyntax,
    /// More than [`MAX_NESTING`](crate::MAX_NESTING) groups open at once.
    NestingTooDeep,
    /// `[` without its `]`.
    MissingCloseBracket,
    /// `[]` or `[^]`.
    EmptyClass,
    /// A class range that runs backwards or ends in a set such as `\d`.
    InvalidClassRange,
    /// An unescaped `[` inside a class.
    UnsupportedClassSyntax,
    /// A quantifier with no atom before it.
    NothingToRepeat,
    /// A `{` that does not start `{n}`, `{n,}` or `{n,m}` with `n <= m`.
    InvalidRepetition,
    /// A count above [`MAX_REPEAT`](crate::MAX_REPEAT).
    RepetitionTooLarge,
    /// `*`, `+` or `{n,}` after an expression that can match the empty
    /// string.
    EmptyRepetition,
    /// `\` followed by a character that has no meaning.
    UnknownEscape,
    /// `\` as the last character of the pattern.
    IncompleteEscape,
    /// `\u` that is not followed by `{`, one to six hex digits and `}`, or
    /// that names a value which is not a Unicode scalar value.
    InvalidUnicodeEscape,
    /// The cost of the pattern is above [`MAX_COST`](crate::MAX_COST).
    PatternTooLarge,
}

impl ErrorKind {
    /// The message of SPEC section 5.
    #[must_use]
    pub fn message(self) -> &'static str {
        match self {
            ErrorKind::MissingCloseParen => "missing closing parenthesis",
            ErrorKind::UnmatchedCloseParen => "unmatched closing parenthesis",
            ErrorKind::UnsupportedGroupSyntax => "unsupported group syntax",
            ErrorKind::NestingTooDeep => "nesting too deep",
            ErrorKind::MissingCloseBracket => "missing closing bracket",
            ErrorKind::EmptyClass => "empty character class",
            ErrorKind::InvalidClassRange => "invalid class range",
            ErrorKind::UnsupportedClassSyntax => "unsupported class syntax",
            ErrorKind::NothingToRepeat => "nothing to repeat",
            ErrorKind::InvalidRepetition => "invalid repetition",
            ErrorKind::RepetitionTooLarge => "repetition count too large",
            ErrorKind::EmptyRepetition => "repeated expression can be empty",
            ErrorKind::UnknownEscape => "unknown escape",
            ErrorKind::IncompleteEscape => "incomplete escape",
            ErrorKind::InvalidUnicodeEscape => "invalid Unicode escape",
            ErrorKind::PatternTooLarge => "pattern too large",
        }
    }
}

impl Error {
    pub(crate) fn new(kind: ErrorKind, offset: usize) -> Error {
        Error { kind, offset }
    }

    /// The byte offset in the pattern that the error points at. It is always
    /// on a character boundary and never greater than the pattern's length.
    #[must_use]
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// What went wrong.
    #[must_use]
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.kind.message())
    }
}

impl std::error::Error for Error {}
