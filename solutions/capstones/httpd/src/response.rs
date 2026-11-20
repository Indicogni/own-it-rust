//! Responses: the status codes the server uses and how a response is
//! written to the wire (SPEC section 6).

use std::fs::File;
use std::io::{self, Read, Write};

use crate::date;

/// Every status this server can send. An enum, not a number, so that a
/// status without a reason phrase cannot exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ok,
    BadRequest,
    Forbidden,
    NotFound,
    MethodNotAllowed,
    RequestTimeout,
    LengthRequired,
    ContentTooLarge,
    UriTooLong,
    UnsupportedMediaType,
    HeaderFieldsTooLarge,
    InternalServerError,
    NotImplemented,
    ServiceUnavailable,
    VersionNotSupported,
}

impl Status {
    pub fn code(self) -> u16 {
        match self {
            Self::Ok => 200,
            Self::BadRequest => 400,
            Self::Forbidden => 403,
            Self::NotFound => 404,
            Self::MethodNotAllowed => 405,
            Self::RequestTimeout => 408,
            Self::LengthRequired => 411,
            Self::ContentTooLarge => 413,
            Self::UriTooLong => 414,
            Self::UnsupportedMediaType => 415,
            Self::HeaderFieldsTooLarge => 431,
            Self::InternalServerError => 500,
            Self::NotImplemented => 501,
            Self::ServiceUnavailable => 503,
            Self::VersionNotSupported => 505,
        }
    }

    pub fn reason(self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::BadRequest => "Bad Request",
            Self::Forbidden => "Forbidden",
            Self::NotFound => "Not Found",
            Self::MethodNotAllowed => "Method Not Allowed",
            Self::RequestTimeout => "Request Timeout",
            Self::LengthRequired => "Length Required",
            Self::ContentTooLarge => "Content Too Large",
            Self::UriTooLong => "URI Too Long",
            Self::UnsupportedMediaType => "Unsupported Media Type",
            Self::HeaderFieldsTooLarge => "Request Header Fields Too Large",
            Self::InternalServerError => "Internal Server Error",
            Self::NotImplemented => "Not Implemented",
            Self::ServiceUnavailable => "Service Unavailable",
            Self::VersionNotSupported => "HTTP Version Not Supported",
        }
    }
}

pub const TEXT_PLAIN: &str = "text/plain; charset=utf-8";

/// A body up to this size is copied behind the head and sent with it in one
/// `write`. Above it, the copy costs more than the second system call. For
/// a file it is also the most that is read into memory at once.
const COALESCE_LIMIT: usize = 16 * 1024;

/// The content of a response.
pub enum Body {
    Bytes(Vec<u8>),
    /// An open file and the number of bytes to send from it. The length is
    /// read once, when the file is opened, because it is promised in
    /// `Content-Length` before the first byte of the file is sent.
    File {
        file: File,
        len: u64,
    },
}

impl Body {
    fn len(&self) -> u64 {
        match self {
            Self::Bytes(bytes) => bytes.len() as u64,
            Self::File { len, .. } => *len,
        }
    }
}

/// A response without the parts that depend on the connection: whether the
/// body is sent (not for `HEAD`) and what `Connection` says are decided by
/// the caller of [`Response::write_to`].
pub struct Response {
    pub status: Status,
    pub content_type: &'static str,
    /// One extra header, for the three statuses that need one (`Allow`,
    /// `Retry-After`). A list would be more general than this server is.
    pub extra: Option<(&'static str, &'static str)>,
    pub body: Body,
}

impl Response {
    pub fn new(status: Status, content_type: &'static str, body: Body) -> Self {
        Self {
            status,
            content_type,
            extra: None,
            body,
        }
    }

    /// The standard response for a status without content of its own: a
    /// plain-text body that repeats the status line (SPEC 6.4).
    pub fn status_only(status: Status) -> Self {
        let body = format!("{} {}\n", status.code(), status.reason());
        Self::new(status, TEXT_PLAIN, Body::Bytes(body.into_bytes()))
    }

    pub fn with_header(mut self, name: &'static str, value: &'static str) -> Self {
        self.extra = Some((name, value));
        self
    }

    /// Writes the response. `Content-Length` always describes the full
    /// body, also when `head_only` suppresses it.
    ///
    /// The head and a small body, from memory or from a file, go out in one
    /// `write`, so that a small response is one TCP segment. A large file
    /// follows the head through `io::copy`, which never holds more than a
    /// buffer of it in memory.
    pub fn write_to(
        self,
        out: &mut impl Write,
        keep_alive: bool,
        head_only: bool,
    ) -> io::Result<()> {
        let mut head = Vec::with_capacity(256);
        write!(
            head,
            "HTTP/1.1 {} {}\r\n",
            self.status.code(),
            self.status.reason()
        )?;
        write!(head, "Date: {}\r\n", date::now())?;
        write!(head, "Content-Type: {}\r\n", self.content_type)?;
        write!(head, "Content-Length: {}\r\n", self.body.len())?;
        if let Some((name, value)) = self.extra {
            write!(head, "{name}: {value}\r\n")?;
        }
        let connection = if keep_alive { "keep-alive" } else { "close" };
        write!(head, "Connection: {connection}\r\n\r\n")?;

        match self.body {
            _ if head_only => out.write_all(&head),
            Body::Bytes(bytes) if bytes.len() <= COALESCE_LIMIT => {
                head.extend_from_slice(&bytes);
                out.write_all(&head)
            }
            Body::Bytes(bytes) => {
                out.write_all(&head)?;
                out.write_all(&bytes)
            }
            Body::File { file, len } => {
                // Never more than `len` bytes, also if the file has grown.
                let mut file = file.take(len);
                let sent = if len <= COALESCE_LIMIT as u64 {
                    // A small file travels with the head, like a small
                    // body from memory. Nothing has been sent yet, so a
                    // file that turns out shorter costs the client nothing.
                    let read = file.read_to_end(&mut head)?;
                    if read as u64 == len {
                        out.write_all(&head)?;
                    }
                    read as u64
                } else {
                    out.write_all(&head)?;
                    io::copy(&mut file, out)?
                };
                if sent == len {
                    Ok(())
                } else {
                    // The file shrank after its length was read. The head
                    // promises bytes that do not exist, so the only honest
                    // signal left is to drop the connection.
                    Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "the file became shorter while it was being sent",
                    ))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn written(response: Response, keep_alive: bool, head_only: bool) -> String {
        let mut out = Vec::new();
        response
            .write_to(&mut out, keep_alive, head_only)
            .expect("writing to a Vec");
        String::from_utf8(out).expect("these responses are text")
    }

    #[test]
    fn a_status_only_response_on_the_wire() {
        let text = written(Response::status_only(Status::NotFound), true, false);
        let (head, body) = text
            .split_once("\r\n\r\n")
            .expect("an empty line ends the head");
        assert_eq!(body, "404 Not Found\n");
        let mut lines = head.split("\r\n");
        assert_eq!(lines.next(), Some("HTTP/1.1 404 Not Found"));
        let headers: Vec<&str> = lines.collect();
        assert!(headers.contains(&"Content-Length: 14"));
        assert!(headers.contains(&"Content-Type: text/plain; charset=utf-8"));
        assert!(headers.contains(&"Connection: keep-alive"));
        assert_eq!(
            headers.iter().filter(|h| h.starts_with("Date: ")).count(),
            1
        );
    }

    #[test]
    fn head_only_keeps_the_length_and_drops_the_body() {
        let text = written(Response::status_only(Status::NotFound), false, true);
        assert!(text.ends_with("Connection: close\r\n\r\n"));
        assert!(text.contains("Content-Length: 14\r\n"));
    }

    #[test]
    fn the_extra_header_is_written() {
        let response = Response::status_only(Status::MethodNotAllowed).with_header("Allow", "POST");
        assert!(written(response, true, false).contains("\r\nAllow: POST\r\n"));
    }

    /// A writer that remembers every `write` call separately.
    #[derive(Default)]
    struct Writes(Vec<Vec<u8>>);

    impl Write for Writes {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.push(bytes.to_vec());
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// Writes a response whose body is a file with `content`, announced
    /// with the length `len`, and returns the result and the `write` calls.
    fn file_response(name: &str, content: &[u8], len: u64) -> (io::Result<()>, Vec<Vec<u8>>) {
        let path = std::env::temp_dir().join(format!("httpd-unit-{}-{name}", std::process::id()));
        std::fs::write(&path, content).expect("the temporary directory can be written");
        let file = File::open(&path).expect("the file was just written");
        let response = Response::new(Status::Ok, TEXT_PLAIN, Body::File { file, len });
        let mut out = Writes::default();
        let result = response.write_to(&mut out, true, false);
        let _ = std::fs::remove_file(&path);
        (result, out.0)
    }

    #[test]
    fn a_small_file_leaves_in_one_write_with_its_head() {
        let (result, writes) = file_response("small", b"hello\n", 6);
        result.expect("writing to memory");
        assert_eq!(
            writes.len(),
            1,
            "head and body of a small file are one write"
        );
        let text = String::from_utf8(writes.concat()).expect("text");
        assert!(text.contains("\r\nContent-Length: 6\r\n"));
        assert!(text.ends_with("\r\n\r\nhello\n"));
    }

    #[test]
    fn a_large_file_is_sent_completely() {
        let content: Vec<u8> = (0..COALESCE_LIMIT * 3 + 7)
            .map(|n| (n % 251) as u8)
            .collect();
        let (result, writes) = file_response("large", &content, content.len() as u64);
        result.expect("writing to memory");
        let all = writes.concat();
        assert!(
            all.ends_with(&content),
            "the bytes of the file follow the head"
        );
        assert_eq!(
            all.len() - content.len(),
            writes[0].len(),
            "the first write is the head"
        );
    }

    #[test]
    fn never_more_than_the_announced_length() {
        // The file grew after its length was read.
        let (result, writes) = file_response("grown", b"hello, and more", 5);
        result.expect("writing to memory");
        assert!(writes.concat().ends_with(b"\r\n\r\nhello"));
    }

    #[test]
    fn a_file_that_became_shorter_is_an_error() {
        let (result, writes) = file_response("shrunk-small", b"hel", 5);
        assert_eq!(
            result.map_err(|error| error.kind()),
            Err(io::ErrorKind::UnexpectedEof)
        );
        assert!(
            writes.is_empty(),
            "a small file is read before anything is sent"
        );

        let announced = COALESCE_LIMIT as u64 * 2;
        let (result, _) = file_response("shrunk-large", b"hel", announced);
        assert_eq!(
            result.map_err(|error| error.kind()),
            Err(io::ErrorKind::UnexpectedEof),
            "the head is out already; the caller must drop the connection"
        );
    }

    #[test]
    fn codes_and_reasons_are_unique_and_plausible() {
        let all = [
            Status::Ok,
            Status::BadRequest,
            Status::Forbidden,
            Status::NotFound,
            Status::MethodNotAllowed,
            Status::RequestTimeout,
            Status::LengthRequired,
            Status::ContentTooLarge,
            Status::UriTooLong,
            Status::UnsupportedMediaType,
            Status::HeaderFieldsTooLarge,
            Status::InternalServerError,
            Status::NotImplemented,
            Status::ServiceUnavailable,
            Status::VersionNotSupported,
        ];
        let mut codes: Vec<u16> = all.iter().map(|s| s.code()).collect();
        codes.dedup();
        assert_eq!(codes.len(), all.len());
        assert!(codes.is_sorted(), "keep the list in numeric order");
        assert!(
            all.iter()
                .all(|s| !s.reason().is_empty() && s.reason().is_ascii())
        );
    }
}
