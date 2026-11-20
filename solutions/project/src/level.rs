//! Milestones m09 (Chapter 10) and m11 (Chapters 11–13): the `Level` enum.

use std::{fmt, str::FromStr};

/// How severe a record is. Variants are declared from least to most severe,
/// so the derived `Ord` orders them that way: `Level::Warn > Level::Info`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    /// Very detailed diagnostics.
    Trace,
    /// Diagnostics for developers.
    Debug,
    /// Normal operation.
    Info,
    /// Something unexpected that the program handled.
    Warn,
    /// A failed operation.
    Error,
}

impl Level {
    /// Every level, from least to most severe.
    pub const ALL: [Level; 5] = [
        Level::Trace,
        Level::Debug,
        Level::Info,
        Level::Warn,
        Level::Error,
    ];

    /// m09 A. Recognize a level name in any ASCII case: `TRACE`, `DEBUG`,
    /// `INFO`, `WARN`, `ERROR`, plus the alias `WARNING` for `Warn`.
    /// Anything else, including `" INFO"` and `""`, is `None`.
    ///
    /// Use a `match`. Hint: normalize case without allocating, or match on
    /// `eq_ignore_ascii_case` in guards.
    pub fn from_name(name: &str) -> Option<Level> {
        match name {
            n if n.eq_ignore_ascii_case("trace") => Some(Level::Trace),
            n if n.eq_ignore_ascii_case("debug") => Some(Level::Debug),
            n if n.eq_ignore_ascii_case("info") => Some(Level::Info),
            n if n.eq_ignore_ascii_case("warn") || n.eq_ignore_ascii_case("warning") => {
                Some(Level::Warn)
            }
            n if n.eq_ignore_ascii_case("error") => Some(Level::Error),
            _ => None,
        }
    }

    /// m09 B. The canonical uppercase name: `"TRACE"`, `"DEBUG"`, `"INFO"`,
    /// `"WARN"`, `"ERROR"`.
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Trace => "TRACE",
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }
}

/// The error returned when text is not a level name. It keeps the rejected
/// text so the message can show it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownLevel(pub String);

impl fmt::Display for UnknownLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown level {:?}", self.0)
    }
}

impl std::error::Error for UnknownLevel {}

/// m11 A. `"warn".parse::<Level>()` works like [`Level::from_name`], and the
/// error keeps the rejected text: `UnknownLevel("loud".to_string())`.
impl FromStr for Level {
    type Err = UnknownLevel;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Level::from_name(s).ok_or_else(|| UnknownLevel(s.to_string()))
    }
}

/// m11 B. Display the canonical name, honoring width and alignment flags:
/// `format!("[{:<5}]", Level::Info)` is `"[INFO ]"`. (`Formatter::pad`
/// does this for you; `write!` ignores the flags.)
impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(self.as_str())
    }
}

/// m11 C. Numeric severities: 0 is `Trace`, 1 `Debug`, 2 `Info`, 3 `Warn`,
/// 4 `Error`. Any other number is returned unchanged as the error.
impl TryFrom<u8> for Level {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, u8> {
        Level::ALL.get(usize::from(value)).copied().ok_or(value)
    }
}

/// m11 C. The inverse of `TryFrom<u8>`: `u8::from(Level::Warn)` is 3.
impl From<Level> for u8 {
    fn from(level: Level) -> u8 {
        level as u8
    }
}
