//! JSON validation and compaction for `POST /echo` (SPEC section 9).
//!
//! The endpoint does not need a tree of values: it must only decide whether
//! the body is one JSON value and copy it without the whitespace between
//! tokens. So this is a scanner that walks the bytes once and copies as it
//! goes. Nesting is tracked on an explicit stack, not by recursion: the
//! depth of the input is chosen by a stranger, and a thread's call stack is
//! the wrong place to store it.

use std::fmt;

/// The deepest nesting of arrays and objects the server accepts.
pub const MAX_DEPTH: usize = 64;

/// Why a body was rejected. The client only sees `400`; the reason and the
/// byte offset go to the log and into test failure messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonError {
    pub offset: usize,
    pub reason: &'static str,
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid JSON at byte {}: {}", self.offset, self.reason)
    }
}

impl std::error::Error for JsonError {}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Container {
    Array,
    Object,
}

/// What the grammar allows at the current position.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Expect {
    /// Any value.
    Value,
    /// Just after `[`: a value, or `]` for the empty array.
    ValueOrArrayEnd,
    /// Just after `{`: a key, or `}` for the empty object.
    KeyOrObjectEnd,
    /// After `,` in an object: a key.
    Key,
    /// After a key.
    Colon,
    /// After a value inside a container: `,` or the closing bracket.
    CommaOrEnd,
    /// After the top-level value: nothing but whitespace.
    End,
}

/// Checks that `input` is exactly one JSON value (RFC 8259) in UTF-8 and
/// returns it with the insignificant whitespace removed. Every token is
/// copied byte for byte: numbers and string escapes are not rewritten.
pub fn compact(input: &[u8]) -> Result<Vec<u8>, JsonError> {
    // One pass over the whole body up front means the string scanner below
    // can pass bytes above 0x7F through without looking at them again.
    if let Err(error) = std::str::from_utf8(input) {
        return Err(JsonError {
            offset: error.valid_up_to(),
            reason: "the body is not valid UTF-8",
        });
    }

    let mut scanner = Scanner { input, at: 0 };
    let mut out = Vec::with_capacity(input.len());
    let mut open: Vec<Container> = Vec::new();
    let mut expect = Expect::Value;

    loop {
        scanner.skip_whitespace();
        let Some(byte) = scanner.peek() else {
            return if expect == Expect::End {
                Ok(out)
            } else {
                Err(scanner.error("the document ends too early"))
            };
        };
        // The state to enter once a complete value has been read.
        let after_value = |open: &[Container]| {
            if open.is_empty() {
                Expect::End
            } else {
                Expect::CommaOrEnd
            }
        };

        expect = match (expect, byte) {
            (Expect::End, _) => return Err(scanner.error("text after the JSON value")),

            (Expect::ValueOrArrayEnd, b']') | (Expect::KeyOrObjectEnd, b'}') => {
                scanner.at += 1;
                out.push(byte);
                open.pop();
                after_value(&open)
            }

            (Expect::Value | Expect::ValueOrArrayEnd, b'[' | b'{') => {
                if open.len() == MAX_DEPTH {
                    return Err(scanner.error("nested too deeply"));
                }
                scanner.at += 1;
                out.push(byte);
                if byte == b'[' {
                    open.push(Container::Array);
                    Expect::ValueOrArrayEnd
                } else {
                    open.push(Container::Object);
                    Expect::KeyOrObjectEnd
                }
            }
            (Expect::Value | Expect::ValueOrArrayEnd, _) => {
                let start = scanner.at;
                match byte {
                    b'"' => scanner.string()?,
                    b'-' | b'0'..=b'9' => scanner.number()?,
                    b't' => scanner.literal(b"true")?,
                    b'f' => scanner.literal(b"false")?,
                    b'n' => scanner.literal(b"null")?,
                    _ => return Err(scanner.error("expected a value")),
                }
                out.extend_from_slice(&input[start..scanner.at]);
                after_value(&open)
            }

            (Expect::KeyOrObjectEnd | Expect::Key, b'"') => {
                let start = scanner.at;
                scanner.string()?;
                out.extend_from_slice(&input[start..scanner.at]);
                Expect::Colon
            }
            (Expect::KeyOrObjectEnd | Expect::Key, _) => {
                return Err(scanner.error("expected a string as object key"));
            }

            (Expect::Colon, b':') => {
                scanner.at += 1;
                out.push(b':');
                Expect::Value
            }
            (Expect::Colon, _) => return Err(scanner.error("expected `:` after the key")),

            (Expect::CommaOrEnd, _) => {
                // `CommaOrEnd` is only entered with a container open.
                let container = *open.last().expect("inside a container");
                match (container, byte) {
                    (Container::Array, b',') => {
                        scanner.at += 1;
                        out.push(b',');
                        Expect::Value
                    }
                    (Container::Object, b',') => {
                        scanner.at += 1;
                        out.push(b',');
                        Expect::Key
                    }
                    (Container::Array, b']') | (Container::Object, b'}') => {
                        scanner.at += 1;
                        out.push(byte);
                        open.pop();
                        after_value(&open)
                    }
                    (Container::Array, _) => return Err(scanner.error("expected `,` or `]`")),
                    (Container::Object, _) => return Err(scanner.error("expected `,` or `}`")),
                }
            }
        };
    }
}

/// A position in the input and the token-level rules.
struct Scanner<'a> {
    input: &'a [u8],
    at: usize,
}

impl Scanner<'_> {
    fn peek(&self) -> Option<u8> {
        self.input.get(self.at).copied()
    }

    fn error(&self, reason: &'static str) -> JsonError {
        JsonError {
            offset: self.at,
            reason,
        }
    }

    /// JSON whitespace is exactly these four bytes; `u8::is_ascii_whitespace`
    /// would also accept form feed.
    fn skip_whitespace(&mut self) {
        while let Some(b' ' | b'\t' | b'\n' | b'\r') = self.peek() {
            self.at += 1;
        }
    }

    fn literal(&mut self, word: &'static [u8]) -> Result<(), JsonError> {
        if self.input[self.at..].starts_with(word) {
            self.at += word.len();
            Ok(())
        } else {
            Err(self.error("expected `true`, `false` or `null`"))
        }
    }

    /// Reads at least one digit.
    fn digits(&mut self) -> Result<(), JsonError> {
        let start = self.at;
        while let Some(b'0'..=b'9') = self.peek() {
            self.at += 1;
        }
        if self.at == start {
            Err(self.error("expected a digit"))
        } else {
            Ok(())
        }
    }

    /// `-? (0 | [1-9][0-9]*) (. [0-9]+)? ([eE] [+-]? [0-9]+)?`
    ///
    /// A number such as `01` stops after the `0`; the `1` is then an error
    /// in the caller, which expects a comma or the end.
    fn number(&mut self) -> Result<(), JsonError> {
        if self.peek() == Some(b'-') {
            self.at += 1;
        }
        if self.peek() == Some(b'0') {
            self.at += 1;
        } else {
            self.digits()?;
        }
        if self.peek() == Some(b'.') {
            self.at += 1;
            self.digits()?;
        }
        if let Some(b'e' | b'E') = self.peek() {
            self.at += 1;
            if let Some(b'+' | b'-') = self.peek() {
                self.at += 1;
            }
            self.digits()?;
        }
        Ok(())
    }

    /// Reads a string, from the opening quote to the closing one.
    fn string(&mut self) -> Result<(), JsonError> {
        self.at += 1; // the opening quote
        loop {
            match self.peek() {
                None => return Err(self.error("unterminated string")),
                Some(b'"') => {
                    self.at += 1;
                    return Ok(());
                }
                Some(b'\\') => {
                    self.at += 1;
                    match self.peek() {
                        Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => {
                            self.at += 1;
                        }
                        Some(b'u') => {
                            self.at += 1;
                            for _ in 0..4 {
                                match self.peek() {
                                    Some(digit) if digit.is_ascii_hexdigit() => self.at += 1,
                                    _ => {
                                        return Err(
                                            self.error("expected four hex digits after \\u")
                                        );
                                    }
                                }
                            }
                        }
                        _ => return Err(self.error("unknown escape in string")),
                    }
                }
                Some(0x00..=0x1F) => {
                    return Err(self.error("control character in string; escape it"));
                }
                Some(_) => self.at += 1,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(input: &str) -> String {
        match compact(input.as_bytes()) {
            Ok(out) => String::from_utf8(out).expect("output of valid input is UTF-8"),
            Err(error) => panic!("{input:?} should be valid JSON, got: {error}"),
        }
    }

    #[test]
    fn whitespace_between_tokens_is_removed() {
        assert_eq!(
            ok(" { \"a\" : [ 1 , 2.50 , \"x y\" ] ,\r\n\t\"b\" : null } "),
            r#"{"a":[1,2.50,"x y"],"b":null}"#
        );
        assert_eq!(ok("[ ]"), "[]");
        assert_eq!(ok("{ }"), "{}");
        assert_eq!(ok("\n true \n"), "true");
    }

    #[test]
    fn tokens_are_copied_byte_for_byte() {
        for token in [
            "0",
            "-0",
            "12",
            "0.5",
            "-1.5e-3",
            "1E+2",
            "1e10",
            "123456789012345678901234567890.123456789e+400",
            r#""""#,
            r#""  two  spaces  ""#,
            r#""a\"b\\c\/d\b\f\n\r\t\u00e9\uD83D\uDE00""#,
            "\"h\u{e9}llo \u{2603}\"",
            "null",
            "false",
            r#"{"a":1,"a":2}"#,
            "[[[[]]]]",
            r#"{"a":{"b":{"c":[{"d":[]}]}}}"#,
        ] {
            assert_eq!(ok(token), token);
        }
    }

    #[test]
    fn invalid_documents_are_rejected() {
        for bad in [
            "",
            "  ",
            "{",
            "[",
            "]",
            "[1,]",
            "[,1]",
            "[1 2]",
            "[1,,2]",
            r#"{"a"}"#,
            r#"{"a":}"#,
            r#"{"a":1,}"#,
            r#"{"a":1 "b":2}"#,
            "{a:1}",
            "{'a':1}",
            "{1:2}",
            "{,}",
            "[01]",
            "[1.]",
            "[.5]",
            "[+1]",
            "[1e]",
            "[1e+]",
            "[-]",
            "-",
            "[0x10]",
            "tru",
            "nul",
            "nulll",
            "True",
            "NaN",
            "[true false]",
            "\"unterminated",
            r#""bad \x escape""#,
            r#""\u12""#,
            r#""\u12G4""#,
            "\"line\nbreak\"",
            "\"tab\there\"",
            "[1]]",
            "{}{}",
            "1 2",
            "[1],",
            "\u{feff}{}",
            "[1]\u{c}",
        ] {
            assert!(
                compact(bad.as_bytes()).is_err(),
                "{bad:?} should be rejected"
            );
        }
    }

    #[test]
    fn invalid_utf8_is_rejected_with_its_offset() {
        let error = compact(b"[\"ok\", \"\xff\"]").unwrap_err();
        assert_eq!(error.offset, 8);
        assert!(compact(b"\"\xc0\xaf\"").is_err(), "overlong encoding");
    }

    #[test]
    fn depth_limit_is_exact_and_needs_no_stack() {
        let nested = |depth: usize| format!("{}{}", "[".repeat(depth), "]".repeat(depth));
        assert!(compact(nested(MAX_DEPTH).as_bytes()).is_ok());
        assert!(compact(nested(MAX_DEPTH + 1).as_bytes()).is_err());
        // Siblings do not add depth.
        let wide = format!("[{}[]]", "[],".repeat(1000));
        assert!(compact(wide.as_bytes()).is_ok());
        // A million open brackets must be an error, not a crash.
        assert!(compact("[".repeat(1_000_000).as_bytes()).is_err());
        let mixed = format!("{}1{}", "{\"k\":[".repeat(32), "]}".repeat(32));
        assert!(
            compact(mixed.as_bytes()).is_ok(),
            "32 objects with 32 arrays is depth 64"
        );
        let mixed = format!("[{}1{}]", "{\"k\":[".repeat(32), "]}".repeat(32));
        assert!(
            compact(mixed.as_bytes()).is_err(),
            "one more level is depth 65"
        );
    }
}
