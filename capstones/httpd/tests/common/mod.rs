//! Helpers shared by the acceptance tests.
//!
//! The tests are black-box tests. They know nothing about your code: they
//! start the `httpd` binary as a child process, read the address it prints,
//! and speak raw HTTP to it over a `TcpStream`. This file holds the three
//! tools they need:
//!
//! - [`Site`]: a temporary directory with the files to serve;
//! - [`Server`]: the child process, which is always killed and removed when
//!   the test ends, also when the test fails;
//! - [`Conn`]: a small HTTP client that checks every response it reads
//!   against SPEC section 6.
//!
//! Nothing here can wait forever: every read, write, connect and wait has a
//! time limit, and a limit that passes fails the test with a message.
//!
//! Each file in `tests/` is its own crate, and not every one uses every
//! helper, so unused-code warnings are switched off here.
#![allow(dead_code)]

use std::fmt::Write as _;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The binary under test. Cargo builds it before the tests and puts its
/// path here.
pub const BIN: &str = env!("CARGO_BIN_EXE_httpd");

/// How long the server may take to print its `listening on` line.
pub const START_TIMEOUT: Duration = Duration::from_secs(10);

/// How long a test waits for a response that should come at once. It is
/// shorter than the server's default request timeout (5 s) on purpose: a
/// server that answers only when its own timeout fires fails the test.
pub const IO_TIMEOUT: Duration = Duration::from_secs(4);

/// Extra time allowed after a moment at which something should happen
/// (a timeout firing, the process exiting). Generous, because test
/// machines can be slow; a correct server needs a small fraction of it.
pub const SLACK: Duration = Duration::from_secs(3);

/// The content of a file that every [`Site`] puts next to the served
/// directory, outside it. No response may ever contain this text.
pub const SECRET: &str = "TOP-SECRET-OUTSIDE-ROOT";

// ---------------------------------------------------------------------------
// Files
// ---------------------------------------------------------------------------

static NEXT_SITE: AtomicUsize = AtomicUsize::new(0);

/// A temporary directory tree, removed when the value is dropped:
///
/// ```text
/// <temp>/httpd-test-<unique>/
///     secret.txt        contains SECRET; outside the served directory
///     public/           the directory given to the server as --root
///         ...           the files of the test
/// ```
pub struct Site {
    base: PathBuf,
}

impl Site {
    /// Creates the tree with the given text files. Paths are relative to
    /// `public/` and use `/`; a path that ends in `/` makes an empty
    /// directory.
    pub fn new(files: &[(&str, &str)]) -> Site {
        let unique = format!(
            "httpd-test-{}-{}-{}",
            std::process::id(),
            NEXT_SITE.fetch_add(1, Ordering::Relaxed),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.subsec_nanos()),
        );
        let base = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(base.join("public"))
            .unwrap_or_else(|e| panic!("test setup: cannot create {}: {e}", base.display()));
        let site = Site { base };
        std::fs::write(site.base.join("secret.txt"), SECRET)
            .expect("test setup: write the secret file");
        for (path, content) in files {
            site.write(path, content);
        }
        site
    }

    /// The directory the server serves.
    pub fn root(&self) -> PathBuf {
        self.base.join("public")
    }

    /// The directory that holds `public/` and `secret.txt`.
    pub fn base(&self) -> &Path {
        &self.base
    }

    /// Creates or replaces a file below the root (or an empty directory, if
    /// `path` ends in `/`). Missing parent directories are created.
    pub fn write(&self, path: &str, content: impl AsRef<[u8]>) {
        let target = path
            .split('/')
            .filter(|part| !part.is_empty())
            .fold(self.root(), |target, part| target.join(part));
        if path.ends_with('/') {
            std::fs::create_dir_all(&target)
                .unwrap_or_else(|e| panic!("test setup: cannot create directory {path}: {e}"));
            return;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).unwrap_or_else(|e| {
                panic!("test setup: cannot create the directory for {path}: {e}")
            });
        }
        std::fs::write(&target, content)
            .unwrap_or_else(|e| panic!("test setup: cannot write {path}: {e}"));
    }
}

impl Drop for Site {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.base);
    }
}

// ---------------------------------------------------------------------------
// The server process
// ---------------------------------------------------------------------------

/// A running `httpd` child process. Dropping it kills the process, waits
/// for it and removes its files.
pub struct Server {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: Receiver<String>,
    stderr: Arc<Mutex<Vec<u8>>>,
    addr: SocketAddr,
    command_line: String,
    // Declared last: the files are removed after the process is gone.
    site: Site,
}

impl Server {
    /// Starts a server with default options that serves `files`.
    #[track_caller]
    pub fn start(files: &[(&str, &str)]) -> Server {
        Server::start_site(Site::new(files), &[])
    }

    /// Starts a server that serves `files`, with extra command-line
    /// options, for example `&["--workers", "2", "--queue", "1"]`.
    #[track_caller]
    pub fn start_with(files: &[(&str, &str)], options: &[&str]) -> Server {
        Server::start_site(Site::new(files), options)
    }

    /// Starts a server on a prepared [`Site`]. The command line is
    /// `httpd --root <site>/public --addr 127.0.0.1:0 <options...>`.
    #[track_caller]
    pub fn start_site(site: Site, options: &[&str]) -> Server {
        let mut command = Command::new(BIN);
        command
            .arg("--root")
            .arg(site.root())
            .args(["--addr", "127.0.0.1:0"])
            .args(options)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let command_line = format!(
            "httpd --root <site>/public --addr 127.0.0.1:0 {}",
            options.join(" ")
        );
        let mut child = command
            .spawn()
            .unwrap_or_else(|e| panic!("cannot start {BIN}: {e}"));

        let stdin = child.stdin.take();
        let stdout = lines_of(child.stdout.take().expect("stdout is piped"));
        let stderr = collect(child.stderr.take().expect("stderr is piped"));
        let mut server = Server {
            child,
            stdin,
            stdout,
            stderr,
            addr: SocketAddr::from(([127, 0, 0, 1], 0)),
            command_line,
            site,
        };

        let line = match server.stdout.recv_timeout(START_TIMEOUT) {
            Ok(line) => line,
            Err(RecvTimeoutError::Timeout) => panic!(
                "the server printed nothing on standard output for {START_TIMEOUT:?}. It must print \
                 `listening on 127.0.0.1:PORT` when it is ready (SPEC 3.3).{}",
                server.diagnostics()
            ),
            Err(RecvTimeoutError::Disconnected) => {
                let status = server.wait_exit(SLACK);
                panic!(
                    "the server exited ({status}) without printing `listening on 127.0.0.1:PORT` \
                     (SPEC 3.3).{}",
                    server.diagnostics()
                )
            }
        };
        let addr = line
            .strip_prefix("listening on ")
            .and_then(|addr| addr.parse::<SocketAddr>().ok())
            .filter(|addr| addr.ip().is_loopback() && addr.port() != 0);
        match addr {
            Some(addr) => server.addr = addr,
            None => panic!(
                "the first line on standard output must be `listening on 127.0.0.1:PORT` with the \
                 real port (SPEC 3.3), but it was {line:?}.{}",
                server.diagnostics()
            ),
        }
        server
    }

    /// The address the server announced.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// The files this server serves; use it to change them while it runs.
    pub fn site(&self) -> &Site {
        &self.site
    }

    /// Opens a new connection to the server.
    #[track_caller]
    pub fn connect(&self) -> Conn {
        Conn::open(self.addr)
    }

    /// Sends one `GET` on a new connection and returns the response.
    #[track_caller]
    pub fn get(&self, target: &str) -> Response {
        let mut conn = self.connect();
        conn.send(get(target));
        conn.response()
    }

    /// Writes one line to the server's standard input.
    #[track_caller]
    pub fn write_stdin(&mut self, line: &str) {
        let Some(stdin) = self.stdin.as_mut() else {
            panic!("test bug: standard input was already closed by this test");
        };
        if let Err(e) = stdin
            .write_all(line.as_bytes())
            .and_then(|()| stdin.flush())
        {
            panic!(
                "cannot write {line:?} to the server's standard input: {e}. Has the process exited?{}",
                self.diagnostics()
            );
        }
    }

    /// Closes the server's standard input, so that it reads end of input.
    pub fn close_stdin(&mut self) {
        self.stdin = None;
    }

    /// The next line the server prints on standard output, or `None` if it
    /// closes standard output first. Fails the test after `within`.
    #[track_caller]
    pub fn stdout_line(&mut self, within: Duration) -> Option<String> {
        match self.stdout.recv_timeout(within) {
            Ok(line) => Some(line),
            Err(RecvTimeoutError::Disconnected) => None,
            Err(RecvTimeoutError::Timeout) => panic!(
                "expected a line on the server's standard output within {within:?}, but none came.{}",
                self.diagnostics()
            ),
        }
    }

    /// True while the process has not exited.
    pub fn is_running(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// Waits for the process to exit and returns its status. Fails the
    /// test if it is still running after `within`.
    #[track_caller]
    pub fn wait_exit(&mut self, within: Duration) -> ExitStatus {
        let deadline = Instant::now() + within;
        loop {
            match self.child.try_wait() {
                Ok(Some(status)) => return status,
                Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
                Ok(None) => panic!(
                    "the server process was still running after {within:?}.{}",
                    self.diagnostics()
                ),
                Err(e) => panic!("cannot wait for the server process: {e}"),
            }
        }
    }

    /// Text to append to a failure message: how the server was started and
    /// what it wrote to standard error.
    pub fn diagnostics(&self) -> String {
        let stderr = self
            .stderr
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut text = format!("\n  command: {}", self.command_line);
        if stderr.is_empty() {
            text.push_str("\n  standard error of the server: (empty)");
        } else {
            let all = String::from_utf8_lossy(&stderr);
            let tail: Vec<&str> = all.lines().rev().take(20).collect();
            text.push_str("\n  standard error of the server (last lines):");
            for line in tail.iter().rev() {
                let _ = write!(text, "\n    {line}");
            }
        }
        text
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        // Kill first, then wait: the child must be gone before `site`
        // removes the directory (Windows refuses to delete open files).
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Forwards the lines of a pipe to a channel from a helper thread, so that
/// the test can wait for a line with a time limit. The thread ends when the
/// child closes the pipe, which happens at the latest when it is killed.
fn lines_of(pipe: impl Read + Send + 'static) -> Receiver<String> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(pipe).lines() {
            let Ok(line) = line else { break };
            if sender.send(line).is_err() {
                break;
            }
        }
    });
    receiver
}

/// Collects everything a pipe delivers. Reading it all the time also keeps
/// a talkative child from blocking on a full pipe.
fn collect(mut pipe: impl Read + Send + 'static) -> Arc<Mutex<Vec<u8>>> {
    let collected = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&collected);
    thread::spawn(move || {
        let mut chunk = [0u8; 4096];
        loop {
            match pipe.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => sink
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .extend_from_slice(&chunk[..n]),
            }
        }
    });
    collected
}

/// What a process that was expected to exit by itself left behind.
pub struct Finished {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

/// Runs `httpd` with exactly `args` and waits for it to exit by itself.
/// Standard input stays open the whole time, so the process does not see
/// end of input. Fails the test (and kills the process) if it is still
/// running after `within`.
#[track_caller]
pub fn run_to_exit(args: &[&str], within: Duration) -> Finished {
    let mut child = Command::new(BIN)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("cannot start {BIN}: {e}"));
    let stdout = collect(child.stdout.take().expect("stdout is piped"));
    let stderr = collect(child.stderr.take().expect("stderr is piped"));
    let text = |bytes: &Arc<Mutex<Vec<u8>>>| {
        String::from_utf8_lossy(
            &bytes
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        )
        .into_owned()
    };

    let deadline = Instant::now() + within;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                panic!(
                    "`httpd {}` should have exited by itself, but it was still running after {within:?}.\n  \
                     standard output: {:?}\n  standard error: {:?}",
                    args.join(" "),
                    text(&stdout),
                    text(&stderr)
                );
            }
            Err(e) => panic!("cannot wait for the server process: {e}"),
        }
    };
    // The pipes are closed now; give the collector threads a moment to
    // read the last bytes.
    thread::sleep(Duration::from_millis(50));
    Finished {
        status,
        stdout: text(&stdout),
        stderr: text(&stderr),
    }
}

// ---------------------------------------------------------------------------
// Requests
// ---------------------------------------------------------------------------

/// A minimal `GET` request for `target`.
pub fn get(target: &str) -> String {
    format!("GET {target} HTTP/1.1\r\nHost: test\r\n\r\n")
}

/// A minimal `HEAD` request for `target`.
pub fn head(target: &str) -> String {
    format!("HEAD {target} HTTP/1.1\r\nHost: test\r\n\r\n")
}

/// A `POST` of `body` to `target` with `Content-Type: application/json`.
pub fn post_json(target: &str, body: impl AsRef<[u8]>) -> Vec<u8> {
    let body = body.as_ref();
    let mut request = format!(
        "POST {target} HTTP/1.1\r\nHost: test\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
        body.len()
    )
    .into_bytes();
    request.extend_from_slice(body);
    request
}

// ---------------------------------------------------------------------------
// The client
// ---------------------------------------------------------------------------

/// The status codes of SPEC 6.2 with their reason phrases.
pub const STATUSES: [(u16, &str); 15] = [
    (200, "OK"),
    (400, "Bad Request"),
    (403, "Forbidden"),
    (404, "Not Found"),
    (405, "Method Not Allowed"),
    (408, "Request Timeout"),
    (411, "Length Required"),
    (413, "Content Too Large"),
    (414, "URI Too Long"),
    (415, "Unsupported Media Type"),
    (431, "Request Header Fields Too Large"),
    (500, "Internal Server Error"),
    (501, "Not Implemented"),
    (503, "Service Unavailable"),
    (505, "HTTP Version Not Supported"),
];

/// The reason phrase SPEC 6.2 gives for `status`.
#[track_caller]
pub fn reason(status: u16) -> &'static str {
    STATUSES
        .iter()
        .find(|(code, _)| *code == status)
        .map(|(_, reason)| *reason)
        .unwrap_or_else(|| panic!("{status} is not a status of SPEC 6.2"))
}

/// A parsed response. Header names are stored in lower case.
#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    /// Empty for a response read with [`Conn::head_response`].
    pub body: Vec<u8>,
}

impl Response {
    /// The value of the header `name`, if the response has it. Fails the
    /// test if the header occurs more than once.
    #[track_caller]
    pub fn header(&self, name: &str) -> Option<&str> {
        let mut values = self
            .headers
            .iter()
            .filter(|(have, _)| have.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str());
        let first = values.next();
        assert!(
            values.next().is_none(),
            "the response has more than one `{name}` header: {:?}",
            self.headers
        );
        first
    }

    /// The value of `Content-Length`. [`Conn`] has already checked that it
    /// is there and is a number.
    pub fn content_length(&self) -> u64 {
        self.header("content-length")
            .and_then(|value| value.parse().ok())
            .expect("checked when the response was read")
    }

    /// True for `Connection: keep-alive`, false for `Connection: close`.
    pub fn keeps_alive(&self) -> bool {
        self.header("connection")
            .expect("checked when the response was read")
            .eq_ignore_ascii_case("keep-alive")
    }

    /// The body as text (invalid UTF-8 is replaced, never a failure).
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    /// Fails the test unless the status is `status`.
    #[track_caller]
    pub fn expect_status(&self, status: u16, context: &str) -> &Self {
        assert_eq!(
            self.status,
            status,
            "{context}: expected status {status}, got {} with body {:?}",
            self.status,
            abbreviate(&self.body)
        );
        self
    }

    /// Fails the test unless this is the standard response of SPEC 6.4 for
    /// `status`: a plain-text body that repeats the status line.
    #[track_caller]
    pub fn expect_error(&self, status: u16, context: &str) -> &Self {
        self.expect_status(status, context);
        assert_eq!(
            self.text(),
            format!("{status} {}\n", reason(status)),
            "{context}: the body of a {status} response must be the status code, a space, the reason \
             phrase and a line feed (SPEC 6.4)"
        );
        assert_eq!(
            self.header("content-type"),
            Some("text/plain; charset=utf-8"),
            "{context}: Content-Type of a {status} response (SPEC 6.4)"
        );
        self
    }

    /// Fails the test unless `Connection` says what `keep_alive` says.
    #[track_caller]
    pub fn expect_keep_alive(&self, keep_alive: bool, context: &str) -> &Self {
        assert_eq!(
            self.keeps_alive(),
            keep_alive,
            "{context}: the {} response has `Connection: {}`, expected `{}` (SPEC 7.1 and 7.2)",
            self.status,
            self.header("connection").unwrap_or_default(),
            if keep_alive { "keep-alive" } else { "close" }
        );
        self
    }
}

/// Why [`Conn::try_response`] could not return a response.
#[derive(Debug, Clone)]
pub struct ResponseError {
    /// True when the server closed the connection and not a single byte of
    /// a response had arrived. Some tests allow exactly this.
    pub closed_without_a_byte: bool,
    /// The text for a failure message.
    pub message: String,
}

impl std::fmt::Display for ResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl From<String> for ResponseError {
    fn from(message: String) -> Self {
        ResponseError {
            closed_without_a_byte: false,
            message,
        }
    }
}

/// One read from the socket.
enum Chunk {
    Data,
    /// The connection is over: a normal end of stream, or a reset.
    Closed,
    TimedOut,
}

/// A client connection. It keeps the bytes it has read and not yet used,
/// so several responses can be read one after the other.
pub struct Conn {
    stream: TcpStream,
    buffer: Vec<u8>,
    patience: Duration,
    /// Set when the connection ended with an error (a reset) instead of a
    /// normal end of stream.
    reset: Option<io::Error>,
}

impl Conn {
    /// Connects, or fails the test.
    #[track_caller]
    pub fn open(addr: SocketAddr) -> Conn {
        match Conn::try_open(addr) {
            Ok(conn) => conn,
            Err(e) => panic!("cannot connect to the server at {addr}: {e}"),
        }
    }

    /// Connects and reports failure as a value.
    pub fn try_open(addr: SocketAddr) -> io::Result<Conn> {
        let stream = TcpStream::connect_timeout(&addr, IO_TIMEOUT)?;
        // Small writes go out at once; the tests decide how a request is
        // split into packets.
        //
        // The results are ignored on purpose. Setting an option fails on a
        // socket that the server has already reset (macOS reports "invalid
        // argument"), and then the next read or write reports the real
        // state of the connection at once.
        let _ = stream.set_nodelay(true);
        let _ = stream.set_write_timeout(Some(IO_TIMEOUT));
        let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
        Ok(Conn {
            stream,
            buffer: Vec::new(),
            patience: IO_TIMEOUT,
            reset: None,
        })
    }

    /// Changes how long [`Conn::response`] waits for a complete response
    /// (the default is [`IO_TIMEOUT`]).
    pub fn patience(mut self, patience: Duration) -> Conn {
        self.patience = patience;
        self
    }

    /// Sends bytes, or fails the test.
    #[track_caller]
    pub fn send(&mut self, bytes: impl AsRef<[u8]>) {
        if let Err(e) = self.try_send(bytes) {
            panic!("cannot send to the server (did it close the connection too early?): {e}");
        }
    }

    /// Sends bytes and reports failure as a value.
    pub fn try_send(&mut self, bytes: impl AsRef<[u8]>) -> io::Result<()> {
        self.stream.write_all(bytes.as_ref())?;
        self.stream.flush()
    }

    /// A second handle to the same socket: for a helper thread that sends
    /// while the test reads, or for a test that wants to read the bytes
    /// itself, at its own speed. The time limits of the connection apply to
    /// this handle too, because they belong to the socket.
    pub fn writer(&self) -> TcpStream {
        self.stream
            .try_clone()
            .expect("test setup: clone the socket handle")
    }

    /// Closes the sending direction only; responses can still be read.
    pub fn finish_sending(&mut self) {
        let _ = self.stream.shutdown(Shutdown::Write);
    }

    /// Reads one response with its body, or fails the test.
    #[track_caller]
    pub fn response(&mut self) -> Response {
        // A `match`, not `unwrap_or_else`: a closure would hide the line of
        // the test from the panic message.
        match self.try_response(true) {
            Ok(response) => response,
            Err(problem) => panic!("{problem}"),
        }
    }

    /// Reads the response to a `HEAD` request: a head and no body.
    #[track_caller]
    pub fn head_response(&mut self) -> Response {
        match self.try_response(false) {
            Ok(response) => response,
            Err(problem) => panic!("{problem}"),
        }
    }

    /// Reads one response and checks its form against SPEC section 6.
    /// `with_body` is false for the response to a `HEAD` request.
    pub fn try_response(&mut self, with_body: bool) -> Result<Response, ResponseError> {
        let deadline = Instant::now() + self.patience;
        let head_end = loop {
            if let Some(at) = find(&self.buffer, b"\r\n\r\n") {
                break at + 4;
            }
            match self.read_chunk(deadline) {
                Chunk::Data => {}
                Chunk::Closed => {
                    return Err(ResponseError {
                        closed_without_a_byte: self.buffer.is_empty(),
                        message: format!(
                            "the server closed the connection before a complete response head arrived. If \
                             it did send a response, the way it closed the connection may have destroyed \
                             it: see SPEC 7.4. Received {} bytes: {:?}",
                            self.buffer.len(),
                            abbreviate(&self.buffer)
                        ),
                    });
                }
                Chunk::TimedOut => {
                    return Err(format!(
                        "no complete response head within {:?}. Received {} bytes: {:?}",
                        self.patience,
                        self.buffer.len(),
                        abbreviate(&self.buffer)
                    )
                    .into());
                }
            }
        };
        let (status, headers) = parse_head(&self.buffer[..head_end])?;
        let mut response = Response {
            status,
            headers,
            body: Vec::new(),
        };

        let lengths: Vec<&str> = response
            .headers
            .iter()
            .filter(|(name, _)| name == "content-length")
            .map(|(_, value)| value.as_str())
            .collect();
        let [length] = lengths[..] else {
            return Err(format!(
                "every response needs exactly one Content-Length header (SPEC 6.3); this {status} response \
                 has {}: {:?}",
                lengths.len(),
                response.headers
            )
            .into());
        };
        if length.is_empty() || !length.bytes().all(|b| b.is_ascii_digit()) {
            return Err(format!("Content-Length must be decimal digits, got {length:?}").into());
        }
        let length: usize = length
            .parse()
            .map_err(|_| format!("Content-Length {length:?} is too large"))?;

        let connections: Vec<&str> = response
            .headers
            .iter()
            .filter(|(name, _)| name == "connection")
            .map(|(_, value)| value.as_str())
            .collect();
        match connections[..] {
            [value]
                if value.eq_ignore_ascii_case("close")
                    || value.eq_ignore_ascii_case("keep-alive") => {}
            _ => {
                return Err(format!(
                    "every response needs exactly one Connection header that says `close` or `keep-alive` \
                     (SPEC 6.3); this {status} response has {connections:?}"
                )
                .into());
            }
        }

        let body_len = if with_body { length } else { 0 };
        while self.buffer.len() < head_end + body_len {
            match self.read_chunk(deadline) {
                Chunk::Data => {}
                Chunk::Closed => {
                    return Err(format!(
                        "the {status} response promised Content-Length: {length}, but the server closed the \
                         connection after {} bytes of body",
                        self.buffer.len() - head_end
                    )
                    .into());
                }
                Chunk::TimedOut => {
                    return Err(format!(
                        "the {status} response promised Content-Length: {length}, but only {} bytes of body \
                         arrived within {:?}",
                        self.buffer.len() - head_end,
                        self.patience
                    )
                    .into());
                }
            }
        }
        response.body = self.buffer[head_end..head_end + body_len].to_vec();
        self.buffer.drain(..head_end + body_len);
        Ok(response)
    }

    /// Waits until the server closes the connection and returns every byte
    /// that arrived before that (including bytes already buffered). Fails
    /// the test if the connection is still open after `within`.
    #[track_caller]
    pub fn read_until_closed(&mut self, within: Duration) -> Vec<u8> {
        let deadline = Instant::now() + within;
        loop {
            match self.read_chunk(deadline) {
                Chunk::Data => {}
                Chunk::Closed => return std::mem::take(&mut self.buffer),
                Chunk::TimedOut => panic!(
                    "the server should have closed the connection within {within:?}, but it is still open \
                     ({} unexpected bytes received: {:?})",
                    self.buffer.len(),
                    abbreviate(&self.buffer)
                ),
            }
        }
    }

    /// Fails the test unless the server closes the connection within
    /// `within` without sending another byte.
    #[track_caller]
    pub fn expect_closed(&mut self, within: Duration, context: &str) {
        let extra = self.read_until_closed(within);
        assert!(
            extra.is_empty(),
            "{context}: the server sent {} unexpected bytes before closing: {:?}",
            extra.len(),
            abbreviate(&extra)
        );
    }

    /// Like [`Conn::expect_closed`], and the connection must also end with
    /// a normal end of stream, not with a reset (SPEC 7.4).
    #[track_caller]
    pub fn expect_orderly_close(&mut self, within: Duration, context: &str) {
        self.expect_closed(within, context);
        if let Some(error) = &self.reset {
            panic!(
                "{context}: the connection did not end normally: {error}. The server closed its socket \
                 while bytes from the client were still unread, and the operating system turned that \
                 into a reset. A reset can destroy a response that is still on its way. Close the \
                 sending side first and read what the client still sends (SPEC 7.4)."
            );
        }
    }

    /// Fails the test if a byte arrives or the connection is closed during
    /// `duration`.
    #[track_caller]
    pub fn expect_silence(&mut self, duration: Duration, context: &str) {
        assert!(
            self.buffer.is_empty(),
            "{context}: unexpected bytes from the server: {:?}",
            abbreviate(&self.buffer)
        );
        match self.read_chunk(Instant::now() + duration) {
            Chunk::TimedOut => {}
            Chunk::Data => panic!(
                "{context}: the server sent bytes when it should have stayed silent for {duration:?}: {:?}",
                abbreviate(&self.buffer)
            ),
            Chunk::Closed => {
                panic!(
                    "{context}: the server closed the connection; it should have kept it open and silent"
                )
            }
        }
    }

    /// Waits up to `wait` for bytes from the server. Returns true if the
    /// server has sent something (now waiting in the buffer) or has closed
    /// the connection, false if nothing happened.
    pub fn wait_for_activity(&mut self, wait: Duration) -> bool {
        !self.buffer.is_empty()
            || !matches!(self.read_chunk(Instant::now() + wait), Chunk::TimedOut)
    }

    fn read_chunk(&mut self, deadline: Instant) -> Chunk {
        let mut chunk = [0u8; 16 * 1024];
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Chunk::TimedOut;
            }
            // If this fails the connection is dead (see `try_open`), and
            // the read below returns at once with what is left.
            let _ = self.stream.set_read_timeout(Some(remaining));
            return match self.stream.read(&mut chunk) {
                Ok(0) => Chunk::Closed,
                Ok(n) => {
                    self.buffer.extend_from_slice(&chunk[..n]);
                    Chunk::Data
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                // Unix reports a read timeout as WouldBlock, Windows as TimedOut.
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                    ) =>
                {
                    Chunk::TimedOut
                }
                // A reset or an abort: the connection is gone as well.
                Err(e) => {
                    self.reset = Some(e);
                    Chunk::Closed
                }
            };
        }
    }
}

/// Parses a response head and checks its form (SPEC 6.1 to 6.3).
fn parse_head(head: &[u8]) -> Result<(u16, Vec<(String, String)>), String> {
    let text = std::str::from_utf8(head)
        .map_err(|_| format!("the response head is not UTF-8: {:?}", abbreviate(head)))?;
    let text = text
        .strip_suffix("\r\n\r\n")
        .expect("the caller found the empty line");
    let mut lines = text.split("\r\n");
    let status_line = lines.next().unwrap_or_default();

    let malformed = || {
        format!(
            "the status line must be `HTTP/1.1 SP three-digit-code SP reason` (SPEC 6.1), got {status_line:?}"
        )
    };
    let rest = status_line
        .strip_prefix("HTTP/1.1 ")
        .ok_or_else(malformed)?;
    let (code, phrase) = rest.split_once(' ').ok_or_else(malformed)?;
    if code.len() != 3 || !code.bytes().all(|b| b.is_ascii_digit()) {
        return Err(malformed());
    }
    let status: u16 = code.parse().map_err(|_| malformed())?;
    match STATUSES.iter().find(|(known, _)| *known == status) {
        None => {
            return Err(format!(
                "status {status} is not in the table of SPEC 6.2: {status_line:?}"
            ));
        }
        Some((_, expected)) if *expected != phrase => {
            return Err(format!(
                "the reason phrase for {status} must be {expected:?} (SPEC 6.2), got {phrase:?}"
            ));
        }
        Some(_) => {}
    }

    let mut headers = Vec::new();
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            return Err(format!("a response header line has no colon: {line:?}"));
        };
        let is_name = !name.is_empty()
            && name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b));
        if !is_name || line.contains(['\r', '\n']) {
            return Err(format!("malformed response header line: {line:?}"));
        }
        headers.push((
            name.to_ascii_lowercase(),
            value.trim_matches([' ', '\t']).to_string(),
        ));
    }
    Ok((status, headers))
}

/// The position of the first occurrence of `needle`.
pub fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// At most the first 300 bytes, as text, for failure messages.
pub fn abbreviate(bytes: &[u8]) -> String {
    let shown = &bytes[..bytes.len().min(300)];
    let mut text = String::from_utf8_lossy(shown).into_owned();
    if bytes.len() > shown.len() {
        let _ = write!(text, "... ({} bytes in total)", bytes.len());
    }
    text
}

// ---------------------------------------------------------------------------
// Small tools
// ---------------------------------------------------------------------------

/// Deterministic bytes for file contents: every byte value occurs, and
/// neighbouring blocks differ, so a response with missing, repeated or
/// reordered blocks cannot equal the original.
pub fn pattern_bytes(len: usize) -> Vec<u8> {
    let mut state: u32 = 0x9E37_79B9;
    (0..len)
        .map(|_| {
            // xorshift32
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            (state >> 8) as u8
        })
        .collect()
}

/// Parses the fixed HTTP date format of SPEC 6.3, for example
/// `Sun, 06 Nov 1994 08:49:37 GMT`, into seconds since 1970-01-01 UTC.
/// The weekday must be the right one for the date.
pub fn parse_http_date(text: &str) -> Result<u64, String> {
    const DAYS: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let wrong =
        |what: &str| format!("{text:?} is not a date like `Sun, 06 Nov 1994 08:49:37 GMT`: {what}");
    let bytes = text.as_bytes();
    if bytes.len() != 29 || !text.is_ascii() {
        return Err(wrong("it must be exactly 29 ASCII characters"));
    }
    let fixed = [
        (3, ","),
        (4, " "),
        (7, " "),
        (11, " "),
        (16, " "),
        (19, ":"),
        (22, ":"),
        (25, " GMT"),
    ];
    for (at, expected) in fixed {
        if &text[at..at + expected.len()] != expected {
            return Err(wrong(&format!("expected {expected:?} at offset {at}")));
        }
    }
    let number = |range: std::ops::Range<usize>| -> Result<u64, String> {
        let digits = &text[range];
        if digits.bytes().all(|b| b.is_ascii_digit()) {
            Ok(digits.parse().expect("a few digits fit"))
        } else {
            Err(wrong(&format!("{digits:?} should be digits")))
        }
    };
    let (day, year) = (number(5..7)?, number(12..16)?);
    let (hour, minute, second) = (number(17..19)?, number(20..22)?, number(23..25)?);
    let month = MONTHS
        .iter()
        .position(|name| *name == &text[8..11])
        .ok_or_else(|| wrong("unknown month name"))? as u64
        + 1;
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let month_length = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if year < 1970 || !(1..=month_length).contains(&day) || hour > 23 || minute > 59 || second > 59
    {
        return Err(wrong("a field is out of range"));
    }

    // Days since 1970-01-01, counted the slow and obvious way.
    let mut days = day - 1;
    for earlier in 1970..year {
        let leap = earlier.is_multiple_of(4)
            && (!earlier.is_multiple_of(100) || earlier.is_multiple_of(400));
        days += if leap { 366 } else { 365 };
    }
    for earlier in 1..month {
        days += match earlier {
            2 if leap => 29,
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        };
    }
    let weekday = DAYS[(days % 7) as usize];
    if &text[0..3] != weekday {
        return Err(wrong(&format!("that day was a {weekday}")));
    }
    Ok(days * 86_400 + hour * 3_600 + minute * 60 + second)
}

/// Seconds since 1970-01-01 UTC on this machine's clock.
pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock of the test machine is set after 1970")
        .as_secs()
}
