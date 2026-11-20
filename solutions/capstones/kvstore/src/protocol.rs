//! The wire protocol (SPEC 8): cutting a byte stream into lines without
//! ever holding more than one legal line, and turning a line into a
//! [`Command`].
//!
//! Nothing here does I/O, which is why all of it can be unit-tested with
//! byte strings.

use crate::record::{Key, Value};

/// The longest request line, without its line feed (SPEC 8.1).
pub const MAX_LINE: usize = 8192;

/// One request. It borrows key and value from the line it was parsed from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command<'a> {
    Ping,
    Get(Key<'a>),
    Set(Key<'a>, Value<'a>),
    Del(Key<'a>),
    Incr(Key<'a>),
    Keys(Option<Key<'a>>),
    Stats,
    Compact,
    Quit,
    Shutdown,
}

/// Why a line is not a request (SPEC 8.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseError {
    UnknownCommand,
    Syntax,
    BadKey,
    ValueTooLarge,
}

impl ParseError {
    /// The response line for this error, without its line feed.
    pub fn reply(self) -> &'static str {
        match self {
            ParseError::UnknownCommand => "ERR unknown command",
            ParseError::Syntax => "ERR syntax",
            ParseError::BadKey => "ERR bad key",
            ParseError::ValueTooLarge => "ERR value too large",
        }
    }
}

/// Cuts `bytes` at the first space. `None` on the right means there was no
/// space; `Some(b"")` means the space was the last byte. The two are
/// different requests: `SET k` is a syntax error, `SET k ` sets an empty
/// value.
fn split_at_space(bytes: &[u8]) -> (&[u8], Option<&[u8]>) {
    match bytes.iter().position(|&b| b == b' ') {
        Some(space) => (&bytes[..space], Some(&bytes[space + 1..])),
        None => (bytes, None),
    }
}

/// Parses one line (without its line feed, at most [`MAX_LINE`] bytes).
/// The checks run in the order SPEC 8.2 gives: command name, shape, key,
/// value.
pub fn parse(line: &[u8]) -> Result<Command<'_>, ParseError> {
    let (name, rest) = split_at_space(line);
    let bare = |command| match rest {
        None => Ok(command),
        Some(_) => Err(ParseError::Syntax),
    };
    let key = || Key::new(rest.ok_or(ParseError::Syntax)?).ok_or(ParseError::BadKey);
    match name {
        b"PING" => bare(Command::Ping),
        b"STATS" => bare(Command::Stats),
        b"COMPACT" => bare(Command::Compact),
        b"QUIT" => bare(Command::Quit),
        b"SHUTDOWN" => bare(Command::Shutdown),
        b"GET" => key().map(Command::Get),
        b"DEL" => key().map(Command::Del),
        b"INCR" => key().map(Command::Incr),
        b"KEYS" => match rest {
            None => Ok(Command::Keys(None)),
            Some(prefix) => Key::new(prefix)
                .map(|p| Command::Keys(Some(p)))
                .ok_or(ParseError::BadKey),
        },
        b"SET" => {
            let (key, value) = split_at_space(rest.ok_or(ParseError::Syntax)?);
            let value = value.ok_or(ParseError::Syntax)?;
            let key = Key::new(key).ok_or(ParseError::BadKey)?;
            // A line cannot contain a line feed, so the only way for the
            // value to be invalid is its length.
            let value = Value::new(value).ok_or(ParseError::ValueTooLarge)?;
            Ok(Command::Set(key, value))
        }
        _ => Err(ParseError::UnknownCommand),
    }
}

/// What [`LineBuffer::feed`] found in the bytes it was given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fed {
    /// No line feed yet; all input was consumed.
    Partial,
    /// A line is complete: read it with [`LineBuffer::line`].
    Line,
    /// A line longer than [`MAX_LINE`] has just ended. Its bytes are gone.
    TooLong,
}

/// Collects one request line from the pieces a socket delivers.
///
/// Memory use is bounded by [`MAX_LINE`]: once a line is known to be too
/// long, its remaining bytes are counted off and dropped, not stored.
#[derive(Debug, Default)]
pub struct LineBuffer {
    line: Vec<u8>,
    state: State,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum State {
    /// Bytes of an unfinished line are in `line`.
    #[default]
    Collecting,
    /// The current line is too long; bytes are dropped until its line feed.
    Skipping,
    /// `line` holds a finished line that the next `feed` will clear.
    Complete,
}

impl LineBuffer {
    pub fn new() -> LineBuffer {
        LineBuffer::default()
    }

    /// Takes bytes from the front of `input` up to and including the first
    /// line feed, or all of them if there is none. Returns how many bytes
    /// were taken; the caller feeds the rest again.
    pub fn feed(&mut self, input: &[u8]) -> (usize, Fed) {
        if self.state == State::Complete {
            self.line.clear();
            self.state = State::Collecting;
        }
        let (piece, ends) = match input.iter().position(|&b| b == b'\n') {
            Some(line_feed) => (&input[..line_feed], true),
            None => (input, false),
        };
        if self.state == State::Collecting {
            if self.line.len() + piece.len() <= MAX_LINE {
                self.line.extend_from_slice(piece);
            } else {
                self.line.clear();
                self.state = State::Skipping;
            }
        }
        match (ends, self.state) {
            (false, _) => (piece.len(), Fed::Partial),
            (true, State::Skipping) => {
                self.state = State::Collecting;
                (piece.len() + 1, Fed::TooLong)
            }
            (true, _) => {
                self.state = State::Complete;
                (piece.len() + 1, Fed::Line)
            }
        }
    }

    /// The line that the last `feed` completed, without its line feed.
    pub fn line(&self) -> &[u8] {
        debug_assert_eq!(self.state, State::Complete, "no complete line to read");
        &self.line
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(text: &str) -> Key<'_> {
        Key::new(text.as_bytes()).unwrap()
    }

    fn value(text: &str) -> Value<'_> {
        Value::new(text.as_bytes()).unwrap()
    }

    #[test]
    fn commands_without_arguments() {
        assert_eq!(parse(b"PING"), Ok(Command::Ping));
        assert_eq!(parse(b"STATS"), Ok(Command::Stats));
        assert_eq!(parse(b"COMPACT"), Ok(Command::Compact));
        assert_eq!(parse(b"QUIT"), Ok(Command::Quit));
        assert_eq!(parse(b"SHUTDOWN"), Ok(Command::Shutdown));
        assert_eq!(parse(b"KEYS"), Ok(Command::Keys(None)));
        for line in [
            &b"PING "[..],
            b"PING x",
            b"STATS 1",
            b"SHUTDOWN now",
            b"QUIT ",
        ] {
            assert_eq!(parse(line), Err(ParseError::Syntax), "{line:?}");
        }
    }

    #[test]
    fn commands_with_a_key() {
        assert_eq!(parse(b"GET a"), Ok(Command::Get(key("a"))));
        assert_eq!(parse(b"DEL a"), Ok(Command::Del(key("a"))));
        assert_eq!(parse(b"INCR a"), Ok(Command::Incr(key("a"))));
        assert_eq!(parse(b"KEYS a"), Ok(Command::Keys(Some(key("a")))));
        assert_eq!(parse(b"GET"), Err(ParseError::Syntax));
        assert_eq!(parse(b"GET "), Err(ParseError::BadKey));
        assert_eq!(parse(b"GET a b"), Err(ParseError::BadKey));
        assert_eq!(parse(b"KEYS "), Err(ParseError::BadKey));
        assert_eq!(parse(b"GET a\r"), Err(ParseError::BadKey));
    }

    #[test]
    fn set() {
        assert_eq!(parse(b"SET k v"), Ok(Command::Set(key("k"), value("v"))));
        assert_eq!(parse(b"SET k "), Ok(Command::Set(key("k"), value(""))));
        assert_eq!(
            parse(b"SET k  a b "),
            Ok(Command::Set(key("k"), value(" a b ")))
        );
        assert_eq!(parse(b"SET"), Err(ParseError::Syntax));
        assert_eq!(parse(b"SET k"), Err(ParseError::Syntax));
        assert_eq!(parse(b"SET  v"), Err(ParseError::BadKey));
        let mut line = b"SET k ".to_vec();
        line.extend_from_slice(&[b'v'; 4096]);
        assert!(parse(&line).is_ok());
        line.push(b'v');
        assert_eq!(parse(&line), Err(ParseError::ValueTooLarge));
        // Key before value.
        let mut line = b"SET a\x7f ".to_vec();
        line.extend_from_slice(&[b'v'; 5000]);
        assert_eq!(parse(&line), Err(ParseError::BadKey));
    }

    #[test]
    fn unknown_commands() {
        for line in [
            &b""[..],
            b" ",
            b"ping",
            b" PING",
            b"PING\r",
            b"GETa",
            b"FOO a b",
        ] {
            assert_eq!(parse(line), Err(ParseError::UnknownCommand), "{line:?}");
        }
    }

    /// Feeds `input` in pieces of `step` bytes and collects what comes out.
    fn lines(input: &[u8], step: usize) -> Vec<Result<Vec<u8>, ()>> {
        let mut buffer = LineBuffer::new();
        let mut out = Vec::new();
        for mut piece in input.chunks(step) {
            while !piece.is_empty() {
                let (used, fed) = buffer.feed(piece);
                piece = &piece[used..];
                match fed {
                    Fed::Partial => assert!(piece.is_empty(), "Partial must consume everything"),
                    Fed::Line => out.push(Ok(buffer.line().to_vec())),
                    Fed::TooLong => out.push(Err(())),
                }
            }
        }
        out
    }

    #[test]
    fn lines_survive_any_split_of_the_stream() {
        let input = b"PING\n\nSET a b c\nlast line has no feed";
        let expected = vec![
            Ok(b"PING".to_vec()),
            Ok(b"".to_vec()),
            Ok(b"SET a b c".to_vec()),
        ];
        for step in 1..=input.len() {
            assert_eq!(lines(input, step), expected, "pieces of {step} bytes");
        }
    }

    #[test]
    fn the_length_limit_is_exact() {
        let mut input = vec![b'a'; MAX_LINE];
        input.push(b'\n');
        input.extend_from_slice(&vec![b'b'; MAX_LINE + 1]);
        input.extend_from_slice(b"\nPING\n");
        let expected = vec![Ok(vec![b'a'; MAX_LINE]), Err(()), Ok(b"PING".to_vec())];
        for step in [1, 7, 4096, MAX_LINE, MAX_LINE + 1, input.len()] {
            assert_eq!(lines(&input, step), expected, "pieces of {step} bytes");
        }
    }

    #[test]
    fn a_huge_line_is_not_stored() {
        let mut buffer = LineBuffer::new();
        let chunk = [b'x'; 4096];
        for _ in 0..1000 {
            assert_eq!(buffer.feed(&chunk), (4096, Fed::Partial));
            assert!(
                buffer.line.capacity() <= 4 * MAX_LINE,
                "the buffer grew with the line"
            );
        }
        assert_eq!(buffer.feed(b"tail\nPING\n"), (5, Fed::TooLong));
        assert_eq!(buffer.feed(b"PING\n"), (5, Fed::Line));
        assert_eq!(buffer.line(), b"PING");
    }
}
