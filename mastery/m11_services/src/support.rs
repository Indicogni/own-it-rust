//! Support code for the lessons. You do not need to edit this file.
//!
//! - [`guess`] marks a prediction you have not written yet.
//! - [`within`] turns a test that would hang into a test that fails with a message.
//! - [`send`] calls a router in memory, without a socket.
//! - [`TestServer`] and [`http`] run a router on a loopback socket and call it.
//! - [`Capture`] collects log output so that a test can read it.
//! - [`TempDb`] is a database file that is deleted when the test ends.
//! - [`CountingDb`] is a connection pool that counts the statements sent through it.
//! - [`ChildGuard`] runs another program and never leaves it behind.
//!
//! Lesson 09 asks you to write helpers like these yourself.

use axum::Router;
use axum::body::{Body, Bytes};
use axum::http::{HeaderMap, Method, Request, StatusCode, header};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tower::ServiceExt;

/// Placeholder for a prediction. Replace the whole call (`crate::guess()` in a
/// lesson file) with your answer before running the lesson, for example
/// `let p1: u16 = 200;`.
#[track_caller]
pub fn guess<T>() -> T {
    panic!("PREDICT: replace `guess()` with your written prediction, then run again")
}

/// Runs `f` on a helper thread and returns its result, or fails the test with a
/// clear message if it has not finished after `secs` seconds.
///
/// `what` names the thing being run and appears in the failure message.
#[track_caller]
pub fn within<R: Send + 'static>(
    secs: u64,
    what: &str,
    f: impl FnOnce() -> R + Send + 'static,
) -> R {
    use std::sync::mpsc::{self, RecvTimeoutError};
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    match rx.recv_timeout(Duration::from_secs(secs)) {
        Ok(value) => value,
        Err(RecvTimeoutError::Timeout) => panic!(
            "{what} was still running after {secs} s. It is probably blocked forever: \
             look for a lock that is never released, a channel that is never closed, \
             or blocking work on the async runtime thread."
        ),
        Err(RecvTimeoutError::Disconnected) => {
            panic!("{what} panicked before finishing; see the panic message above")
        }
    }
}

/// Fails the test if `future` has not finished after `secs` seconds of tokio time.
///
/// With `start_paused = true` the seconds are virtual, so a hang is reported at once.
pub async fn deadline<F: Future>(secs: u64, what: &str, future: F) -> F::Output {
    match tokio::time::timeout(Duration::from_secs(secs), future).await {
        Ok(value) => value,
        Err(_) => panic!("{what} did not finish within {secs} s"),
    }
}

/// For tests with paused time (`start_paused = true`) that use only in-memory
/// I/O: waits until every other task is blocked. Tokio advances a paused clock
/// only when the runtime has nothing left to run, so this short sleep cannot
/// finish while another task can still make progress.
pub async fn settle() {
    tokio::time::sleep(Duration::from_millis(1)).await;
}

/// A fully read HTTP response.
#[derive(Debug, Clone)]
pub struct TestResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Bytes,
}

impl TestResponse {
    /// The numeric status code, for example 404.
    pub fn code(&self) -> u16 {
        self.status.as_u16()
    }

    /// The body as text. Panics if it is not UTF-8.
    #[track_caller]
    pub fn text(&self) -> String {
        String::from_utf8(self.body.to_vec()).expect("the response body is not UTF-8")
    }

    /// The body parsed as JSON. Panics with the raw body if it is not JSON.
    #[track_caller]
    pub fn json(&self) -> serde_json::Value {
        match serde_json::from_slice(&self.body) {
            Ok(value) => value,
            Err(err) => panic!(
                "the response body is not JSON ({err}); status {}, body: {:?}",
                self.status,
                String::from_utf8_lossy(&self.body)
            ),
        }
    }

    /// One header as text, or `None` if it is missing.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|value| value.to_str().ok())
    }
}

/// Builds a request without a body.
pub fn request(method: Method, uri: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .body(Body::empty())
        .expect("valid request")
}

/// Builds a `GET` request.
pub fn get(uri: &str) -> Request<Body> {
    request(Method::GET, uri)
}

/// Builds a request with a JSON body and `content-type: application/json`.
pub fn json_request(method: Method, uri: &str, body: &serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("valid request")
}

/// Builds a `POST` request with a JSON body.
pub fn post_json(uri: &str, body: &serde_json::Value) -> Request<Body> {
    json_request(Method::POST, uri, body)
}

/// Adds one header to a request.
pub fn with_header(mut request: Request<Body>, name: &'static str, value: &str) -> Request<Body> {
    request
        .headers_mut()
        .insert(name, value.parse().expect("valid header value"));
    request
}

/// Calls the router in memory: no socket, no HTTP parsing. The router is cloned,
/// so the caller keeps its own handle; shared state behind `Arc` stays shared.
pub async fn send(app: &Router, request: Request<Body>) -> TestResponse {
    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("a Router never returns an error");
    let (parts, body) = response.into_parts();
    let body = axum::body::to_bytes(body, usize::MAX)
        .await
        .expect("reading the response body failed");
    TestResponse {
        status: parts.status,
        headers: parts.headers,
        body,
    }
}

/// Sends a request over a real TCP connection and reads the whole response.
/// The request URI must be absolute; see [`TestServer::url`].
/// Each call opens a new connection, so no idle connection outlives the call.
pub async fn http(request: Request<Body>) -> Result<TestResponse, String> {
    use hyper_util::client::legacy::Client;
    use hyper_util::rt::TokioExecutor;
    let client: Client<_, Body> = Client::builder(TokioExecutor::new())
        .pool_max_idle_per_host(0)
        .build_http();
    let response = client
        .request(request)
        .await
        .map_err(|err| format!("request failed: {err:?}"))?;
    let (parts, body) = response.into_parts();
    let body = axum::body::to_bytes(Body::new(body), usize::MAX)
        .await
        .map_err(|err| format!("reading the body failed: {err}"))?;
    Ok(TestResponse {
        status: parts.status,
        headers: parts.headers,
        body,
    })
}

/// A router served on `127.0.0.1` with a port chosen by the operating system.
/// Dropping the value stops the server task.
pub struct TestServer {
    pub addr: SocketAddr,
    stop: Option<oneshot::Sender<()>>,
    task: JoinHandle<()>,
}

impl TestServer {
    /// Binds `127.0.0.1:0` and serves `app` on a background task.
    pub async fn start(app: Router) -> TestServer {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("binding a loopback port failed");
        let addr = listener.local_addr().expect("listener has an address");
        let (stop, stopped) = oneshot::channel::<()>();
        let task = tokio::spawn(async move {
            let _ = axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = stopped.await;
                })
                .await;
        });
        TestServer {
            addr,
            stop: Some(stop),
            task,
        }
    }

    /// `http://127.0.0.1:PORT` followed by `path`.
    pub fn url(&self, path: &str) -> String {
        format!("http://{}{path}", self.addr)
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        self.task.abort();
    }
}

/// A log sink for tests. Clones share one buffer.
/// (The ecosystem track, lesson E04, builds this type step by step.)
#[derive(Clone, Default)]
pub struct Capture(Arc<Mutex<Vec<u8>>>);

impl Capture {
    /// Everything written so far, as text.
    pub fn contents(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().expect("capture lock")).into_owned()
    }

    /// The captured lines, without the trailing newline characters.
    pub fn lines(&self) -> Vec<String> {
        self.contents().lines().map(str::to_owned).collect()
    }
}

impl std::io::Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .expect("capture lock")
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Capture {
    type Writer = Capture;
    fn make_writer(&'a self) -> Capture {
        self.clone()
    }
}

/// A `tracing` subscriber that writes plain lines (no colours, no timestamps,
/// no targets) at `INFO` and above into `capture`.
pub fn plain_subscriber(capture: &Capture) -> impl tracing::Subscriber + Send + Sync + 'static {
    tracing_subscriber::fmt()
        .with_writer(capture.clone())
        .with_ansi(false)
        .without_time()
        .with_target(false)
        .with_max_level(tracing::Level::INFO)
        .finish()
}

/// A database file in the system's temporary directory, with a unique name.
/// The file and SQLite's side files are deleted when the value is dropped.
#[derive(Debug)]
pub struct TempDb {
    path: std::path::PathBuf,
}

impl TempDb {
    /// Picks a fresh path; `label` only makes the file name readable.
    pub fn new(label: &str) -> TempDb {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let number = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let name = format!("m11-{}-{number}-{label}.db", std::process::id());
        let path = std::env::temp_dir().join(name);
        let db = TempDb { path };
        db.remove_files();
        db
    }

    /// The path of the database file.
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// A connection URL that sqlx accepts: `sqlite://` followed by the path.
    pub fn url(&self) -> String {
        format!("sqlite://{}", self.path.display())
    }

    fn remove_files(&self) {
        for suffix in ["", "-wal", "-shm", "-journal"] {
            let mut name = self.path.clone().into_os_string();
            name.push(suffix);
            let _ = std::fs::remove_file(name);
        }
    }
}

impl Drop for TempDb {
    fn drop(&mut self) {
        self.remove_files();
    }
}

/// A pool wrapper that counts the statements executed through it.
///
/// `&CountingDb` can be used wherever sqlx expects an executor:
/// `sqlx::query("...").fetch_all(&db)`. Lesson 03 uses the count to detect the
/// "N+1 queries" pattern.
#[derive(Debug)]
pub struct CountingDb {
    pool: sqlx::SqlitePool,
    statements: std::sync::atomic::AtomicUsize,
}

impl CountingDb {
    pub fn new(pool: sqlx::SqlitePool) -> CountingDb {
        CountingDb {
            pool,
            statements: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Statements executed through this wrapper so far.
    pub fn statements(&self) -> usize {
        self.statements.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// The wrapped pool. Statements sent to it directly are not counted.
    pub fn pool(&self) -> &sqlx::SqlitePool {
        &self.pool
    }

    fn count(&self) {
        self.statements
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

type SqliteStep = sqlx::Either<sqlx::sqlite::SqliteQueryResult, sqlx::sqlite::SqliteRow>;

impl<'c> sqlx::Executor<'c> for &'c CountingDb {
    type Database = sqlx::Sqlite;

    fn fetch_many<'e, 'q: 'e, E>(
        self,
        query: E,
    ) -> futures_core::stream::BoxStream<'e, Result<SqliteStep, sqlx::Error>>
    where
        'c: 'e,
        E: 'q + sqlx::Execute<'q, sqlx::Sqlite>,
    {
        self.count();
        self.pool.fetch_many(query)
    }

    fn fetch_optional<'e, 'q: 'e, E>(
        self,
        query: E,
    ) -> futures_core::future::BoxFuture<'e, Result<Option<sqlx::sqlite::SqliteRow>, sqlx::Error>>
    where
        'c: 'e,
        E: 'q + sqlx::Execute<'q, sqlx::Sqlite>,
    {
        self.count();
        self.pool.fetch_optional(query)
    }

    fn prepare_with<'e>(
        self,
        sql: sqlx::SqlStr,
        parameters: &'e [sqlx::sqlite::SqliteTypeInfo],
    ) -> futures_core::future::BoxFuture<'e, Result<sqlx::sqlite::SqliteStatement, sqlx::Error>>
    where
        'c: 'e,
    {
        self.pool.prepare_with(sql, parameters)
    }

    fn describe<'e>(
        self,
        sql: sqlx::SqlStr,
    ) -> futures_core::future::BoxFuture<'e, Result<sqlx::Describe<sqlx::Sqlite>, sqlx::Error>>
    where
        'c: 'e,
    {
        self.pool.describe(sql)
    }
}

/// A child process for tests. Its standard output is read line by line, its
/// standard error is collected for failure messages, and the process is killed
/// and reaped when the guard is dropped, also when the test panics.
pub struct ChildGuard {
    name: String,
    child: std::process::Child,
    stdout: std::sync::mpsc::Receiver<String>,
    stderr: Arc<Mutex<String>>,
    stderr_reader: Option<std::thread::JoinHandle<()>>,
}

impl ChildGuard {
    /// Starts `command` with piped output. `name` appears in failure messages.
    #[track_caller]
    pub fn spawn(name: &str, command: &mut std::process::Command) -> ChildGuard {
        use std::io::{BufRead, BufReader, Read};
        use std::process::Stdio;
        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|err| panic!("starting {name} failed: {err}"));
        let (line_tx, stdout) = std::sync::mpsc::channel();
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

    /// The next line of standard output. Panics, with the collected standard
    /// error, if none arrives within `secs` seconds or the output has ended.
    #[track_caller]
    pub fn line(&mut self, secs: u64) -> String {
        use std::sync::mpsc::RecvTimeoutError;
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
                let give_up = std::time::Instant::now() + Duration::from_secs(2);
                let mut status = self.child.try_wait().ok().flatten();
                while status.is_none() && std::time::Instant::now() < give_up {
                    std::thread::sleep(Duration::from_millis(5));
                    status = self.child.try_wait().ok().flatten();
                }
                if status.is_some() {
                    self.finish_stderr();
                }
                panic!(
                    "{} closed its output without printing the expected line \
                     (exit status: {status:?}). Its stderr:\n{}",
                    self.name,
                    self.stderr()
                )
            }
        }
    }

    /// Waits up to `secs` seconds for the process to exit and returns its exit
    /// code (`None` if a signal ended it). Kills it and panics on timeout.
    #[track_caller]
    pub fn wait(&mut self, secs: u64) -> Option<i32> {
        let give_up = std::time::Instant::now() + Duration::from_secs(secs);
        loop {
            match self.child.try_wait() {
                Ok(Some(status)) => {
                    self.finish_stderr();
                    return status.code();
                }
                Ok(None) if std::time::Instant::now() < give_up => {
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

    /// Kills the process now (no chance to clean up) and reaps it.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn within_returns_the_value() {
        assert_eq!(within(5, "addition", || 2 + 2), 4);
    }

    #[test]
    #[should_panic(expected = "still running after 1 s")]
    fn within_reports_a_hang() {
        within(1, "sleeper", || std::thread::sleep(Duration::from_secs(30)));
    }

    #[tokio::test]
    async fn send_and_http_agree() {
        let app = Router::new().route("/ping", axum::routing::get(|| async { "pong" }));
        let in_memory = send(&app, get("/ping")).await;
        assert_eq!((in_memory.code(), in_memory.text().as_str()), (200, "pong"));
        let server = TestServer::start(app).await;
        let over_tcp = http(get(&server.url("/ping"))).await.unwrap();
        assert_eq!((over_tcp.code(), over_tcp.text().as_str()), (200, "pong"));
    }
}
