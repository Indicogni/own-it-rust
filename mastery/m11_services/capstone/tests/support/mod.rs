//! Helpers for the acceptance tests. Supplied; you do not need to edit this file.
//!
//! Everything here is blocking, std-only code with timeouts, on purpose: the
//! tests must not share an async runtime, a bug or a hang with the programs
//! they judge.

#![allow(dead_code)]

use beacon_wire::{Message, decode_frame, encode_frame};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// A child process that is killed and reaped when the guard is dropped, also
/// when the test panics.
pub struct ChildGuard {
    name: String,
    child: Child,
    stdout: mpsc::Receiver<String>,
    stderr: Arc<Mutex<String>>,
    stderr_reader: Option<JoinHandle<()>>,
}

impl ChildGuard {
    #[track_caller]
    pub fn spawn(name: &str, command: &mut Command) -> ChildGuard {
        // The programs under test read BEACON_ variables; none may leak in
        // from the shell that runs the tests.
        for (variable, _) in std::env::vars() {
            if variable.starts_with("BEACON_")
                && command.get_envs().all(|(k, _)| k != variable.as_str())
            {
                command.env_remove(variable);
            }
        }
        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|err| panic!("starting {name} failed: {err}"));
        let (line_tx, stdout) = mpsc::channel();
        let out = child.stdout.take().expect("piped stdout");
        std::thread::spawn(move || {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                if line_tx.send(line).is_err() {
                    break;
                }
            }
        });
        let stderr = Arc::new(Mutex::new(String::new()));
        let collected = Arc::clone(&stderr);
        let mut err = child.stderr.take().expect("piped stderr");
        let stderr_reader = std::thread::spawn(move || {
            let mut chunk = [0u8; 1024];
            while let Ok(count) = err.read(&mut chunk) {
                if count == 0 {
                    break;
                }
                let text = String::from_utf8_lossy(&chunk[..count]).into_owned();
                collected.lock().expect("stderr lock").push_str(&text);
            }
        });
        ChildGuard {
            name: name.to_owned(),
            child,
            stdout,
            stderr,
            stderr_reader: Some(stderr_reader),
        }
    }

    /// After the process has ended: wait until its standard error has been read
    /// to the end, so that a test never sees only half of the output.
    fn finish_stderr(&mut self) {
        if let Some(reader) = self.stderr_reader.take() {
            let _ = reader.join();
        }
    }

    /// Everything the process has written to standard error so far.
    pub fn stderr(&self) -> String {
        self.stderr.lock().expect("stderr lock").clone()
    }

    /// The next line of standard output, or a panic that shows the standard
    /// error of the process.
    #[track_caller]
    pub fn line(&mut self, secs: u64) -> String {
        match self.stdout.recv_timeout(Duration::from_secs(secs)) {
            Ok(line) => line,
            Err(RecvTimeoutError::Timeout) => panic!(
                "{} printed no line within {secs} s. Its stderr so far:\n{}",
                self.name,
                self.stderr()
            ),
            Err(RecvTimeoutError::Disconnected) => {
                // The output ended, so the process is ending too: give it a
                // moment, then collect everything it wrote to standard error.
                let give_up = Instant::now() + Duration::from_secs(2);
                let mut status = self.child.try_wait().ok().flatten();
                while status.is_none() && Instant::now() < give_up {
                    std::thread::sleep(Duration::from_millis(5));
                    status = self.child.try_wait().ok().flatten();
                }
                if status.is_some() {
                    self.finish_stderr();
                }
                panic!(
                    "{} ended without printing the expected line (exit status: \
                     {status:?}). Its stderr:\n{}",
                    self.name,
                    self.stderr()
                )
            }
        }
    }

    /// Waits for the process to exit and returns its exit code (`None` if a
    /// signal ended it). Kills it and panics after `secs` seconds.
    #[track_caller]
    pub fn wait(&mut self, secs: u64) -> Option<i32> {
        let give_up = Instant::now() + Duration::from_secs(secs);
        loop {
            match self.child.try_wait() {
                Ok(Some(status)) => {
                    self.finish_stderr();
                    return status.code();
                }
                Ok(None) if Instant::now() < give_up => {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Ok(None) => {
                    let _ = self.child.kill();
                    let _ = self.child.wait();
                    panic!(
                        "{} was still running after {secs} s and was killed. Its stderr:\n{}",
                        self.name,
                        self.stderr()
                    );
                }
                Err(err) => panic!("waiting for {} failed: {err}", self.name),
            }
        }
    }

    /// Kills the process at once, like a power cut: no chance to clean up.
    pub fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.finish_stderr();
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        self.kill();
    }
}

/// A database file with a unique name in the temporary directory; removed on drop.
pub struct TempDb(PathBuf);

impl TempDb {
    pub fn new(label: &str) -> TempDb {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let number = NEXT.fetch_add(1, Ordering::Relaxed);
        let name = format!("beacon-{}-{number}-{label}.db", std::process::id());
        let db = TempDb(std::env::temp_dir().join(name));
        db.remove();
        db
    }

    /// The value for `BEACON_DB`.
    pub fn url(&self) -> String {
        format!("sqlite://{}", self.0.display())
    }

    fn remove(&self) {
        for suffix in ["", "-wal", "-shm", "-journal"] {
            let mut name = self.0.clone().into_os_string();
            name.push(suffix);
            let _ = std::fs::remove_file(name);
        }
    }
}

impl Drop for TempDb {
    fn drop(&mut self) {
        self.remove();
    }
}

/// An HTTP response, fully read.
#[derive(Debug, Clone)]
pub struct Reply {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl Reply {
    #[track_caller]
    pub fn json(&self) -> serde_json::Value {
        serde_json::from_str(&self.body).unwrap_or_else(|err| {
            panic!(
                "the body is not JSON ({err}); status {}, body {:?}",
                self.status, self.body
            )
        })
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

/// One HTTP/1.1 request on its own connection. `body` is sent as JSON.
#[track_caller]
pub fn http(
    addr: &str,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: Option<&str>,
) -> Reply {
    let target: SocketAddr = addr
        .parse()
        .unwrap_or_else(|_| panic!("{addr:?} is not an address"));
    let mut stream = TcpStream::connect_timeout(&target, Duration::from_secs(10))
        .unwrap_or_else(|err| panic!("connecting to {addr} failed: {err}"));
    stream
        .set_read_timeout(Some(Duration::from_secs(20)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(20)))
        .unwrap();
    let mut request = format!("{method} {path} HTTP/1.1\r\nhost: {addr}\r\nconnection: close\r\n");
    for (name, value) in headers {
        request.push_str(&format!("{name}: {value}\r\n"));
    }
    if let Some(body) = body {
        request.push_str(&format!(
            "content-type: application/json\r\ncontent-length: {}\r\n",
            body.len()
        ));
    }
    request.push_str("\r\n");
    request.push_str(body.unwrap_or(""));
    stream
        .write_all(request.as_bytes())
        .unwrap_or_else(|err| panic!("sending {method} {path} failed: {err}"));
    let mut raw = Vec::new();
    stream
        .read_to_end(&mut raw)
        .unwrap_or_else(|err| panic!("reading the response to {method} {path} failed: {err}"));
    let text = String::from_utf8_lossy(&raw).into_owned();
    let (head, rest) = text
        .split_once("\r\n\r\n")
        .unwrap_or_else(|| panic!("not an HTTP response to {method} {path}: {text:?}"));
    let mut lines = head.lines();
    let status = lines
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("no status line in the response to {method} {path}: {head:?}"));
    let headers: Vec<(String, String)> = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_owned(), value.trim().to_owned()))
        .collect();
    let chunked = headers.iter().any(|(name, value)| {
        name.eq_ignore_ascii_case("transfer-encoding") && value.contains("chunked")
    });
    let body = if chunked {
        unchunk(rest)
    } else {
        rest.to_owned()
    };
    Reply {
        status,
        headers,
        body,
    }
}

fn unchunk(mut rest: &str) -> String {
    let mut body = String::new();
    while let Some((size_line, after)) = rest.split_once("\r\n") {
        let size = usize::from_str_radix(size_line.trim(), 16).unwrap_or(0);
        if size == 0 || after.len() < size {
            break;
        }
        body.push_str(&after[..size]);
        rest = after[size..].trim_start_matches("\r\n");
    }
    body
}

/// What a read on a protocol connection produced.
#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    Message(Message),
    /// The other side closed the connection (cleanly or not) or sent bytes
    /// that are not a frame.
    Closed,
    /// Nothing arrived in the given time.
    Quiet,
}

/// One `beacon_wire` connection, from either side, with blocking I/O.
pub struct Peer {
    stream: TcpStream,
    buf: Vec<u8>,
}

impl Peer {
    pub fn new(stream: TcpStream) -> Peer {
        stream.set_nonblocking(false).expect("blocking mode");
        stream
            .set_write_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        Peer {
            stream,
            buf: Vec::new(),
        }
    }

    #[track_caller]
    pub fn connect(addr: &str) -> Peer {
        let target: SocketAddr = addr
            .parse()
            .unwrap_or_else(|_| panic!("{addr:?} is not an address"));
        let stream = TcpStream::connect_timeout(&target, Duration::from_secs(10))
            .unwrap_or_else(|err| panic!("connecting to {addr} failed: {err}"));
        Peer::new(stream)
    }

    /// Sends one frame. Returns `false` if the connection is gone.
    pub fn send(&mut self, message: &Message) -> bool {
        let mut frame = Vec::new();
        encode_frame(message, &mut frame).expect("encodable message");
        self.stream.write_all(&frame).is_ok()
    }

    /// Sends bytes that are not a frame.
    pub fn send_raw(&mut self, bytes: &[u8]) -> bool {
        self.stream.write_all(bytes).is_ok()
    }

    /// Waits up to `wait` for the next message.
    pub fn recv(&mut self, wait: Duration) -> Event {
        let give_up = Instant::now() + wait;
        loop {
            match decode_frame(&self.buf) {
                Ok(Some((message, used))) => {
                    self.buf.drain(..used);
                    return Event::Message(message);
                }
                Ok(None) => {}
                Err(_) => return Event::Closed,
            }
            let left = give_up.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Event::Quiet;
            }
            self.stream.set_read_timeout(Some(left)).unwrap();
            let mut chunk = [0u8; 4096];
            match self.stream.read(&mut chunk) {
                Ok(0) => return Event::Closed,
                Ok(count) => self.buf.extend_from_slice(&chunk[..count]),
                Err(err)
                    if matches!(
                        err.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    return Event::Quiet;
                }
                Err(_) => return Event::Closed,
            }
        }
    }
}

/// What a [`FakeIngest`] saw on one connection.
#[derive(Debug, Clone, Default)]
pub struct ConnectionLog {
    /// 0 for the first accepted connection, 1 for the next, ...
    pub index: usize,
    pub agent: Option<String>,
    /// Values of samples in batches that this connection acknowledged.
    pub acked: Vec<i64>,
    /// Values of samples in batches that were read but not acknowledged.
    pub unacked: Vec<i64>,
    /// Sample times, in the order received, of all batches on this connection.
    pub times: Vec<u64>,
}

struct FakeShared {
    acking: AtomicBool,
    drop_first_connection_after_one_batch: bool,
    stop: AtomicBool,
    logs: Mutex<Vec<ConnectionLog>>,
}

/// A stand-in for `beacon-server`'s ingest port, run by the test, so that the
/// agent can be tested alone and the "server" can misbehave on purpose.
///
/// - While `acking` is off, it reads everything and acknowledges nothing.
/// - When `acking` is switched on, connections opened before the switch are
///   closed; connections opened afterwards get an `Ack` for every batch.
/// - With `drop_first_connection_after_one_batch`, the first connection is
///   closed right after its first batch, without an `Ack`.
pub struct FakeIngest {
    pub addr: String,
    shared: Arc<FakeShared>,
    acceptor: Option<JoinHandle<()>>,
}

impl FakeIngest {
    pub fn start(acking: bool, drop_first_connection_after_one_batch: bool) -> FakeIngest {
        let listener = TcpListener::bind("127.0.0.1:0").expect("binding a loopback port");
        let addr = listener.local_addr().unwrap().to_string();
        listener.set_nonblocking(true).unwrap();
        let shared = Arc::new(FakeShared {
            acking: AtomicBool::new(acking),
            drop_first_connection_after_one_batch,
            stop: AtomicBool::new(false),
            logs: Mutex::new(Vec::new()),
        });
        let for_thread = Arc::clone(&shared);
        let acceptor = std::thread::spawn(move || {
            let mut connections = Vec::new();
            let mut index = 0;
            while !for_thread.stop.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let shared = Arc::clone(&for_thread);
                        let this = index;
                        index += 1;
                        connections.push(std::thread::spawn(move || {
                            serve_fake(Peer::new(stream), this, &shared);
                        }));
                    }
                    Err(_) => std::thread::sleep(Duration::from_millis(2)),
                }
            }
            for connection in connections {
                let _ = connection.join();
            }
        });
        FakeIngest {
            addr,
            shared,
            acceptor: Some(acceptor),
        }
    }

    /// From now on, acknowledge batches (on new connections).
    pub fn start_acking(&self) {
        self.shared.acking.store(true, Ordering::SeqCst);
    }

    /// Stops the fake server and returns what each connection saw, by index.
    pub fn finish(mut self) -> Vec<ConnectionLog> {
        self.shared.stop.store(true, Ordering::SeqCst);
        if let Some(acceptor) = self.acceptor.take() {
            let _ = acceptor.join();
        }
        let mut logs = std::mem::take(&mut *self.shared.logs.lock().unwrap());
        logs.sort_by_key(|log| log.index);
        logs
    }
}

impl Drop for FakeIngest {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        if let Some(acceptor) = self.acceptor.take() {
            let _ = acceptor.join();
        }
    }
}

fn serve_fake(mut peer: Peer, index: usize, shared: &FakeShared) {
    let opened_acking = shared.acking.load(Ordering::SeqCst);
    let mut log = ConnectionLog {
        index,
        ..ConnectionLog::default()
    };
    loop {
        if shared.stop.load(Ordering::SeqCst) {
            break;
        }
        if !opened_acking && shared.acking.load(Ordering::SeqCst) {
            break; // the mode changed: drop this connection
        }
        match peer.recv(Duration::from_millis(20)) {
            Event::Quiet => continue,
            Event::Closed => break,
            Event::Message(Message::Hello { agent }) => log.agent = Some(agent),
            Event::Message(Message::Batch { seq, samples, .. }) => {
                log.times.extend(samples.iter().map(|sample| sample.ts_ms));
                let values = samples.iter().map(|sample| sample.value);
                if index == 0 && shared.drop_first_connection_after_one_batch {
                    log.unacked.extend(values);
                    break;
                }
                if opened_acking && peer.send(&Message::Ack { seq }) {
                    log.acked.extend(values);
                } else {
                    log.unacked.extend(values);
                }
            }
            Event::Message(Message::Ack { .. }) => break,
        }
    }
    shared.logs.lock().unwrap().push(log);
}
