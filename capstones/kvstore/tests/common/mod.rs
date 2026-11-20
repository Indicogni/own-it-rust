//! Helpers shared by the acceptance tests.
//!
//! The tests are black-box tests: they start your `kvstore` binary as a
//! child process, talk to it over TCP, stop it, and look at the files it
//! left behind. Nothing here calls into your code.
//!
//! Three things in this file are worth knowing when a test fails:
//!
//! - Every wait has a time limit (10 seconds by default; set the environment
//!   variable `KVSTORE_TEST_TIMEOUT_SECS` to change it). A server that does
//!   not answer makes the test fail with a message; it never hangs the run.
//! - A [`Server`] kills its child process when it is dropped, also when the
//!   test fails. No server survives a test run.
//! - A [`TestDir`] is deleted when its test passes and **kept when the test
//!   fails**; the failure output names the directory so that you can look at
//!   the log file.
//!
//! Each file in `tests/` is its own crate and uses only some of the helpers,
//! so unused-code warnings are switched off here.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, Shutdown, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// The server binary that Cargo built for this test run.
pub const BIN: &str = env!("CARGO_BIN_EXE_kvstore");

// ---------------------------------------------------------------------------
// Limits and names from SPEC.md, repeated here so that the tests do not
// depend on the program they check.
// ---------------------------------------------------------------------------

/// SPEC 7: the longest key, in bytes.
pub const MAX_KEY: usize = 128;
/// SPEC 7: the longest value, in bytes.
pub const MAX_VALUE: usize = 4096;
/// SPEC 8.1: the longest request line, in bytes, without its line feed.
pub const MAX_LINE: usize = 8192;
/// SPEC 5.1: the eight bytes at the start of the log file.
pub const MAGIC: &[u8; 8] = b"KVSTORE1";
/// SPEC 5.2: the size of the fixed part of a record.
pub const RECORD_HEADER: usize = 11;
/// SPEC 4: file names inside the data directory.
pub const LOG_FILE: &str = "kv.log";
pub const COMPACT_FILE: &str = "kv.compact";

/// How long a test waits for anything before it fails.
pub fn patience() -> Duration {
    let secs = std::env::var("KVSTORE_TEST_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    Duration::from_secs(secs)
}

/// SPEC 12: a server must have exited this long after it answered
/// `SHUTDOWN`.
pub const SHUTDOWN_LIMIT: Duration = Duration::from_secs(5);

/// Shows bytes in a failure message: printable ASCII as it is, everything
/// else as an escape such as `\n` or `\xff`.
pub fn show(bytes: &[u8]) -> String {
    const MOST: usize = 120;
    let mut text = bytes
        .iter()
        .take(MOST)
        .flat_map(|b| std::ascii::escape_default(*b))
        .map(char::from)
        .collect::<String>();
    if bytes.len() > MOST {
        text.push_str(&format!("... ({} bytes in all)", bytes.len()));
    }
    text
}

// ---------------------------------------------------------------------------
// A data directory for one test.
// ---------------------------------------------------------------------------

/// A fresh directory below Cargo's target directory. See the module comment
/// for when it is deleted.
pub struct TestDir {
    path: PathBuf,
}

impl TestDir {
    /// `name` should be the name of the test; it becomes part of the path.
    pub fn new(name: &str) -> TestDir {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let unique = format!(
            "kvstore-{name}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(unique);
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path)
            .unwrap_or_else(|e| panic!("cannot create the test directory {}: {e}", path.display()));
        TestDir { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The path of the log file inside this directory.
    pub fn log(&self) -> PathBuf {
        self.path.join(LOG_FILE)
    }

    /// The bytes of the log file. Only call this while no command is in
    /// flight: after a response has arrived, or after the server has stopped.
    #[track_caller]
    pub fn read_log(&self) -> Vec<u8> {
        std::fs::read(self.log())
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", self.log().display()))
    }

    /// The size of the log file in bytes.
    #[track_caller]
    pub fn log_len(&self) -> u64 {
        std::fs::metadata(self.log())
            .unwrap_or_else(|e| panic!("cannot stat {}: {e}", self.log().display()))
            .len()
    }

    /// Replaces the log file. The server must not be running.
    #[track_caller]
    pub fn write_log(&self, bytes: &[u8]) {
        std::fs::write(self.log(), bytes)
            .unwrap_or_else(|e| panic!("cannot write {}: {e}", self.log().display()));
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        if std::thread::panicking() {
            // Nothing to look at in an empty directory.
            if std::fs::remove_dir(&self.path).is_ok() {
                return;
            }
            eprintln!(
                "note: the data directory of the failed test was kept: {}",
                self.path.display()
            );
            return;
        }
        // On Windows a file can stay busy for a moment after its process has
        // gone, so try a few times and then give up quietly.
        for _ in 0..20 {
            if std::fs::remove_dir_all(&self.path).is_ok() {
                return;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }
}

// ---------------------------------------------------------------------------
// The server as a child process.
// ---------------------------------------------------------------------------

/// Collects everything a child writes to one of its output pipes, on a
/// helper thread, so that a talkative server can never block on a full pipe.
struct Pipe {
    bytes: Arc<Mutex<Vec<u8>>>,
    thread: Option<JoinHandle<()>>,
}

impl Pipe {
    /// If `first_line` is given, the first complete line (with its line
    /// feed) is sent through it as soon as it has been read. The sender is
    /// dropped when the pipe closes, which tells the receiver that no line
    /// will ever come.
    fn drain(
        mut source: impl Read + Send + 'static,
        first_line: Option<mpsc::Sender<Vec<u8>>>,
    ) -> Pipe {
        const KEEP: usize = 1 << 20;
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let shared = Arc::clone(&bytes);
        let thread = std::thread::spawn(move || {
            let mut first_line = first_line;
            let mut chunk = [0u8; 4096];
            loop {
                let n = match source.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => n,
                };
                let mut all = shared.lock().unwrap_or_else(|e| e.into_inner());
                if all.len() < KEEP {
                    all.extend_from_slice(&chunk[..n]);
                }
                if first_line.is_some()
                    && let Some(end) = all.iter().position(|&b| b == b'\n')
                    && let Some(tx) = first_line.take()
                {
                    let _ = tx.send(all[..=end].to_vec());
                }
            }
        });
        Pipe {
            bytes,
            thread: Some(thread),
        }
    }

    /// What has been read so far. If the child has exited, this waits a
    /// moment for the helper thread to reach the end of the pipe.
    fn text(&mut self) -> String {
        if let Some(thread) = self.thread.take() {
            let deadline = Instant::now() + Duration::from_secs(2);
            while !thread.is_finished() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(5));
            }
            if thread.is_finished() {
                let _ = thread.join();
            } else {
                self.thread = Some(thread);
            }
        }
        let bytes = self.bytes.lock().unwrap_or_else(|e| e.into_inner());
        String::from_utf8_lossy(&bytes).into_owned()
    }
}

/// What is known about a server process that has ended.
#[derive(Debug)]
pub struct Exited {
    /// The exit code; `None` if a signal ended the process.
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl fmt::Display for Exited {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.code {
            Some(code) => write!(f, "exit code {code}")?,
            None => write!(f, "ended by a signal")?,
        }
        write!(
            f,
            "\n--- server stdout ---\n{}\n--- server stderr ---\n{}",
            if self.stdout.is_empty() {
                "(nothing)"
            } else {
                self.stdout.trim_end()
            },
            if self.stderr.is_empty() {
                "(nothing)"
            } else {
                self.stderr.trim_end()
            },
        )
    }
}

/// A running server. Dropping it kills the process.
pub struct Server {
    child: Child,
    /// The address from the `LISTENING` line.
    pub addr: SocketAddr,
    stdout: Pipe,
    stderr: Pipe,
}

/// The options most tests use: no fsync (the tests crash processes, not
/// machines, so it would only make them slow) and no automatic compaction
/// (so that the size of the log is predictable).
pub const FAST: &[&str] = &["--fsync", "never", "--auto-compact", "0"];

impl Server {
    /// Starts `kvstore --dir <dir> --port 0` with [`FAST`] options and waits
    /// for its `LISTENING` line. Fails the test if the server does not come
    /// up.
    #[track_caller]
    pub fn start(dir: &Path) -> Server {
        Server::start_with(dir, FAST)
    }

    /// Like [`Server::start`], with your own options after `--dir` and
    /// `--port 0`.
    #[track_caller]
    pub fn start_with(dir: &Path, options: &[&str]) -> Server {
        match Server::launch(&server_args(dir, options)) {
            Ok(server) => server,
            Err(exited) => panic!(
                "the server stopped before it printed its LISTENING line (SPEC 3.3): {exited}"
            ),
        }
    }

    /// Starts the binary with exactly `args`. `Ok` if it printed a correct
    /// `LISTENING` line, `Err` if it exited without printing any line.
    #[track_caller]
    pub fn launch(args: &[OsString]) -> Result<Server, Exited> {
        let mut child = Command::new(BIN)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("cannot start {BIN}: {e}"));
        let (tx, rx) = mpsc::channel();
        let stdout = Pipe::drain(child.stdout.take().expect("stdout is piped"), Some(tx));
        let stderr = Pipe::drain(child.stderr.take().expect("stderr is piped"), None);
        let mut server = Server {
            child,
            addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
            stdout,
            stderr,
        };
        match rx.recv_timeout(patience()) {
            Ok(line) => match parse_listening(&line) {
                Some(addr) => {
                    server.addr = addr;
                    Ok(server)
                }
                None => {
                    server.stop();
                    panic!(
                        "the first line on standard output must be `LISTENING 127.0.0.1:<port>` \
                         (SPEC 3.3), but the server printed `{}`",
                        show(&line)
                    );
                }
            },
            Err(RecvTimeoutError::Timeout) => {
                server.stop();
                panic!(
                    "the server printed no complete line on standard output within {} s \
                     (SPEC 3.3). Is the LISTENING line missing its line feed, or not flushed?\n\
                     --- server stderr ---\n{}",
                    patience().as_secs(),
                    server.stderr.text()
                );
            }
            // Standard output was closed without a line: the process has
            // ended, or is about to.
            Err(RecvTimeoutError::Disconnected) => match server.wait_exit(patience()) {
                Some(exited) => Err(exited),
                None => {
                    server.stop();
                    panic!(
                        "the server closed standard output without printing a LISTENING line, \
                         but kept running"
                    );
                }
            },
        }
    }

    /// A new connection to this server.
    #[track_caller]
    pub fn client(&self) -> Client {
        Client::connect(self.addr)
    }

    /// Simulates a crash: ends the process at once, without any chance to
    /// clean up (`SIGKILL` on Unix, `TerminateProcess` on Windows), and waits
    /// until it is gone.
    pub fn kill(mut self) {
        self.stop();
    }

    /// Sends `SHUTDOWN` on a new connection, checks the `OK`, and waits for
    /// the process to exit with code 0 within [`SHUTDOWN_LIMIT`] (SPEC 12).
    #[track_caller]
    pub fn shutdown(self) -> Exited {
        let mut client = self.client();
        let reply = client.req("SHUTDOWN");
        assert_eq!(reply, "OK", "SHUTDOWN must be answered with OK (SPEC 12)");
        self.expect_clean_exit()
    }

    /// Waits for a server that has already been told to shut down.
    #[track_caller]
    pub fn expect_clean_exit(mut self) -> Exited {
        match self.wait_exit(SHUTDOWN_LIMIT) {
            Some(exited) => {
                assert_eq!(
                    exited.code,
                    Some(0),
                    "after SHUTDOWN the process must exit with code 0 (SPEC 12): {exited}"
                );
                exited
            }
            None => {
                self.stop();
                panic!(
                    "the server was still running {} s after it answered SHUTDOWN (SPEC 12). \
                     Look for a thread that is blocked in `accept` or in a read from an idle \
                     client.\n--- server stderr ---\n{}",
                    SHUTDOWN_LIMIT.as_secs(),
                    self.stderr.text()
                );
            }
        }
    }

    /// Waits up to `limit` for the process to end by itself.
    pub fn wait_exit(&mut self, limit: Duration) -> Option<Exited> {
        let deadline = Instant::now() + limit;
        loop {
            match self.child.try_wait() {
                Ok(Some(status)) => {
                    return Some(Exited {
                        code: status.code(),
                        stdout: self.stdout.text(),
                        stderr: self.stderr.text(),
                    });
                }
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(5));
                }
                _ => return None,
            }
        }
    }

    /// What the server has written to standard error so far.
    pub fn stderr(&mut self) -> String {
        self.stderr.text()
    }

    fn stop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop();
    }
}

/// `--dir <dir> --port 0` followed by `options`.
pub fn server_args(dir: &Path, options: &[&str]) -> Vec<OsString> {
    let mut args = vec![
        OsString::from("--dir"),
        dir.as_os_str().to_owned(),
        OsString::from("--port"),
        OsString::from("0"),
    ];
    args.extend(options.iter().map(OsString::from));
    args
}

/// Turns string arguments into the form [`Server::launch`] takes.
pub fn os_args<S: AsRef<OsStr>>(args: &[S]) -> Vec<OsString> {
    args.iter().map(|a| a.as_ref().to_owned()).collect()
}

fn parse_listening(line: &[u8]) -> Option<SocketAddr> {
    let line = std::str::from_utf8(line).ok()?;
    let port = line
        .strip_prefix("LISTENING 127.0.0.1:")?
        .strip_suffix('\n')?;
    // `u16::from_str` would also accept `+80`; the SPEC wants digits only.
    if port.is_empty() || !port.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let port: u16 = port.parse().ok()?;
    (port != 0).then(|| SocketAddr::from((Ipv4Addr::LOCALHOST, port)))
}

/// Runs the binary with `args` and requires that it refuses to start: it
/// must exit with `code`, print nothing on standard output and something on
/// standard error (SPEC 3.4).
#[track_caller]
pub fn expect_refusal(args: &[OsString], code: i32, why: &str) -> Exited {
    match Server::launch(args) {
        Ok(server) => {
            let addr = server.addr;
            drop(server);
            panic!(
                "{why}: the server must exit with code {code}, but it started and printed \
                 `LISTENING {addr}`"
            );
        }
        Err(exited) => {
            assert_eq!(
                exited.code,
                Some(code),
                "{why}: wrong exit code (SPEC 3.4): {exited}"
            );
            assert!(
                exited.stdout.is_empty(),
                "{why}: a server that does not start must print nothing on standard output \
                 (SPEC 3.4): {exited}"
            );
            assert!(
                !exited.stderr.trim().is_empty(),
                "{why}: the reason must be printed on standard error (SPEC 3.4): {exited}"
            );
            exited
        }
    }
}

// ---------------------------------------------------------------------------
// A client connection.
// ---------------------------------------------------------------------------

/// One TCP connection with read and write time limits.
pub struct Client {
    reader: BufReader<TcpStream>,
    stream: TcpStream,
    /// The start of the last request, for failure messages.
    last: Vec<u8>,
}

/// The numbers in a `STATS` response (SPEC 8.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    pub keys: u64,
    pub log_bytes: u64,
    pub live_bytes: u64,
    pub compactions: u64,
}

impl Stats {
    /// SPEC 10: the bytes of the log that a compaction would remove.
    pub fn garbage(&self) -> u64 {
        self.log_bytes.saturating_sub(self.live_bytes)
    }
}

impl Client {
    #[track_caller]
    pub fn connect(addr: SocketAddr) -> Client {
        Client::try_connect(addr).unwrap_or_else(|e| panic!("cannot connect to {addr}: {e}"))
    }

    pub fn try_connect(addr: SocketAddr) -> io::Result<Client> {
        let stream = TcpStream::connect_timeout(&addr, patience())?;
        stream.set_read_timeout(Some(patience()))?;
        stream.set_write_timeout(Some(patience()))?;
        // The tests send many small requests and wait for each answer.
        stream.set_nodelay(true)?;
        Ok(Client {
            reader: BufReader::new(stream.try_clone()?),
            stream,
            last: Vec::new(),
        })
    }

    /// Sends bytes exactly as given, without adding a line feed.
    #[track_caller]
    pub fn send_raw(&mut self, bytes: &[u8]) {
        self.try_send_raw(bytes).unwrap_or_else(|e| {
            panic!(
                "cannot send `{}` to the server: {e}. Did the server close the connection or \
                 stop reading?",
                show(bytes)
            )
        });
    }

    pub fn try_send_raw(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.last = bytes[..bytes.len().min(80)].to_vec();
        self.stream.write_all(bytes)
    }

    /// Sends one request line; the line feed is added here.
    #[track_caller]
    pub fn send(&mut self, line: impl AsRef<[u8]>) {
        let mut bytes = line.as_ref().to_vec();
        bytes.push(b'\n');
        self.send_raw(&bytes);
    }

    /// Reads one line without its line feed. `Ok(None)` means the server
    /// closed the connection before sending another byte.
    pub fn try_recv(&mut self) -> io::Result<Option<Vec<u8>>> {
        let mut line = Vec::new();
        self.reader.read_until(b'\n', &mut line)?;
        if line.is_empty() {
            return Ok(None);
        }
        if line.pop() != Some(b'\n') {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "the connection was closed in the middle of a response line",
            ));
        }
        Ok(Some(line))
    }

    /// Reads one response line, without its line feed.
    #[track_caller]
    pub fn recv(&mut self) -> Vec<u8> {
        match self.try_recv() {
            Ok(Some(line)) => line,
            Ok(None) => panic!(
                "the server closed the connection instead of answering `{}`",
                show(&self.last)
            ),
            Err(e) if is_timeout(&e) => panic!(
                "no complete response line within {} s after `{}`. Does every response end \
                 with a line feed, and is it flushed?",
                patience().as_secs(),
                show(&self.last)
            ),
            Err(e) => panic!("reading the response to `{}` failed: {e}", show(&self.last)),
        }
    }

    /// Sends one request line and returns the response line as bytes.
    #[track_caller]
    pub fn req_bytes(&mut self, line: impl AsRef<[u8]>) -> Vec<u8> {
        self.send(line);
        self.recv()
    }

    /// Sends one request line and returns the response line as text
    /// (bytes that are not UTF-8 become U+FFFD).
    #[track_caller]
    pub fn req(&mut self, line: impl AsRef<[u8]>) -> String {
        String::from_utf8_lossy(&self.req_bytes(line)).into_owned()
    }

    /// `SET`, which must answer `OK`.
    #[track_caller]
    pub fn set(&mut self, key: impl AsRef<[u8]>, value: impl AsRef<[u8]>) {
        let mut line = b"SET ".to_vec();
        line.extend_from_slice(key.as_ref());
        line.push(b' ');
        line.extend_from_slice(value.as_ref());
        let reply = self.req_bytes(&line);
        assert_eq!(
            show(&reply),
            "OK",
            "`{}` must be answered with OK (SPEC 8.5)",
            show(&line)
        );
    }

    /// `GET`: the value, or `None` for `NOT_FOUND`.
    #[track_caller]
    pub fn get(&mut self, key: impl AsRef<[u8]>) -> Option<Vec<u8>> {
        let mut line = b"GET ".to_vec();
        line.extend_from_slice(key.as_ref());
        let reply = self.req_bytes(&line);
        if reply == b"NOT_FOUND" {
            None
        } else if let Some(value) = reply.strip_prefix(b"VALUE ") {
            Some(value.to_vec())
        } else {
            panic!(
                "`{}` must be answered with `VALUE <value>` or `NOT_FOUND` (SPEC 8.4), not `{}`",
                show(&line),
                show(&reply)
            )
        }
    }

    /// `DEL`: `true` for `OK`, `false` for `NOT_FOUND`.
    #[track_caller]
    pub fn del(&mut self, key: impl AsRef<[u8]>) -> bool {
        let mut line = b"DEL ".to_vec();
        line.extend_from_slice(key.as_ref());
        let reply = self.req(&line);
        match reply.as_str() {
            "OK" => true,
            "NOT_FOUND" => false,
            _ => panic!(
                "`{}` must be answered with OK or NOT_FOUND (SPEC 8.6), not `{reply}`",
                show(&line)
            ),
        }
    }

    /// `KEYS` with an optional prefix: the listed keys in the order sent.
    #[track_caller]
    pub fn keys(&mut self, prefix: Option<&str>) -> Vec<String> {
        let request = match prefix {
            Some(prefix) => format!("KEYS {prefix}"),
            None => "KEYS".to_string(),
        };
        let head = self.req(&request);
        let count: usize = head
            .strip_prefix("KEYS ")
            .and_then(|n| n.parse().ok())
            .unwrap_or_else(|| {
                panic!("`{request}` must be answered with `KEYS <count>` first (SPEC 8.8), not `{head}`")
            });
        (0..count)
            .map(|i| match self.try_recv() {
                Ok(Some(key)) => String::from_utf8_lossy(&key).into_owned(),
                Ok(None) => panic!(
                    "`{request}` was answered with `{head}`, but the server closed the \
                     connection after {i} of the {count} key lines (SPEC 8.8)"
                ),
                Err(e) if is_timeout(&e) => panic!(
                    "`{request}` was answered with `{head}`, but only {i} key lines followed \
                     within {} s. The count must be the number of lines that follow it \
                     (SPEC 8.8)",
                    patience().as_secs()
                ),
                Err(e) => panic!("reading the key lines after `{head}` failed: {e}"),
            })
            .collect()
    }

    /// `STATS`, parsed.
    #[track_caller]
    pub fn stats(&mut self) -> Stats {
        let reply = self.req("STATS");
        let parsed = (|| {
            let mut fields = reply.strip_prefix("STATS ")?.split(' ');
            let mut next = |name: &str| -> Option<u64> {
                let digits = fields.next()?.strip_prefix(name)?.strip_prefix('=')?;
                if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                    return None;
                }
                digits.parse().ok()
            };
            Some(Stats {
                keys: next("keys")?,
                log_bytes: next("log_bytes")?,
                live_bytes: next("live_bytes")?,
                compactions: next("compactions")?,
            })
        })();
        parsed.unwrap_or_else(|| {
            panic!(
                "STATS must be answered with `STATS keys=<n> log_bytes=<n> live_bytes=<n> \
                 compactions=<n>` (SPEC 8.9), not `{reply}`"
            )
        })
    }

    /// Everything in the store: `KEYS`, then one `GET` per key.
    #[track_caller]
    pub fn dump(&mut self) -> Model {
        let keys = self.keys(None);
        let mut all = Model::new();
        for key in keys {
            let value = self
                .get(&key)
                .unwrap_or_else(|| panic!("KEYS listed `{key}`, but GET says NOT_FOUND"));
            all.insert(key.into_bytes(), value);
        }
        all
    }

    /// Sends `requests` (complete lines, line feeds included) in one piece
    /// from a helper thread while this thread reads `responses` lines. A
    /// client that pipelines must read while it writes: otherwise both sides
    /// can end up waiting for the other to make room.
    #[track_caller]
    pub fn pipeline(&mut self, requests: &[u8], responses: usize) -> Vec<Vec<u8>> {
        let mut sender = self
            .stream
            .try_clone()
            .unwrap_or_else(|e| panic!("cannot clone the connection: {e}"));
        self.last = requests[..requests.len().min(80)].to_vec();
        std::thread::scope(|scope| {
            let writer = scope.spawn(move || sender.write_all(requests));
            let replies: Vec<Vec<u8>> = (0..responses).map(|_| self.recv()).collect();
            if let Err(e) = writer.join().expect("the pipeline writer panicked") {
                panic!("sending pipelined requests failed: {e}");
            }
            replies
        })
    }

    /// Tells the server that no more requests will come (closes the sending
    /// half of the connection). Responses can still be read.
    #[track_caller]
    pub fn finish_sending(&mut self) {
        self.stream
            .shutdown(Shutdown::Write)
            .unwrap_or_else(|e| panic!("cannot close the sending half of the connection: {e}"));
    }

    /// Everything the server still sends until it closes the connection.
    #[track_caller]
    pub fn read_to_end(&mut self) -> Vec<u8> {
        let mut rest = Vec::new();
        match self.reader.read_to_end(&mut rest) {
            Ok(_) => rest,
            Err(e) if is_timeout(&e) => panic!(
                "the server did not close the connection within {} s (it had sent `{}` so far)",
                patience().as_secs(),
                show(&rest)
            ),
            // A reset also ends the connection; keep what arrived before it.
            Err(_) => rest,
        }
    }

    /// Requires that the server ends this connection without sending
    /// another byte. A connection reset counts as closed.
    #[track_caller]
    pub fn expect_closed(&mut self, when: &str) {
        let mut byte = [0u8; 1];
        match self.reader.read(&mut byte) {
            Ok(0) => {}
            Ok(_) => panic!(
                "{when}: the server must close the connection, but it sent more data \
                 (first byte `{}`)",
                show(&byte)
            ),
            Err(e) if is_timeout(&e) => panic!(
                "{when}: the server must close the connection, but it was still open after {} s",
                patience().as_secs()
            ),
            Err(_) => {}
        }
    }
}

/// A read or write that ran into the time limit. Unix reports `WouldBlock`,
/// Windows `TimedOut`.
pub fn is_timeout(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
    )
}

/// Asks `STATS` again and again until `done` accepts the answer; fails the
/// test with `what` after [`patience`].
#[track_caller]
pub fn wait_for_stats(client: &mut Client, what: &str, done: impl Fn(&Stats) -> bool) -> Stats {
    let deadline = Instant::now() + patience();
    loop {
        let stats = client.stats();
        if done(&stats) {
            return stats;
        }
        assert!(
            Instant::now() < deadline,
            "{what}: still not true after {} s; the last answer was {stats:?}",
            patience().as_secs()
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Adds one to a counter when it goes out of scope, also when its thread
/// panics. Tests in which some threads work until "all the others have
/// finished" count with this: a thread that fails still counts as finished,
/// so the test fails with its message and does not run for ever.
pub struct CountWhenDone<'a>(pub &'a AtomicUsize);

impl Drop for CountWhenDone<'_> {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

// ---------------------------------------------------------------------------
// The log format of SPEC 5, written out a second time. The tests build log
// files with it and take the server's files apart with it.
// ---------------------------------------------------------------------------

/// What the store should contain: keys in byte order with their values.
pub type Model = BTreeMap<Vec<u8>, Vec<u8>>;

/// Builds a [`Model`] from string pairs.
pub fn model(pairs: &[(&str, &str)]) -> Model {
    pairs
        .iter()
        .map(|(k, v)| (k.as_bytes().to_vec(), v.as_bytes().to_vec()))
        .collect()
}

/// CRC-32 as SPEC 5.3 defines it, one bit at a time.
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

pub const KIND_SET: u8 = 1;
pub const KIND_DEL: u8 = 2;

/// A record with a correct checksum but any header fields and any body, for
/// tests that need records no correct server would write.
pub fn raw_record(kind: u8, klen: u16, vlen: u32, body: &[u8]) -> Vec<u8> {
    let mut rest = vec![kind];
    rest.extend_from_slice(&klen.to_le_bytes());
    rest.extend_from_slice(&vlen.to_le_bytes());
    rest.extend_from_slice(body);
    let mut record = crc32(&rest).to_le_bytes().to_vec();
    record.extend_from_slice(&rest);
    record
}

/// The record for `SET key value`.
pub fn set_record(key: impl AsRef<[u8]>, value: impl AsRef<[u8]>) -> Vec<u8> {
    let (key, value) = (key.as_ref(), value.as_ref());
    let body = [key, value].concat();
    raw_record(KIND_SET, key.len() as u16, value.len() as u32, &body)
}

/// The record for `DEL key`.
pub fn del_record(key: impl AsRef<[u8]>) -> Vec<u8> {
    let key = key.as_ref();
    raw_record(KIND_DEL, key.len() as u16, 0, key)
}

/// The size of the record that stores `key` with `value`.
pub fn record_len(key: &[u8], value: &[u8]) -> u64 {
    (RECORD_HEADER + key.len() + value.len()) as u64
}

/// A log file: the magic bytes followed by `records`.
pub fn log_of(records: &[Vec<u8>]) -> Vec<u8> {
    let mut log = MAGIC.to_vec();
    for record in records {
        log.extend_from_slice(record);
    }
    log
}

/// SPEC 10.1: the log a compaction must produce for this content.
pub fn canonical_log(model: &Model) -> Vec<u8> {
    let records: Vec<Vec<u8>> = model.iter().map(|(k, v)| set_record(k, v)).collect();
    log_of(&records)
}

/// SPEC 8.9: `live_bytes` for this content.
pub fn live_bytes(model: &Model) -> u64 {
    MAGIC.len() as u64 + model.iter().map(|(k, v)| record_len(k, v)).sum::<u64>()
}

/// One record of a log file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Record {
    Set(Vec<u8>, Vec<u8>),
    Del(Vec<u8>),
}

impl Record {
    pub fn set(key: &str, value: &str) -> Record {
        Record::Set(key.as_bytes().to_vec(), value.as_bytes().to_vec())
    }

    pub fn del(key: &str) -> Record {
        Record::Del(key.as_bytes().to_vec())
    }
}

/// A log file taken apart by the rules of SPEC 5 and 6.
#[derive(Debug)]
pub struct ParsedLog {
    /// The valid records, in file order.
    pub records: Vec<Record>,
    /// Where the valid prefix ends. Equal to the file size if every byte of
    /// the file belongs to a valid record.
    pub valid_len: u64,
    /// The size of the file.
    pub file_len: u64,
}

impl ParsedLog {
    /// The content that replaying the valid records gives.
    pub fn replay(&self) -> Model {
        let mut model = Model::new();
        for record in &self.records {
            match record {
                Record::Set(k, v) => {
                    model.insert(k.clone(), v.clone());
                }
                Record::Del(k) => {
                    model.remove(k);
                }
            }
        }
        model
    }

    /// Fails unless the whole file is valid records.
    #[track_caller]
    pub fn expect_clean(&self, when: &str) {
        assert_eq!(
            self.valid_len, self.file_len,
            "{when}: the log must end at a record boundary, but only the first {} of its {} \
             bytes are valid records (SPEC 5)",
            self.valid_len, self.file_len
        );
    }
}

/// Reads a log file the way SPEC 6 tells a server to: record after record,
/// stopping at the first one that is not valid.
#[track_caller]
pub fn parse_log(bytes: &[u8]) -> ParsedLog {
    assert!(
        bytes.len() >= MAGIC.len() && &bytes[..MAGIC.len()] == MAGIC,
        "the log file must start with the 8 bytes `KVSTORE1` (SPEC 5.1), but it starts with `{}`",
        show(&bytes[..bytes.len().min(16)])
    );
    let mut records = Vec::new();
    let mut at = MAGIC.len();
    while let Some((record, len)) = parse_record(&bytes[at..]) {
        records.push(record);
        at += len;
    }
    ParsedLog {
        records,
        valid_len: at as u64,
        file_len: bytes.len() as u64,
    }
}

fn parse_record(bytes: &[u8]) -> Option<(Record, usize)> {
    let header = bytes.get(..RECORD_HEADER)?;
    let crc = u32::from_le_bytes(header[0..4].try_into().unwrap());
    let kind = header[4];
    let klen = usize::from(u16::from_le_bytes(header[5..7].try_into().unwrap()));
    let vlen = u32::from_le_bytes(header[7..11].try_into().unwrap()) as usize;
    let lengths_ok = (1..=MAX_KEY).contains(&klen)
        && match kind {
            KIND_SET => vlen <= MAX_VALUE,
            KIND_DEL => vlen == 0,
            _ => false,
        };
    if !lengths_ok {
        return None;
    }
    let len = RECORD_HEADER + klen + vlen;
    let record = bytes.get(..len)?;
    if crc32(&record[4..]) != crc {
        return None;
    }
    let key = &record[RECORD_HEADER..RECORD_HEADER + klen];
    let value = &record[RECORD_HEADER + klen..];
    if !key.iter().all(|b| (0x21..=0x7E).contains(b)) || value.contains(&b'\n') {
        return None;
    }
    let record = if kind == KIND_SET {
        Record::Set(key.to_vec(), value.to_vec())
    } else {
        Record::Del(key.to_vec())
    };
    Some((record, len))
}

// ---------------------------------------------------------------------------
// A small random number generator with a fixed seed, so that "random" tests
// do the same thing on every run and on every machine.
// ---------------------------------------------------------------------------

/// SplitMix64.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number in `0..n`. `n` must not be zero.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}
