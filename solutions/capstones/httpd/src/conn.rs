//! One connection, from the first byte to the close (SPEC sections 4 to 7).
//!
//! A worker thread calls [`serve`] and stays in it until the connection
//! ends. The code is blocking and reads top to bottom in the order of the
//! SPEC's processing steps: wait, read the head, decide the framing, read
//! the body, route, answer. Every blocking call has a deadline.

use std::io::{self, Read};
use std::net::{Shutdown, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::files::{self, PathError};
use crate::json;
use crate::request::{self, Head, HeadEnd, HeadError, Method, Version};
use crate::response::{Body, Response, Status};

/// What every connection needs from the server.
pub struct Context {
    pub config: Config,
    /// The canonical form of `config.root`, resolved once at start-up.
    pub root: PathBuf,
    /// Set once, when shutdown begins; never cleared.
    pub shutdown: AtomicBool,
}

impl Context {
    pub fn shutting_down(&self) -> bool {
        // The flag guards no other data: a worker that reads the old value
        // a moment longer only starts one more wait. `Relaxed` is enough.
        self.shutdown.load(Ordering::Relaxed)
    }
}

/// How often a worker that waits for a request looks at the shutdown flag.
/// The standard library cannot wait for "bytes or a signal from another
/// thread", so the wait is cut into slices. DESIGN.md discusses the cost.
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Limits for [`linger`].
const LINGER_TIME: Duration = Duration::from_secs(1);
const LINGER_BYTES: usize = 64 * 1024;

/// How a request/response exchange left the connection.
enum Next {
    /// Wait for the next request.
    KeepAlive,
    /// A response with `Connection: close` was sent: close politely.
    Linger,
    /// Nothing more to say, or nobody to say it to: just close.
    Close,
}

/// Why reading stopped before the wanted bytes were there.
enum Stop {
    /// The deadline passed.
    TimedOut,
    /// The client closed its side, or the connection broke.
    Gone,
}

/// Serves requests on `stream` until the connection ends.
pub fn serve(stream: TcpStream, context: &Context) {
    // Responses are written with one or two `write` calls. Without this,
    // the second small write could wait for the client's delayed
    // acknowledgement of the first (Nagle's algorithm): tens of
    // milliseconds per request.
    let _ = stream.set_nodelay(true);
    // A client that stops reading must not hold a worker forever (SPEC 7.3).
    let _ = stream.set_write_timeout(Some(context.config.request_timeout));

    let mut connection = Connection {
        stream,
        buffer: Vec::new(),
        context,
    };
    loop {
        match connection.exchange() {
            Next::KeepAlive => {}
            Next::Linger => {
                linger(&connection.stream);
                return;
            }
            Next::Close => return,
        }
    }
}

struct Connection<'a> {
    stream: TcpStream,
    /// Bytes received and not yet used. After a request has been answered,
    /// its bytes are removed; what remains is the start of the next request
    /// (a client may send several without waiting).
    buffer: Vec<u8>,
    context: &'a Context,
}

impl Connection<'_> {
    /// Handles one request: SPEC section 4, steps 1 to 6.
    fn exchange(&mut self) -> Next {
        let config = &self.context.config;

        // Step 1: wait for the first byte.
        if self.buffer.is_empty() && !self.wait_for_request() {
            return Next::Close;
        }
        // One deadline for the whole request. A timeout per read would let
        // a client keep a worker by sending one byte at a time.
        let deadline = Instant::now() + config.request_timeout;

        // Step 2: the head.
        let head_len = match self.read_head(deadline) {
            Ok(Ok(len)) => len,
            Ok(Err(status)) => return self.refuse(status, false),
            Err(Stop::TimedOut) => return self.refuse(Status::RequestTimeout, false),
            Err(Stop::Gone) => return Next::Close,
        };
        let head = match request::parse(&self.buffer[..head_len]) {
            Ok(head) => head,
            Err(error) => {
                let status = match error {
                    HeadError::Malformed(_) => Status::BadRequest,
                    HeadError::Version => Status::VersionNotSupported,
                    HeadError::TargetTooLong => Status::UriTooLong,
                    HeadError::TooManyHeaders => Status::HeaderFieldsTooLarge,
                };
                return self.refuse(status, false);
            }
        };
        let head_only = head.method == Method::Head;

        // Step 3: where does the request end?
        let body_len = match body_length(&head, config.max_body) {
            Ok(len) => len,
            Err(status) => return self.refuse(status, head_only),
        };

        // Step 4: the body. The buffer grows as bytes arrive; reserving
        // `body_len` up front would let a client make the server allocate
        // the maximum by sending one header line.
        let request_len = head_len + body_len;
        while self.buffer.len() < request_len {
            match self.read_more(deadline) {
                Ok(()) => {}
                Err(Stop::TimedOut) => return self.refuse(Status::RequestTimeout, head_only),
                Err(Stop::Gone) => return Next::Close,
            }
        }

        // Step 5: the request is complete; decide the answer.
        let response = route(
            &head,
            &self.buffer[head_len..request_len],
            &self.context.root,
        );
        self.buffer.drain(..request_len);
        // A large body made the buffer large. Give the memory back, so
        // that an idle connection costs a head's worth and not a body's.
        self.buffer.shrink_to(request::MAX_HEAD);

        // Step 6: answer. The connection stays open only if the client
        // wants that and the server is not shutting down.
        let keep_alive = wants_keep_alive(&head) && !self.context.shutting_down();
        match response.write_to(&mut self.stream, keep_alive, head_only) {
            Ok(()) if keep_alive => Next::KeepAlive,
            Ok(()) => Next::Linger,
            Err(_) => Next::Close,
        }
    }

    /// Sends the standard response for `status` with `Connection: close`.
    /// Used whenever the server cannot be sure where the request ends.
    fn refuse(&mut self, status: Status, head_only: bool) -> Next {
        match Response::status_only(status).write_to(&mut self.stream, false, head_only) {
            Ok(()) => Next::Linger,
            Err(_) => Next::Close,
        }
    }

    /// Waits until the first byte of a request is in the buffer. Returns
    /// `false` when the connection should be closed without a response:
    /// the idle timeout passed, the client left, or shutdown began.
    fn wait_for_request(&mut self) -> bool {
        let idle_deadline = Instant::now() + self.context.config.idle_timeout;
        loop {
            if self.context.shutting_down() {
                return false;
            }
            let slice_end = idle_deadline.min(Instant::now() + POLL_INTERVAL);
            match self.read_more(slice_end) {
                Ok(()) => return true,
                Err(Stop::TimedOut) if Instant::now() < idle_deadline => {}
                Err(_) => return false,
            }
        }
    }

    /// Reads until the buffer starts with a complete head and returns its
    /// length. The inner `Err` is a status to refuse the request with.
    fn read_head(&mut self, deadline: Instant) -> Result<Result<usize, Status>, Stop> {
        let mut scanned = 0;
        loop {
            match request::find_head_end(&self.buffer, scanned) {
                HeadEnd::At(len) => return Ok(Ok(len)),
                HeadEnd::BareLineFeed => return Ok(Err(Status::BadRequest)),
                HeadEnd::TooLong => return Ok(Err(Status::HeaderFieldsTooLarge)),
                HeadEnd::Incomplete(next) => scanned = next,
            }
            self.read_more(deadline)?;
        }
    }

    /// Appends at least one byte to the buffer, waiting until `deadline`
    /// at most.
    fn read_more(&mut self, deadline: Instant) -> Result<(), Stop> {
        let mut chunk = [0u8; 8192];
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            // `set_read_timeout` rejects zero, which would mean "no timeout".
            if remaining.is_zero() {
                return Err(Stop::TimedOut);
            }
            // Setting an option fails on a socket that is already dead (on
            // macOS, for one, after the client has reset the connection).
            // Calling that a timeout would make the idle wait spin.
            if self.stream.set_read_timeout(Some(remaining)).is_err() {
                return Err(Stop::Gone);
            }
            return match self.stream.read(&mut chunk) {
                Ok(0) => Err(Stop::Gone),
                Ok(n) => {
                    self.buffer.extend_from_slice(&chunk[..n]);
                    Ok(())
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) if is_timeout(&error) => Err(Stop::TimedOut),
                Err(_) => Err(Stop::Gone),
            };
        }
    }
}

/// A read or write that ran into its timeout reports `WouldBlock` on Unix
/// and `TimedOut` on Windows.
fn is_timeout(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
    )
}

/// SPEC section 4, step 3: the length of the body, or the status that
/// refuses the request.
fn body_length(head: &Head, max_body: u64) -> Result<usize, Status> {
    // Chunked bodies are a non-goal. A request that uses any transfer
    // coding cannot be delimited, so it cannot be skipped either.
    if head.headers.contains("transfer-encoding") {
        return Err(Status::NotImplemented);
    }
    let mut lengths = head.headers.all("content-length");
    let value = match (lengths.next(), lengths.next()) {
        (None, _) if head.method == Method::Post => return Err(Status::LengthRequired),
        (None, _) => return Ok(0),
        // Two lengths, even equal ones, are how request smuggling starts.
        (Some(_), Some(_)) => return Err(Status::BadRequest),
        (Some(value), None) => value,
    };
    if value.is_empty() || !value.iter().all(u8::is_ascii_digit) {
        return Err(Status::BadRequest);
    }
    // All digits but too large for `u64` is still a length, just an absurd
    // one: 413, like any other length above the limit.
    let length = value.iter().try_fold(0u64, |length, digit| {
        length.checked_mul(10)?.checked_add(u64::from(digit - b'0'))
    });
    match length {
        Some(length) if length <= max_body => {
            // `max_body` is at most 16 MiB, which fits `usize` everywhere.
            usize::try_from(length).map_err(|_| Status::ContentTooLarge)
        }
        _ => Err(Status::ContentTooLarge),
    }
}

/// SPEC section 4, step 5: the method is looked at first, then the path.
fn route(head: &Head, body: &[u8], root: &Path) -> Response {
    let path = head.target.split('?').next().unwrap_or(&head.target);
    match (head.method, path == "/echo") {
        (Method::Unknown, _) => Response::status_only(Status::NotImplemented),
        (Method::Post, true) => echo(head, body),
        (Method::Get | Method::Head, true) => {
            Response::status_only(Status::MethodNotAllowed).with_header("Allow", "POST")
        }
        (Method::Post, false) => {
            Response::status_only(Status::MethodNotAllowed).with_header("Allow", "GET, HEAD")
        }
        (Method::Get | Method::Head, false) => match files::resolve(root, path) {
            Ok(file_path) => files::respond(file_path),
            Err(PathError::Malformed) => Response::status_only(Status::BadRequest),
            Err(PathError::Forbidden) => Response::status_only(Status::Forbidden),
        },
    }
}

/// `POST /echo` (SPEC section 9).
fn echo(head: &Head, body: &[u8]) -> Response {
    // The media type is the part before any `;parameter`.
    let is_json = head.headers.first("content-type").is_some_and(|value| {
        let media_type = value.split(|&b| b == b';').next().unwrap_or(value);
        media_type
            .trim_ascii()
            .eq_ignore_ascii_case(b"application/json")
    });
    if !is_json {
        return Response::status_only(Status::UnsupportedMediaType);
    }
    match json::compact(body) {
        Ok(compact) => Response::new(Status::Ok, "application/json", Body::Bytes(compact)),
        Err(_) => Response::status_only(Status::BadRequest),
    }
}

/// SPEC section 7.1.
fn wants_keep_alive(head: &Head) -> bool {
    if head.headers.has_token("connection", "close") {
        return false;
    }
    match head.version {
        Version::Http11 => true,
        Version::Http10 => head.headers.has_token("connection", "keep-alive"),
    }
}

/// Closes the sending side and then reads and discards what the client is
/// still sending, within limits (SPEC 7.4).
///
/// Closing a socket that has unread bytes makes many systems send a reset
/// instead of a normal close, and a reset can destroy a response that the
/// client has not read yet. A client that was told "413" must be able to
/// read that answer even though the rest of its upload is still arriving.
fn linger(stream: &TcpStream) {
    if stream.shutdown(Shutdown::Write).is_err() {
        return;
    }
    let deadline = Instant::now() + LINGER_TIME;
    let mut discarded = 0;
    let mut sink = [0u8; 4096];
    // `Read` is implemented for `&TcpStream`, so a shared reference is
    // enough to read from the socket.
    let mut reader = stream;
    while discarded < LINGER_BYTES {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() || stream.set_read_timeout(Some(remaining)).is_err() {
            return;
        }
        match reader.read(&mut sink) {
            // The client has closed its side too: the clean ending.
            Ok(0) => return,
            Ok(n) => discarded += n,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => return,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn head(text: &str) -> Head {
        request::parse(text.as_bytes()).expect("the test wrote a valid head")
    }

    #[test]
    fn body_length_rules() {
        let length = |headers: &str, method: &str| {
            body_length(
                &head(&format!("{method} / HTTP/1.1\r\nHost: h\r\n{headers}\r\n")),
                100,
            )
        };
        assert_eq!(length("", "GET"), Ok(0));
        assert_eq!(length("", "BREW"), Ok(0));
        assert_eq!(length("", "POST"), Err(Status::LengthRequired));
        assert_eq!(length("Content-Length: 0\r\n", "POST"), Ok(0));
        assert_eq!(length("content-length: 100\r\n", "POST"), Ok(100));
        assert_eq!(length("Content-Length: 007\r\n", "GET"), Ok(7));
        assert_eq!(
            length("Content-Length: 101\r\n", "GET"),
            Err(Status::ContentTooLarge)
        );
        assert_eq!(
            length("Content-Length: 18446744073709551616\r\n", "POST"),
            Err(Status::ContentTooLarge)
        );
        for bad in ["", "+5", "-1", "5 5", "5,5", "0x10", "1e3", "five"] {
            assert_eq!(
                length(&format!("Content-Length: {bad}\r\n"), "POST"),
                Err(Status::BadRequest),
                "{bad:?}"
            );
        }
        assert_eq!(
            length("Content-Length: 5\r\nContent-Length: 5\r\n", "POST"),
            Err(Status::BadRequest)
        );
        assert_eq!(
            length(
                "Content-Length: 5\r\nTransfer-Encoding: chunked\r\n",
                "POST"
            ),
            Err(Status::NotImplemented)
        );
    }

    #[test]
    fn keep_alive_rules() {
        let keep = |line: &str, headers: &str| {
            wants_keep_alive(&head(&format!("{line}\r\nHost: h\r\n{headers}\r\n")))
        };
        assert!(keep("GET / HTTP/1.1", ""));
        assert!(!keep("GET / HTTP/1.1", "Connection: close\r\n"));
        assert!(!keep(
            "GET / HTTP/1.1",
            "Connection: keep-alive\r\nConnection: te, Close\r\n"
        ));
        assert!(!keep("GET / HTTP/1.0", ""));
        assert!(keep("GET / HTTP/1.0", "Connection: Keep-Alive\r\n"));
        assert!(!keep("GET / HTTP/1.0", "Connection: keep-alive, close\r\n"));
    }

    #[test]
    fn routing_looks_at_the_method_first() {
        let status = |line: &str, headers: &str, body: &[u8]| {
            let head = head(&format!("{line}\r\nHost: h\r\n{headers}\r\n"));
            // The root does not exist, so every file lookup is 404.
            route(&head, body, Path::new("no-such-root")).status
        };
        let json = "Content-Type: application/json\r\n";
        assert_eq!(status("POST /echo HTTP/1.1", json, b"[1]"), Status::Ok);
        assert_eq!(status("POST /echo?x=1 HTTP/1.1", json, b"[1]"), Status::Ok);
        assert_eq!(
            status("POST /echo HTTP/1.1", json, b"[1"),
            Status::BadRequest
        );
        assert_eq!(
            status("POST /echo HTTP/1.1", "", b"[1]"),
            Status::UnsupportedMediaType
        );
        assert_eq!(
            status(
                "POST /echo HTTP/1.1",
                "Content-Type: Application/JSON ; charset=utf-8\r\n",
                b"1"
            ),
            Status::Ok
        );
        assert_eq!(
            status(
                "POST /echo HTTP/1.1",
                "Content-Type: application/jsonx\r\n",
                b"1"
            ),
            Status::UnsupportedMediaType
        );
        assert_eq!(
            status("GET /echo HTTP/1.1", "", b""),
            Status::MethodNotAllowed
        );
        assert_eq!(
            status("HEAD /echo HTTP/1.1", "", b""),
            Status::MethodNotAllowed
        );
        assert_eq!(
            status("POST /../x HTTP/1.1", "", b""),
            Status::MethodNotAllowed
        );
        assert_eq!(
            status("BREW /../x HTTP/1.1", "", b""),
            Status::NotImplemented
        );
        assert_eq!(status("GET /../x HTTP/1.1", "", b""), Status::Forbidden);
        assert_eq!(status("GET /%zz HTTP/1.1", "", b""), Status::BadRequest);
        assert_eq!(status("GET /echo/ HTTP/1.1", "", b""), Status::NotFound);
        assert_eq!(status("GET /x HTTP/1.1", "", b""), Status::NotFound);
    }
}
