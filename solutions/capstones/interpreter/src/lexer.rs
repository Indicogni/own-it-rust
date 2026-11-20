//! Text to tokens (SPEC section 3).
//!
//! The lexer never fails. A lexical error becomes a [`TokenKind::Invalid`]
//! token, lexing stops there, and the parser reports it when (and only if)
//! it gets that far. That gives the rule of SPEC 11.3, "the first error in
//! source order wins", without any bookkeeping.

use std::fmt;
use std::rc::Rc;

use crate::source::Span;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Int(i64),
    /// A string literal; the payload is the value after escapes.
    Str(Rc<str>),
    /// A name. The text is read from the source through the span.
    Ident,

    Let,
    Var,
    Fn,
    If,
    Else,
    While,
    For,
    In,
    Return,
    Break,
    Continue,
    And,
    Or,
    Not,
    True,
    False,
    Nil,

    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Comma,
    Semicolon,
    DotDot,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    EqEq,
    BangEq,
    Lt,
    LtEq,
    Gt,
    GtEq,

    /// Text that is not a token. Always the last token before `Eof`.
    Invalid(LexError),
    /// The end of the text; its span is empty.
    Eof,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LexError {
    UnexpectedChar(char),
    UnterminatedString,
    /// A backslash followed by this character.
    InvalidEscape(char),
    IntTooLarge,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexError::UnexpectedChar(c) => write!(f, "unexpected character '{c}'"),
            LexError::UnterminatedString => f.write_str("unterminated string"),
            LexError::InvalidEscape(c) => write!(f, "invalid escape sequence '\\{c}'"),
            LexError::IntTooLarge => f.write_str("integer literal is too large"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

/// The tokens of `text` from byte offset `start` on. Spans are offsets into
/// the whole of `text`. The last token is always `Eof`; if there is a
/// lexical error, the one before it is `Invalid` and nothing after the
/// error is lexed.
pub fn tokenize(text: &str, start: usize) -> Vec<Token> {
    let mut lexer = Lexer { text, pos: start };
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token();
        let last = matches!(token.kind, TokenKind::Invalid(_) | TokenKind::Eof);
        tokens.push(token);
        if last {
            break;
        }
    }
    if tokens.last().is_some_and(|t| t.kind != TokenKind::Eof) {
        tokens.push(Token {
            kind: TokenKind::Eof,
            span: Span::new(text.len(), text.len()),
        });
    }
    tokens
}

/// True when `text` leaves a `(`, `[` or `{` open and has no lexical error:
/// the REPL then asks for another line (SPEC 12.2).
pub fn needs_more_input(text: &str) -> bool {
    let mut open = 0_i64;
    for token in tokenize(text, 0) {
        match token.kind {
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => open += 1,
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => open -= 1,
            TokenKind::Invalid(_) => return false,
            _ => {}
        }
    }
    open > 0
}

struct Lexer<'a> {
    text: &'a str,
    /// Byte offset of the next unread character.
    pos: usize,
}

impl Lexer<'_> {
    fn rest(&self) -> &str {
        &self.text[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    /// Consumes `expected` if it is next.
    fn eat(&mut self, expected: char) -> bool {
        let found = self.peek() == Some(expected);
        if found {
            self.pos += expected.len_utf8();
        }
        found
    }

    /// At a line feed, at a carriage return directly before one, or at the
    /// end of the text.
    fn at_line_end(&self) -> bool {
        let rest = self.rest();
        rest.is_empty() || rest.starts_with('\n') || rest.starts_with("\r\n")
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            let rest = self.rest();
            if rest.starts_with([' ', '\t', '\r', '\n']) {
                self.pos += 1;
            } else if rest.starts_with("//") {
                self.pos += rest.find('\n').unwrap_or(rest.len());
            } else {
                return;
            }
        }
    }

    fn next_token(&mut self) -> Token {
        self.skip_whitespace_and_comments();
        let start = self.pos;
        let kind = match self.bump() {
            None => TokenKind::Eof,
            Some('0'..='9') => self.number(start),
            Some('a'..='z' | 'A'..='Z' | '_') => self.word(start),
            Some('"') => return self.string(start),
            Some('(') => TokenKind::LParen,
            Some(')') => TokenKind::RParen,
            Some('[') => TokenKind::LBracket,
            Some(']') => TokenKind::RBracket,
            Some('{') => TokenKind::LBrace,
            Some('}') => TokenKind::RBrace,
            Some(',') => TokenKind::Comma,
            Some(';') => TokenKind::Semicolon,
            Some('+') => TokenKind::Plus,
            Some('-') => TokenKind::Minus,
            Some('*') => TokenKind::Star,
            Some('/') => TokenKind::Slash,
            Some('%') => TokenKind::Percent,
            Some('.') if self.eat('.') => TokenKind::DotDot,
            Some('=') if self.eat('=') => TokenKind::EqEq,
            Some('=') => TokenKind::Eq,
            Some('!') if self.eat('=') => TokenKind::BangEq,
            Some('<') if self.eat('=') => TokenKind::LtEq,
            Some('<') => TokenKind::Lt,
            Some('>') if self.eat('=') => TokenKind::GtEq,
            Some('>') => TokenKind::Gt,
            Some(other) => TokenKind::Invalid(LexError::UnexpectedChar(other)),
        };
        Token {
            kind,
            span: Span::new(start, self.pos),
        }
    }

    fn number(&mut self, start: usize) -> TokenKind {
        while matches!(self.peek(), Some('0'..='9')) {
            self.pos += 1;
        }
        // `parse` fails only on overflow here: the text is ASCII digits.
        match self.text[start..self.pos].parse() {
            Ok(value) => TokenKind::Int(value),
            Err(_) => TokenKind::Invalid(LexError::IntTooLarge),
        }
    }

    fn word(&mut self, start: usize) -> TokenKind {
        while matches!(self.peek(), Some('a'..='z' | 'A'..='Z' | '0'..='9' | '_')) {
            self.pos += 1;
        }
        match &self.text[start..self.pos] {
            "let" => TokenKind::Let,
            "var" => TokenKind::Var,
            "fn" => TokenKind::Fn,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "return" => TokenKind::Return,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "not" => TokenKind::Not,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "nil" => TokenKind::Nil,
            _ => TokenKind::Ident,
        }
    }

    /// The opening quote at `start` has been consumed.
    fn string(&mut self, start: usize) -> Token {
        let quote = Span::new(start, start + 1);
        let unterminated = Token {
            kind: TokenKind::Invalid(LexError::UnterminatedString),
            span: quote,
        };
        let mut value = String::new();
        loop {
            if self.at_line_end() {
                return unterminated;
            }
            let at = self.pos;
            match self.bump() {
                Some('"') => {
                    return Token {
                        kind: TokenKind::Str(value.into()),
                        span: Span::new(start, self.pos),
                    };
                }
                Some('\\') => {
                    if self.at_line_end() {
                        return unterminated;
                    }
                    match self.bump() {
                        Some('n') => value.push('\n'),
                        Some('t') => value.push('\t'),
                        Some('r') => value.push('\r'),
                        Some('0') => value.push('\0'),
                        Some('\\') => value.push('\\'),
                        Some('"') => value.push('"'),
                        Some(other) => {
                            return Token {
                                kind: TokenKind::Invalid(LexError::InvalidEscape(other)),
                                span: Span::new(at, self.pos),
                            };
                        }
                        None => return unterminated,
                    }
                }
                Some(c) => value.push(c),
                None => return unterminated,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TokenKind::*;
    use super::*;

    fn kinds(text: &str) -> Vec<TokenKind> {
        tokenize(text, 0).into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn punctuation_takes_the_longest_match() {
        assert_eq!(
            kinds("<= < == = != >= > .."),
            [LtEq, Lt, EqEq, Eq, BangEq, GtEq, Gt, DotDot, Eof]
        );
        assert_eq!(kinds("1..5"), [Int(1), DotDot, Int(5), Eof]);
    }

    #[test]
    fn keywords_need_the_whole_word() {
        assert_eq!(
            kinds("for form in inn _ nil nil_"),
            [For, Ident, In, Ident, Ident, Nil, Ident, Eof]
        );
    }

    #[test]
    fn comments_and_all_four_whitespace_characters_are_skipped() {
        assert_eq!(kinds("1 // one\r\n\t2//two"), [Int(1), Int(2), Eof]);
    }

    #[test]
    fn spans_are_byte_offsets() {
        let tokens = tokenize("ab \"é\" c", 0);
        let spans: Vec<(u32, u32)> = tokens.iter().map(|t| (t.span.start, t.span.end)).collect();
        assert_eq!(spans, [(0, 2), (3, 7), (8, 9), (9, 9)]);
    }

    #[test]
    fn lexing_can_start_in_the_middle_and_keeps_absolute_offsets() {
        let tokens = tokenize("skipped @ text\nab", 15);
        assert_eq!(tokens[0].kind, Ident);
        assert_eq!((tokens[0].span.start, tokens[0].span.end), (15, 17));
    }

    #[test]
    fn escapes_are_decoded() {
        assert_eq!(
            kinds(r#""a\n\t\r\0\\\"z""#),
            [Str("a\n\t\r\0\\\"z".into()), Eof]
        );
    }

    #[test]
    fn the_largest_integer_is_accepted_and_the_next_one_is_not() {
        assert_eq!(kinds("9223372036854775807"), [Int(i64::MAX), Eof]);
        assert_eq!(
            kinds("9223372036854775808 1"),
            [Invalid(LexError::IntTooLarge), Eof]
        );
    }

    #[test]
    fn lexing_stops_at_the_first_error() {
        assert_eq!(
            kinds("1 @ \"open"),
            [Int(1), Invalid(LexError::UnexpectedChar('@')), Eof]
        );
    }

    #[test]
    fn string_errors_point_at_the_right_characters() {
        let unterminated = &tokenize("x \"abc\nmore", 0)[1];
        assert_eq!(unterminated.kind, Invalid(LexError::UnterminatedString));
        assert_eq!((unterminated.span.start, unterminated.span.end), (2, 3));

        let escape = &tokenize(r#""ab\qcd""#, 0)[0];
        assert_eq!(escape.kind, Invalid(LexError::InvalidEscape('q')));
        assert_eq!((escape.span.start, escape.span.end), (3, 5));

        // A backslash at the end of a line has no character to escape.
        assert_eq!(
            kinds("\"ab\\\r\n"),
            [Invalid(LexError::UnterminatedString), Eof]
        );
        assert_eq!(
            kinds("\"ab\\"),
            [Invalid(LexError::UnterminatedString), Eof]
        );
    }

    #[test]
    fn open_delimiters_ask_for_more_input() {
        assert!(needs_more_input("fn f() {"));
        assert!(needs_more_input("[1,\n(2"));
        assert!(!needs_more_input("fn f() { }"));
        assert!(!needs_more_input("}"));
        // Delimiters inside strings and comments do not count.
        assert!(!needs_more_input("\"(\" // {"));
        // A lexical error ends the entry so that it is reported at once.
        assert!(!needs_more_input("{ \"open"));
    }
}
