//! The request head: finding where it ends and parsing it (SPEC section 5).
//!
//! Nothing in this module touches a socket. [`find_head_end`] and [`parse`]
//! are pure functions over bytes, so the grammar is tested without a network.

use std::fmt;

/// The head (request line, header lines and the empty line) may not be
/// longer than this many bytes.
pub const MAX_HEAD: usize = 8192;
/// The most header lines a request may have.
pub const MAX_HEADERS: usize = 64;
/// The longest request target.
pub const MAX_TARGET: usize = 2048;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Head,
    Post,
    /// A well-formed method name that this server does not implement.
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Version {
    Http10,
    Http11,
}

/// A parsed request head.
#[derive(Debug, PartialEq, Eq)]
pub struct Head {
    pub method: Method,
    /// The request target as sent: it starts with `/` and is printable ASCII.
    pub target: String,
    pub version: Version,
    pub headers: Headers,
}

/// Header lines in the order they arrived. Values are bytes: the grammar
/// allows bytes above 0x7F in them, and nothing says they are UTF-8.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Headers(Vec<(String, Vec<u8>)>);

impl Headers {
    /// All values of the header `name`; names compare without regard to
    /// ASCII case.
    pub fn all<'a>(&'a self, name: &str) -> impl Iterator<Item = &'a [u8]> {
        self.0
            .iter()
            .filter(move |(have, _)| have.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_slice())
    }

    /// The first value of the header `name`.
    pub fn first(&self, name: &str) -> Option<&[u8]> {
        self.all(name).next()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.first(name).is_some()
    }

    /// True when a comma-separated header such as `Connection` lists
    /// `token` (compared without regard to ASCII case) in any of its lines.
    pub fn has_token(&self, name: &str, token: &str) -> bool {
        self.all(name).any(|value| {
            value
                .split(|&b| b == b',')
                .any(|item| item.trim_ascii().eq_ignore_ascii_case(token.as_bytes()))
        })
    }
}

/// Why a head was rejected. Each variant is one status code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadError {
    /// 400, with the reason for the log.
    Malformed(&'static str),
    /// 505: the version has the right shape but is not 1.0 or 1.1.
    Version,
    /// 414
    TargetTooLong,
    /// 431: more than [`MAX_HEADERS`] header lines.
    TooManyHeaders,
}

impl fmt::Display for HeadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed(reason) => write!(f, "malformed request: {reason}"),
            Self::Version => write!(f, "unsupported HTTP version"),
            Self::TargetTooLong => write!(f, "request target longer than {MAX_TARGET} bytes"),
            Self::TooManyHeaders => write!(f, "more than {MAX_HEADERS} header lines"),
        }
    }
}

/// What a scan for the end of the head found.
#[derive(Debug, PartialEq, Eq)]
pub enum HeadEnd {
    /// The head is the first `n` bytes, including the empty line.
    At(usize),
    /// No empty line yet. Call again with the returned position as `from`
    /// once more bytes have arrived.
    Incomplete(usize),
    /// A line feed without a carriage return before it (SPEC 5.1): the
    /// request can be refused now, without waiting for a head that a client
    /// with the wrong line endings will never finish.
    BareLineFeed,
    /// The head would be longer than [`MAX_HEAD`].
    TooLong,
}

/// Looks for the empty line that ends the head. `from` is where an earlier
/// call stopped (0 at first), so that each byte is examined once however
/// the request is split into reads.
pub fn find_head_end(buffer: &[u8], from: usize) -> HeadEnd {
    // Bytes after MAX_HEAD cannot be part of an acceptable head.
    let window = &buffer[..buffer.len().min(MAX_HEAD)];
    for at in from..window.len() {
        if window[at] != b'\n' {
            continue;
        }
        if at == 0 || window[at - 1] != b'\r' {
            return HeadEnd::BareLineFeed;
        }
        if at >= 3 && &window[at - 3..at - 1] == b"\r\n" {
            return HeadEnd::At(at + 1);
        }
    }
    if buffer.len() >= MAX_HEAD {
        HeadEnd::TooLong
    } else {
        HeadEnd::Incomplete(window.len())
    }
}

/// Parses a complete head: the bytes up to and including the empty line, as
/// delimited by [`find_head_end`].
pub fn parse(head: &[u8]) -> Result<Head, HeadError> {
    let lines = head.strip_suffix(b"\r\n\r\n").ok_or(HeadError::Malformed(
        "the head does not end with an empty line",
    ))?;
    let mut lines = split_lines(lines);

    let request_line = lines
        .next()
        .ok_or(HeadError::Malformed("empty request"))??;
    let (method, target, version) = parse_request_line(request_line)?;

    let mut headers = Vec::new();
    for line in lines {
        if headers.len() == MAX_HEADERS {
            return Err(HeadError::TooManyHeaders);
        }
        headers.push(parse_header_line(line?)?);
    }
    let headers = Headers(headers);

    // HTTP/1.1 made `Host` mandatory; two of them means two components could
    // disagree about which site the request is for.
    let hosts = headers.all("host").count();
    if hosts > 1 || (hosts == 0 && version == Version::Http11) {
        return Err(HeadError::Malformed(
            "HTTP/1.1 needs exactly one Host header",
        ));
    }

    Ok(Head {
        method,
        target,
        version,
        headers,
    })
}

/// Splits at CR LF. A CR or LF that is not part of that pair is an error
/// (SPEC 5.1): accepting other line endings is how two programs come to
/// disagree about where a request ends.
fn split_lines(mut rest: &[u8]) -> impl Iterator<Item = Result<&[u8], HeadError>> {
    let mut done = false;
    std::iter::from_fn(move || {
        if done {
            return None;
        }
        let end = rest.iter().position(|&b| b == b'\r' || b == b'\n');
        let line = match end {
            None => {
                done = true;
                rest
            }
            Some(end) if rest[end..].starts_with(b"\r\n") => {
                let line = &rest[..end];
                rest = &rest[end + 2..];
                line
            }
            Some(_) => {
                done = true;
                return Some(Err(HeadError::Malformed("CR or LF outside a CR LF pair")));
            }
        };
        Some(Ok(line))
    })
}

fn parse_request_line(line: &[u8]) -> Result<(Method, String, Version), HeadError> {
    // Exactly two single spaces. `split` yields empty pieces for doubled,
    // leading or trailing spaces, and the checks below refuse empty pieces.
    let mut parts = line.split(|&b| b == b' ');
    let (Some(method), Some(target), Some(version), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(HeadError::Malformed(
            "the request line is not `METHOD SP target SP version`",
        ));
    };

    if !is_token(method) {
        return Err(HeadError::Malformed("the method is not a token"));
    }
    let method = match method {
        b"GET" => Method::Get,
        b"HEAD" => Method::Head,
        b"POST" => Method::Post,
        _ => Method::Unknown,
    };

    let version = match version {
        b"HTTP/1.1" => Version::Http11,
        b"HTTP/1.0" => Version::Http10,
        [b'H', b'T', b'T', b'P', b'/', major, b'.', minor]
            if major.is_ascii_digit() && minor.is_ascii_digit() =>
        {
            return Err(HeadError::Version);
        }
        _ => {
            return Err(HeadError::Malformed(
                "the version is not `HTTP/digit.digit`",
            ));
        }
    };

    if target.first() != Some(&b'/') {
        return Err(HeadError::Malformed("the target does not start with `/`"));
    }
    if !target.iter().all(u8::is_ascii_graphic) {
        return Err(HeadError::Malformed(
            "the target has a byte outside printable ASCII",
        ));
    }
    if target.len() > MAX_TARGET {
        return Err(HeadError::TargetTooLong);
    }
    // Printable ASCII was checked just above, so this cannot fail.
    let target = String::from_utf8(target.to_vec())
        .map_err(|_| HeadError::Malformed("the target is not ASCII"))?;

    Ok((method, target, version))
}

fn parse_header_line(line: &[u8]) -> Result<(String, Vec<u8>), HeadError> {
    let colon = line
        .iter()
        .position(|&b| b == b':')
        .ok_or(HeadError::Malformed("a header line has no colon"))?;
    let (name, value) = (&line[..colon], &line[colon + 1..]);
    // This one check also refuses a space before the colon and a line that
    // starts with a space (the obsolete way to continue the previous line).
    if !is_token(name) {
        return Err(HeadError::Malformed("a header name is not a token"));
    }
    let value = trim_spaces_and_tabs(value);
    let allowed = |&b: &u8| b == b'\t' || (0x20..=0x7E).contains(&b) || b >= 0x80;
    if !value.iter().all(allowed) {
        return Err(HeadError::Malformed(
            "a header value has a control character",
        ));
    }
    // Tokens are ASCII, so the name is valid UTF-8.
    let name = String::from_utf8(name.to_vec())
        .map_err(|_| HeadError::Malformed("a header name is not ASCII"))?;
    Ok((name, value.to_vec()))
}

/// One or more of the characters HTTP allows in names (RFC 9110, `token`).
fn is_token(bytes: &[u8]) -> bool {
    !bytes.is_empty()
        && bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(b))
}

/// Removes the optional whitespace around a header value. HTTP means space
/// and horizontal tab only, so `trim_ascii` (which also removes form feed
/// and line breaks) is not the right tool.
fn trim_spaces_and_tabs(mut bytes: &[u8]) -> &[u8] {
    while let [b' ' | b'\t', rest @ ..] = bytes {
        bytes = rest;
    }
    while let [rest @ .., b' ' | b'\t'] = bytes {
        bytes = rest;
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn head(text: &str) -> Result<Head, HeadError> {
        parse(text.as_bytes())
    }

    fn malformed(text: &str) -> bool {
        matches!(head(text), Err(HeadError::Malformed(_)))
    }

    #[test]
    fn a_plain_request_parses() {
        let head =
            head("GET /a/b?c=d HTTP/1.1\r\nHost: example\r\nX-Two:  1 \t\r\nx-two:2\r\n\r\n")
                .expect("valid");
        assert_eq!(head.method, Method::Get);
        assert_eq!(head.target, "/a/b?c=d");
        assert_eq!(head.version, Version::Http11);
        assert_eq!(head.headers.first("HOST"), Some(&b"example"[..]));
        let twos: Vec<&[u8]> = head.headers.all("X-TWO").collect();
        assert_eq!(twos, [b"1", b"2"]);
        assert!(!head.headers.contains("x-three"));
    }

    #[test]
    fn methods_and_versions() {
        assert_eq!(
            head("HEAD / HTTP/1.0\r\n\r\n").unwrap().method,
            Method::Head
        );
        assert_eq!(
            head("POST / HTTP/1.0\r\n\r\n").unwrap().version,
            Version::Http10
        );
        assert_eq!(
            head("BREW / HTTP/1.0\r\n\r\n").unwrap().method,
            Method::Unknown
        );
        assert_eq!(
            head("get / HTTP/1.0\r\n\r\n").unwrap().method,
            Method::Unknown
        );
        for version in ["HTTP/2.0", "HTTP/0.9", "HTTP/1.2", "HTTP/3.0"] {
            let text = format!("GET / {version}\r\nHost: h\r\n\r\n");
            assert_eq!(head(&text), Err(HeadError::Version), "{version}");
        }
        for version in [
            "http/1.1",
            "HTTP/1",
            "HTTP/1.10",
            "HTTP/11",
            "FTP/1.1",
            "HTTP/a.b",
        ] {
            let text = format!("GET / {version}\r\nHost: h\r\n\r\n");
            assert!(malformed(&text), "{version}");
        }
    }

    #[test]
    fn request_line_shape() {
        for line in [
            "GET /",
            "GET  / HTTP/1.1",
            " GET / HTTP/1.1",
            "GET / HTTP/1.1 ",
            "GET / HTTP/1.1 extra",
            "G@T / HTTP/1.1",
            "GET x HTTP/1.1",
            "GET * HTTP/1.1",
            "GET http://example/ HTTP/1.1",
            "GET /a\tb HTTP/1.1",
            "GET /caf\u{e9} HTTP/1.1",
            "",
        ] {
            assert!(malformed(&format!("{line}\r\nHost: h\r\n\r\n")), "{line:?}");
        }
    }

    #[test]
    fn host_rules() {
        assert!(malformed("GET / HTTP/1.1\r\n\r\n"));
        assert!(malformed("GET / HTTP/1.1\r\nHost: a\r\nhost: a\r\n\r\n"));
        assert!(malformed("GET / HTTP/1.0\r\nHost: a\r\nHost: b\r\n\r\n"));
        assert!(head("GET / HTTP/1.0\r\n\r\n").is_ok());
    }

    #[test]
    fn header_line_shape() {
        for line in [
            "Host : h",
            "NoColon",
            ": empty-name",
            " folded: 1",
            "\tfolded",
            "X Y: 1",
            "X(Y): 1",
            "X: a\u{0}b",
            "X: a\u{1}b",
            "X: a\u{7f}b",
            "X: a\rb",
            "X: a\nb",
        ] {
            let text = format!("GET / HTTP/1.1\r\nHost: h\r\n{line}\r\n\r\n");
            assert!(malformed(&text), "{line:?}");
        }
        let mut bytes =
            b"GET / HTTP/1.1\r\nHost: h\r\nX-Note: caf\xe9\tok\r\nX-Empty:\r\n\r\n".to_vec();
        let head = parse(&bytes).expect("obs-text, tab and an empty value are allowed");
        assert_eq!(head.headers.first("x-note"), Some(&b"caf\xe9\tok"[..]));
        assert_eq!(head.headers.first("x-empty"), Some(&b""[..]));
        bytes.truncate(bytes.len() - 2);
        assert!(parse(&bytes).is_err(), "a head must end with an empty line");
    }

    #[test]
    fn limits() {
        let with_headers = |n: usize| {
            let mut text = String::from("GET / HTTP/1.1\r\nHost: h\r\n");
            for i in 1..n {
                text.push_str(&format!("X-{i}: v\r\n"));
            }
            text + "\r\n"
        };
        assert!(head(&with_headers(MAX_HEADERS)).is_ok());
        assert_eq!(
            head(&with_headers(MAX_HEADERS + 1)),
            Err(HeadError::TooManyHeaders)
        );

        let with_target =
            |n: usize| format!("GET /{} HTTP/1.1\r\nHost: h\r\n\r\n", "a".repeat(n - 1));
        assert!(head(&with_target(MAX_TARGET)).is_ok());
        assert_eq!(
            head(&with_target(MAX_TARGET + 1)),
            Err(HeadError::TargetTooLong)
        );
    }

    #[test]
    fn connection_tokens() {
        let head =
            head("GET / HTTP/1.1\r\nHost: h\r\nConnection: Keep-Alive ,\tCLOSE\r\n\r\n").unwrap();
        assert!(head.headers.has_token("connection", "close"));
        assert!(head.headers.has_token("connection", "keep-alive"));
        assert!(!head.headers.has_token("connection", "clos"));
    }

    #[test]
    fn the_end_of_the_head_is_found_however_the_bytes_arrive() {
        let request = b"GET / HTTP/1.1\r\nHost: h\r\n\r\nBODY";
        let end = request.len() - 4;
        assert_eq!(find_head_end(request, 0), HeadEnd::At(end));
        // Fed one byte at a time, resuming where the last scan stopped.
        let mut from = 0;
        for have in 1..end {
            match find_head_end(&request[..have], from) {
                HeadEnd::Incomplete(next) => from = next,
                other => panic!("after {have} bytes: {other:?}"),
            }
        }
        assert_eq!(find_head_end(&request[..end], from), HeadEnd::At(end));
        assert_eq!(find_head_end(b"\r\n\r\n", 0), HeadEnd::At(4));
        assert_eq!(find_head_end(b"", 0), HeadEnd::Incomplete(0));
    }

    #[test]
    fn a_bare_line_feed_is_reported_at_once() {
        assert_eq!(find_head_end(b"GET / HTTP/1.1\n", 0), HeadEnd::BareLineFeed);
        assert_eq!(find_head_end(b"\n", 0), HeadEnd::BareLineFeed);
        assert_eq!(
            find_head_end(b"GET / HTTP/1.1\r\nHost: h\n\r\n", 0),
            HeadEnd::BareLineFeed
        );
    }

    #[test]
    fn the_head_limit_is_exact() {
        let padded = |total: usize| {
            let fixed = "GET / HTTP/1.1\r\nHost: h\r\nX-Pad: \r\n\r\n".len();
            format!(
                "GET / HTTP/1.1\r\nHost: h\r\nX-Pad: {}\r\n\r\n",
                "a".repeat(total - fixed)
            )
        };
        assert_eq!(
            find_head_end(padded(MAX_HEAD).as_bytes(), 0),
            HeadEnd::At(MAX_HEAD)
        );
        assert_eq!(
            find_head_end(padded(MAX_HEAD + 1).as_bytes(), 0),
            HeadEnd::TooLong
        );
        // One byte short of the limit and still no end: more may come.
        let open = vec![b'a'; MAX_HEAD - 1];
        assert_eq!(find_head_end(&open, 0), HeadEnd::Incomplete(MAX_HEAD - 1));
        let open = vec![b'a'; MAX_HEAD];
        assert_eq!(find_head_end(&open, 0), HeadEnd::TooLong);
    }
}
