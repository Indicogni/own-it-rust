//! LESSON 08 — Observability and operations: config, metrics, logs, health, shutdown
//! Read first: CONCEPTS.md, "Lesson 08". Start after: lessons 02 and 03, ecosystem E04.
//! Run: cargo test --manifest-path mastery/m11_services/Cargo.toml l08_
//!      cargo test --manifest-path mastery/m11_services/Cargo.toml --test l08_tracing
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: your service gets the signal to
//! stop while three requests are in progress and a load balancer still sends new
//! ones. Put these in the right order: stop accepting connections, report "not
//! ready", finish the three requests, exit.
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! Task D is checked by `tests/l08_tracing.rs`, a test binary of its own, because
//! tracing caches per call site whether anyone listens, and that cache is shared
//! by all tests in one process. The container LAB is in `lab/README.md`; nothing
//! in the tests needs Docker.
//! EXPLAIN afterward: What question does `/healthz` answer and what question does
//! `/readyz` answer, and who asks each? Why must the `route` label come from the
//! route template and not from the request path? Why does a span entered with
//! `enter()` and held across `.await` produce wrong logs?
//! JUDGMENT: This lesson writes the metrics registry by hand so that you see the
//! exposition format. In production use the `metrics` facade with
//! `metrics-exporter-prometheus`, or `prometheus-client`, or OpenTelemetry when
//! your organisation has a collector. For request logging, `tower_http::trace::
//! TraceLayer` is the ready-made version of task D. Configuration here is a file
//! plus environment variables, merged by hand; the `config` crate does the same
//! with more sources. Whatever you use: validate at start-up, fail fast, and never
//! print secrets.

use axum::extract::{MatchedPath, Request, State};
use axum::http::{StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use sqlx::SqlitePool;
use std::collections::BTreeMap;
use std::future::{Future, IntoFuture};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tracing::Instrument;

// ---------------------------------------------------------------------------
// A. Configuration
// ---------------------------------------------------------------------------

/// A value that must not appear in logs or error messages. Supplied.
#[derive(Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: &str) -> Secret {
        Secret(value.to_owned())
    }
    /// The only way to read the value; easy to find in code review.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret(<redacted>)")
    }
}

/// The validated configuration of the service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub listen: SocketAddr,
    pub database_url: String,
    pub drain_seconds: u64,
    pub api_token: Secret,
}

/// Why the configuration could not be loaded.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    /// The file is not valid TOML, has a key we do not know, or a value of the
    /// wrong type.
    #[error("configuration file: {0}")]
    File(String),
    /// An environment variable starts with `APP_` but is not one of ours:
    /// probably a typo.
    #[error("unknown environment variable {0}")]
    UnknownVariable(String),
    /// A setting has no value and no default.
    #[error("{0} is required")]
    Missing(&'static str),
    /// A setting has a value that is not acceptable. `key` is the TOML key.
    #[error("{key}: {problem}")]
    Invalid { key: &'static str, problem: String },
}

/// A. Build the configuration from an optional TOML file and the environment.
///
/// | TOML key        | Environment variable | Default            | Rule                         |
/// | --------------- | -------------------- | ------------------ | ---------------------------- |
/// | `listen`        | `APP_LISTEN`         | `127.0.0.1:8080`   | a socket address             |
/// | `database_url`  | `APP_DATABASE_URL`   | `sqlite::memory:`  | not empty                    |
/// | `drain_seconds` | `APP_DRAIN_SECONDS`  | `10`               | a whole number from 1 to 300 |
/// | `api_token`     | `APP_API_TOKEN`      | none: required     | at least 16 characters       |
///
/// - Precedence: environment over file over default.
/// - `file` is the text of the file, or `None` if there is no file. A key that is
///   not in the table is `File(..)`: a typo must not pass silently.
/// - `env` is the process environment as pairs. Variables without the `APP_`
///   prefix are not ours and are ignored. An `APP_` variable that is not in the
///   table is `UnknownVariable(name)`.
/// - Values that break a rule are `Invalid` with the TOML key, wherever they came
///   from. An empty value is a value: `APP_DRAIN_SECONDS=` is `Invalid`, it does
///   not mean "use the default".
/// - No error may contain the value of `api_token`, in `Display` or in `Debug`.
///   Careful with the TOML parser: its `Display` output quotes the offending
///   line of the file, and its messages can quote a value of the wrong type.
///
/// Taking the environment as an argument keeps this function a pure function,
/// so its tests need not change the real environment (which is `unsafe` in
/// edition 2024, because other threads may be reading it).
pub fn load_config(
    file: Option<&str>,
    env: impl IntoIterator<Item = (String, String)>,
) -> Result<Config, ConfigError> {
    /// Every setting as text exactly as written, before validation.
    #[derive(Default)]
    struct Raw {
        listen: Option<String>,
        database_url: Option<String>,
        drain_seconds: Option<String>,
        api_token: Option<Secret>,
    }

    let mut raw = Raw::default();
    if let Some(text) = file {
        // The parser's own messages can repeat file contents (the offending
        // line, or a value of the wrong type), so none of its text is passed
        // on: only the line number.
        let table: toml::Table = toml::from_str(text).map_err(|err| {
            let line = err.span().map_or(1, |span| {
                text[..span.start.min(text.len())].matches('\n').count() + 1
            });
            ConfigError::File(format!("not valid TOML near line {line}"))
        })?;
        for (key, value) in table {
            use toml::Value;
            match (key.as_str(), value) {
                ("listen", Value::String(text)) => raw.listen = Some(text),
                ("database_url", Value::String(text)) => raw.database_url = Some(text),
                ("drain_seconds", Value::Integer(number)) => {
                    raw.drain_seconds = Some(number.to_string());
                }
                ("api_token", Value::String(text)) => raw.api_token = Some(Secret(text)),
                ("listen" | "database_url" | "drain_seconds" | "api_token", _) => {
                    return Err(ConfigError::File(format!("{key} has the wrong type")));
                }
                _ => return Err(ConfigError::File(format!("unknown key {key}"))),
            }
        }
    }
    for (name, value) in env {
        let Some(setting) = name.strip_prefix("APP_") else {
            continue;
        };
        match setting {
            "LISTEN" => raw.listen = Some(value),
            "DATABASE_URL" => raw.database_url = Some(value),
            "DRAIN_SECONDS" => raw.drain_seconds = Some(value),
            "API_TOKEN" => raw.api_token = Some(Secret(value)),
            _ => return Err(ConfigError::UnknownVariable(name)),
        }
    }

    let invalid = |key: &'static str, problem: &str| ConfigError::Invalid {
        key,
        problem: problem.to_owned(),
    };
    let listen = raw
        .listen
        .unwrap_or_else(|| "127.0.0.1:8080".to_owned())
        .parse()
        .map_err(|_| invalid("listen", "must be an address such as 127.0.0.1:8080"))?;
    let database_url = raw
        .database_url
        .unwrap_or_else(|| "sqlite::memory:".to_owned());
    if database_url.trim().is_empty() {
        return Err(invalid("database_url", "must not be empty"));
    }
    let drain_seconds = match raw.drain_seconds {
        None => 10,
        Some(text) => match text.trim().parse::<u64>() {
            Ok(seconds) if (1..=300).contains(&seconds) => seconds,
            _ => {
                return Err(invalid(
                    "drain_seconds",
                    "must be a whole number from 1 to 300",
                ));
            }
        },
    };
    let api_token = raw.api_token.ok_or(ConfigError::Missing("api_token"))?;
    if api_token.expose().chars().count() < 16 {
        return Err(invalid("api_token", "must have at least 16 characters"));
    }
    Ok(Config {
        listen,
        database_url,
        drain_seconds,
        api_token,
    })
}

// ---------------------------------------------------------------------------
// B and C. Metrics
// ---------------------------------------------------------------------------

/// Upper bounds of the latency histogram buckets, in milliseconds. Their `le`
/// labels in the output are `0.01`, `0.1` and `1` (seconds).
pub const BUCKET_BOUNDS_MS: [u64; 3] = [10, 100, 1000];

/// B. A small metrics registry: one counter family and one histogram.
/// Clones share the same numbers. You may change the private fields.
#[derive(Debug, Clone, Default)]
pub struct Metrics {
    inner: Arc<Mutex<MetricsInner>>,
}

#[derive(Debug, Default)]
struct MetricsInner {
    /// (method, route, status) → number of requests.
    requests: BTreeMap<(String, String, u16), u64>,
    /// Observations per bucket, NOT cumulative; the last entry is "above all bounds".
    buckets: [u64; BUCKET_BOUNDS_MS.len() + 1],
    sum_ms: u64,
}

impl Metrics {
    /// B. Count one finished request and record how long it took.
    pub fn record(&self, method: &str, route: &str, status: u16, elapsed: Duration) {
        let mut inner = self.inner.lock().expect("metrics lock");
        *inner
            .requests
            .entry((method.to_owned(), route.to_owned(), status))
            .or_insert(0) += 1;
        let millis = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);
        let bucket = BUCKET_BOUNDS_MS
            .iter()
            .position(|bound| millis <= *bound)
            .unwrap_or(BUCKET_BOUNDS_MS.len());
        inner.buckets[bucket] += 1;
        inner.sum_ms = inner.sum_ms.saturating_add(millis);
    }

    /// B. The Prometheus text exposition format, exactly like this example (two
    /// requests of 5 ms and 2500 ms to one route, one of 50 ms to another):
    ///
    /// ```text
    /// # HELP http_requests_total Requests handled, by method, route and status.
    /// # TYPE http_requests_total counter
    /// http_requests_total{method="GET",route="/items/{id}",status="200"} 2
    /// http_requests_total{method="POST",route="/items",status="201"} 1
    /// # HELP http_request_duration_seconds Time from request to response.
    /// # TYPE http_request_duration_seconds histogram
    /// http_request_duration_seconds_bucket{le="0.01"} 1
    /// http_request_duration_seconds_bucket{le="0.1"} 2
    /// http_request_duration_seconds_bucket{le="1"} 2
    /// http_request_duration_seconds_bucket{le="+Inf"} 3
    /// http_request_duration_seconds_sum 2.555
    /// http_request_duration_seconds_count 3
    /// ```
    ///
    /// - Counter lines are sorted by method, then route, then status. With no
    ///   requests there are no counter lines, but the HELP and TYPE lines remain.
    /// - Histogram buckets are **cumulative**: each counts every observation that
    ///   is less than or equal to its bound, so `+Inf` equals `_count`. Durations
    ///   are counted in whole milliseconds; the sum has exactly three decimals.
    /// - Inside a label value, write `\` as `\\`, `"` as `\"` and a line feed as
    ///   `\n`.
    /// - Every line, including the last, ends with `\n`.
    pub fn render(&self) -> String {
        use std::fmt::Write;
        fn escape(value: &str) -> String {
            value
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
        }
        let inner = self.inner.lock().expect("metrics lock");
        let mut out = String::new();
        out.push_str("# HELP http_requests_total Requests handled, by method, route and status.\n");
        out.push_str("# TYPE http_requests_total counter\n");
        for ((method, route, status), count) in &inner.requests {
            let _ = writeln!(
                out,
                "http_requests_total{{method=\"{}\",route=\"{}\",status=\"{status}\"}} {count}",
                escape(method),
                escape(route),
            );
        }
        out.push_str("# HELP http_request_duration_seconds Time from request to response.\n");
        out.push_str("# TYPE http_request_duration_seconds histogram\n");
        let mut running = 0;
        for (label, count) in ["0.01", "0.1", "1", "+Inf"].iter().zip(inner.buckets) {
            running += count;
            let _ = writeln!(
                out,
                "http_request_duration_seconds_bucket{{le=\"{label}\"}} {running}"
            );
        }
        let _ = writeln!(
            out,
            "http_request_duration_seconds_sum {}.{:03}",
            inner.sum_ms / 1000,
            inner.sum_ms % 1000
        );
        let _ = writeln!(out, "http_request_duration_seconds_count {running}");
        out
    }
}

/// C. Middleware for `axum::middleware::from_fn_with_state`: record every request
/// that passes, timed with `tokio::time::Instant`.
///
/// The `route` label is the **route template** that matched, for example
/// `/items/{id}`, which axum stores in the request extensions as [`MatchedPath`].
/// It is not the path the client sent: with real paths, every id becomes a new
/// time series and the metrics system runs out of memory. A request that matched
/// no route gets the label `unmatched`.
pub async fn track_metrics(
    State(metrics): State<Metrics>,
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().as_str().to_owned();
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map_or_else(|| "unmatched".to_owned(), |path| path.as_str().to_owned());
    let start = tokio::time::Instant::now();
    let response = next.run(request).await;
    metrics.record(&method, &route, response.status().as_u16(), start.elapsed());
    response
}

/// C. Wrap `routes` so that every request to them (matched or not) is recorded in
/// `metrics`, and add `GET /metrics`, which answers 200 with
/// `content-type: text/plain; version=0.0.4` and `metrics.render()`. Requests to
/// `/metrics` itself are not recorded.
pub fn with_metrics(routes: Router, metrics: Metrics) -> Router {
    let exposition = metrics.clone();
    routes
        .layer(axum::middleware::from_fn_with_state(metrics, track_metrics))
        // Added after `.layer`, so the middleware does not wrap this route.
        .route(
            "/metrics",
            get(move || async move {
                (
                    [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
                    exposition.render(),
                )
            }),
        )
}

// ---------------------------------------------------------------------------
// D. Request spans
// ---------------------------------------------------------------------------

/// D. Middleware for `axum::middleware::from_fn`: run the rest of the stack inside
/// a span, and log one line when the response is ready.
///
/// - The span is named `request`, at level INFO, with the fields `method`, `path`
///   and `request_id`, all recorded with `Display` (`%`). `path` is the URI path
///   **without** the query string (queries often carry tokens). `request_id` is
///   the `x-request-id` header, or `-` if there is none.
/// - Every event that a handler emits must appear inside that span, also after
///   the handler has waited at an `.await`, and also when other requests run in
///   between on the same thread. Attach the span to the future
///   (`tracing::Instrument`); do not hold an `enter()` guard across `.await`.
/// - When the response is ready, emit `tracing::info!` inside the span with the
///   message `request finished` and the field `status` (the numeric code).
pub async fn trace_requests(request: Request, next: Next) -> Response {
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("-")
        .to_owned();
    let span = tracing::info_span!(
        "request",
        method = %request.method(),
        path = %request.uri().path(),
        request_id = %request_id,
    );
    async move {
        let response = next.run(request).await;
        tracing::info!(status = response.status().as_u16(), "request finished");
        response
    }
    // The span is entered each time this future is polled and left when the
    // poll returns, so it never leaks into another task.
    .instrument(span)
    .await
}

// ---------------------------------------------------------------------------
// E. Health and readiness
// ---------------------------------------------------------------------------

/// Where the service is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Started, but not yet able to serve (migrations, cache warm-up, ...).
    Starting,
    /// Serving normally.
    Ready,
    /// Asked to stop: finishing requests, taking no new work.
    Draining,
}

/// The current [`Phase`], shared between the server and its endpoints. Supplied.
#[derive(Debug, Clone)]
pub struct Lifecycle(Arc<AtomicU8>);

impl Lifecycle {
    /// A new lifecycle in phase `Starting`.
    pub fn new() -> Lifecycle {
        Lifecycle(Arc::new(AtomicU8::new(0)))
    }
    pub fn set(&self, phase: Phase) {
        let number = match phase {
            Phase::Starting => 0,
            Phase::Ready => 1,
            Phase::Draining => 2,
        };
        self.0.store(number, Ordering::SeqCst);
    }
    pub fn phase(&self) -> Phase {
        match self.0.load(Ordering::SeqCst) {
            0 => Phase::Starting,
            1 => Phase::Ready,
            _ => Phase::Draining,
        }
    }
}

impl Default for Lifecycle {
    fn default() -> Lifecycle {
        Lifecycle::new()
    }
}

/// E. Two endpoints for whatever runs your service (an orchestrator, a load
/// balancer, a person with curl):
///
/// - `GET /healthz` (liveness): 200 with the body `ok` whenever the process can
///   answer at all. It checks nothing else: a failed liveness check gets the
///   process restarted, and a restart does not repair a database that is down.
/// - `GET /readyz` (readiness): "should traffic be sent here right now?"
///   - phase `Starting`: 503 and `{"ready": false, "reason": "starting"}`
///   - phase `Draining`: 503 and `{"ready": false, "reason": "draining"}`
///   - phase `Ready`: run `SELECT 1` on `pool`. If it succeeds within
///     `check_timeout`: 200 and `{"ready": true}`. If it fails or takes longer:
///     503 and `{"ready": false, "reason": "database"}`.
///
///   The answer must arrive after about `check_timeout` at the latest, even when
///   the pool has no free connection; the pool's own acquire timeout may be much
///   longer.
pub fn health_routes(lifecycle: Lifecycle, pool: SqlitePool, check_timeout: Duration) -> Router {
    let not_ready = |reason: &'static str| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "ready": false, "reason": reason })),
        )
            .into_response()
    };
    let readyz = move || async move {
        match lifecycle.phase() {
            Phase::Starting => return not_ready("starting"),
            Phase::Draining => return not_ready("draining"),
            Phase::Ready => {}
        }
        let probe = sqlx::query("SELECT 1").execute(&pool);
        match tokio::time::timeout(check_timeout, probe).await {
            Ok(Ok(_)) => Json(serde_json::json!({ "ready": true })).into_response(),
            Ok(Err(_)) | Err(_) => not_ready("database"),
        }
    };
    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/readyz", get(readyz))
}

// ---------------------------------------------------------------------------
// F. Graceful shutdown
// ---------------------------------------------------------------------------

/// How a shutdown ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrainOutcome {
    /// Every in-flight request finished.
    Clean,
    /// Requests were still running when the drain time was over.
    TimedOut,
}

/// F. Serve `app` on `listener` until `shutdown` completes, then drain.
///
/// When `shutdown` completes:
/// 1. Set `lifecycle` to `Draining`, so `/readyz` tells the load balancer to stop
///    sending traffic.
/// 2. Stop accepting connections, and let requests that are already in progress
///    finish and send their responses (axum's graceful shutdown does both).
/// 3. Wait for that at most `drain`. Return `Clean` if all requests finished in
///    time and `TimedOut` if not. Do not wait longer than `drain`: whoever asked
///    you to stop will kill the process soon after.
///
/// The function does not set the phase to `Ready`; the caller does that when
/// start-up work is done.
pub async fn serve_until(
    listener: TcpListener,
    app: Router,
    lifecycle: Lifecycle,
    drain: Duration,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> std::io::Result<DrainOutcome> {
    let (draining_tx, draining_rx) = oneshot::channel::<()>();
    let signal = async move {
        shutdown.await;
        lifecycle.set(Phase::Draining);
        let _ = draining_tx.send(());
    };
    let server = axum::serve(listener, app)
        .with_graceful_shutdown(signal)
        .into_future();
    tokio::pin!(server);

    tokio::select! {
        // The server ended on its own (an accept error): nothing to drain.
        result = &mut server => return result.map(|()| DrainOutcome::Clean),
        _ = draining_rx => {}
    }
    // The drain clock starts when the signal arrives, not when the server started.
    match tokio::time::timeout(drain, &mut server).await {
        Ok(result) => result.map(|()| DrainOutcome::Clean),
        Err(_) => Ok(DrainOutcome::TimedOut),
    }
}

/// Completes when the process is asked to stop: Ctrl-C everywhere, and SIGTERM on
/// Unix (the signal that `docker stop` and Kubernetes send). Supplied; pass it to
/// [`serve_until`] in a real `main`.
pub async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};
        match signal(SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::{deadline, get as http_get, http, request, send};
    use axum::extract::Path;
    use axum::http::Method;
    use serde_json::json;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::time::Instant;
    use tokio::net::TcpStream;
    use tokio::sync::{Semaphore, mpsc};

    const TOKEN: &str = "s3cr3t-token-0123456789";

    fn env(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect()
    }

    #[test]
    fn a_defaults_file_and_environment_are_merged_in_order() {
        let only_token = load_config(None, env(&[("APP_API_TOKEN", TOKEN)])).unwrap();
        assert_eq!(
            only_token,
            Config {
                listen: "127.0.0.1:8080".parse().unwrap(),
                database_url: "sqlite::memory:".to_owned(),
                drain_seconds: 10,
                api_token: Secret::new(TOKEN),
            }
        );

        let file = format!(
            "listen = \"0.0.0.0:9000\"\ndatabase_url = \"sqlite://data/app.db\"\n\
             drain_seconds = 25\napi_token = \"{TOKEN}\"\n"
        );
        let from_file = load_config(Some(&file), env(&[("PATH", "/usr/bin"), ("HOME", "/root")]))
            .expect("variables without the APP_ prefix are ignored");
        assert_eq!(from_file.listen, "0.0.0.0:9000".parse().unwrap());
        assert_eq!(from_file.database_url, "sqlite://data/app.db");
        assert_eq!(from_file.drain_seconds, 25);
        assert_eq!(from_file.api_token.expose(), TOKEN);

        let overridden = load_config(
            Some(&file),
            env(&[
                ("APP_DRAIN_SECONDS", "40"),
                ("APP_LISTEN", "127.0.0.1:0"),
                ("APP_API_TOKEN", "another-token-9876543210"),
            ]),
        )
        .unwrap();
        assert_eq!(overridden.drain_seconds, 40, "environment beats file");
        assert_eq!(overridden.listen.port(), 0);
        assert_eq!(overridden.api_token.expose(), "another-token-9876543210");
        assert_eq!(
            overridden.database_url, "sqlite://data/app.db",
            "settings the environment does not mention keep the file's value"
        );
    }

    #[test]
    fn a_mistakes_are_reported_by_key() {
        let token = ("APP_API_TOKEN", TOKEN);
        assert_eq!(
            load_config(None, env(&[])),
            Err(ConfigError::Missing("api_token"))
        );
        assert_eq!(
            load_config(None, env(&[token, ("APP_DRIAN_SECONDS", "5")])),
            Err(ConfigError::UnknownVariable("APP_DRIAN_SECONDS".to_owned())),
            "a misspelled APP_ variable must not be ignored"
        );
        let key_of = |result: Result<Config, ConfigError>| match result {
            Err(ConfigError::Invalid { key, problem }) => {
                assert!(!problem.is_empty());
                key
            }
            other => panic!("expected Invalid, got {other:?}"),
        };
        for bad in ["0", "301", "-1", "ten", "", "1.5"] {
            assert_eq!(
                key_of(load_config(None, env(&[token, ("APP_DRAIN_SECONDS", bad)]))),
                "drain_seconds",
                "{bad:?}"
            );
        }
        assert_eq!(
            key_of(load_config(
                None,
                env(&[token, ("APP_LISTEN", "localhost")])
            )),
            "listen"
        );
        assert_eq!(
            key_of(load_config(None, env(&[token, ("APP_DATABASE_URL", " ")]))),
            "database_url"
        );
        assert_eq!(
            key_of(load_config(Some("drain_seconds = 0"), env(&[token]))),
            "drain_seconds",
            "file values are validated like environment values"
        );
        assert!(matches!(
            load_config(Some("drain_secs = 5"), env(&[token])),
            Err(ConfigError::File(_))
        ));
        assert!(matches!(
            load_config(Some("listen = [1, 2]"), env(&[token])),
            Err(ConfigError::File(_))
        ));
        assert!(matches!(
            load_config(Some("this is not toml"), env(&[token])),
            Err(ConfigError::File(_))
        ));
    }

    #[test]
    fn a_secrets_stay_out_of_errors_and_debug_output() {
        let config = load_config(None, env(&[("APP_API_TOKEN", TOKEN)])).unwrap();
        assert!(
            !format!("{config:?}").contains(TOKEN),
            "Debug output of the config must not show the token"
        );

        let too_short = "short-secret";
        let broken_line = "api_token = \"unterminated-secret-value";
        let wrong_type =
            "drain_seconds = \"quoted-secret-neighbour\"\napi_token = 12345678901234567890";
        let cases = [
            (
                load_config(None, env(&[("APP_API_TOKEN", too_short)])),
                too_short,
            ),
            (
                load_config(Some(&format!("api_token = \"{too_short}\"")), env(&[])),
                too_short,
            ),
            (
                load_config(Some(broken_line), env(&[])),
                "unterminated-secret-value",
            ),
            (
                load_config(Some(wrong_type), env(&[])),
                "12345678901234567890",
            ),
        ];
        for (result, secret) in cases {
            let error = result.expect_err("these configurations are invalid");
            let shown = format!("{error} / {error:?}");
            assert!(
                !shown.contains(secret),
                "the error text leaks a secret value: {shown}"
            );
        }
    }

    const EMPTY_EXPOSITION: &str = "\
# HELP http_requests_total Requests handled, by method, route and status.
# TYPE http_requests_total counter
# HELP http_request_duration_seconds Time from request to response.
# TYPE http_request_duration_seconds histogram
http_request_duration_seconds_bucket{le=\"0.01\"} 0
http_request_duration_seconds_bucket{le=\"0.1\"} 0
http_request_duration_seconds_bucket{le=\"1\"} 0
http_request_duration_seconds_bucket{le=\"+Inf\"} 0
http_request_duration_seconds_sum 0.000
http_request_duration_seconds_count 0
";

    #[test]
    fn b_render_matches_the_exposition_format() {
        let metrics = Metrics::default();
        assert_eq!(metrics.render(), EMPTY_EXPOSITION);

        let ms = Duration::from_millis;
        metrics.record("POST", "/items", 201, ms(50));
        metrics.record("GET", "/items/{id}", 200, ms(5));
        metrics.clone().record("GET", "/items/{id}", 200, ms(2500));
        assert_eq!(
            metrics.render(),
            "\
# HELP http_requests_total Requests handled, by method, route and status.
# TYPE http_requests_total counter
http_requests_total{method=\"GET\",route=\"/items/{id}\",status=\"200\"} 2
http_requests_total{method=\"POST\",route=\"/items\",status=\"201\"} 1
# HELP http_request_duration_seconds Time from request to response.
# TYPE http_request_duration_seconds histogram
http_request_duration_seconds_bucket{le=\"0.01\"} 1
http_request_duration_seconds_bucket{le=\"0.1\"} 2
http_request_duration_seconds_bucket{le=\"1\"} 2
http_request_duration_seconds_bucket{le=\"+Inf\"} 3
http_request_duration_seconds_sum 2.555
http_request_duration_seconds_count 3
",
            "clones share one registry; buckets are cumulative"
        );
    }

    #[test]
    fn b_bucket_bounds_are_inclusive_and_labels_are_escaped() {
        let metrics = Metrics::default();
        let ms = Duration::from_millis;
        for millis in [0, 10, 11, 100, 101, 1000, 1001] {
            metrics.record("GET", "/", 200, ms(millis));
        }
        let text = metrics.render();
        for line in [
            "http_request_duration_seconds_bucket{le=\"0.01\"} 2",
            "http_request_duration_seconds_bucket{le=\"0.1\"} 4",
            "http_request_duration_seconds_bucket{le=\"1\"} 6",
            "http_request_duration_seconds_bucket{le=\"+Inf\"} 7",
            "http_request_duration_seconds_sum 2.223",
            "http_request_duration_seconds_count 7",
        ] {
            assert!(
                text.lines().any(|l| l == line),
                "missing line {line:?}; `le` means less than or equal. Got:\n{text}"
            );
        }

        let odd = Metrics::default();
        odd.record("GET", "/say/\"hi\"\\there\nyou", 200, ms(1));
        assert!(
            odd.render()
                .contains(r#"route="/say/\"hi\"\\there\nyou",status="200"} 1"#),
            "label values need escaping:\n{}",
            odd.render()
        );
    }

    fn item_routes() -> Router {
        Router::new()
            .route(
                "/items/{id}",
                get(|Path(id): Path<u32>| async move {
                    tokio::time::sleep(Duration::from_millis(u64::from(id))).await;
                    if id == 404 {
                        StatusCode::NOT_FOUND
                    } else {
                        StatusCode::OK
                    }
                }),
            )
            .route(
                "/items",
                axum::routing::post(|| async { StatusCode::CREATED }),
            )
    }

    #[tokio::test(start_paused = true)]
    async fn c_requests_are_counted_by_route_template() {
        crate::support::deadline(
            600,
            "the test `c_requests_are_counted_by_route_template`",
            async {
                let metrics = Metrics::default();
                let app = with_metrics(item_routes(), metrics.clone());
                for uri in ["/items/5", "/items/50", "/items/7", "/items/404"] {
                    send(&app, http_get(uri)).await;
                }
                send(&app, request(Method::POST, "/items")).await;
                send(&app, http_get("/wp-login.php")).await;
                send(&app, http_get("/.env")).await;

                let response = send(&app, http_get("/metrics")).await;
                assert_eq!(response.code(), 200);
                assert_eq!(
                    response.header("content-type"),
                    Some("text/plain; version=0.0.4")
                );
                let expected = "\
# HELP http_requests_total Requests handled, by method, route and status.
# TYPE http_requests_total counter
http_requests_total{method=\"GET\",route=\"/items/{id}\",status=\"200\"} 3
http_requests_total{method=\"GET\",route=\"/items/{id}\",status=\"404\"} 1
http_requests_total{method=\"GET\",route=\"unmatched\",status=\"404\"} 2
http_requests_total{method=\"POST\",route=\"/items\",status=\"201\"} 1
# HELP http_request_duration_seconds Time from request to response.
# TYPE http_request_duration_seconds histogram
http_request_duration_seconds_bucket{le=\"0.01\"} 5
http_request_duration_seconds_bucket{le=\"0.1\"} 6
http_request_duration_seconds_bucket{le=\"1\"} 7
http_request_duration_seconds_bucket{le=\"+Inf\"} 7
http_request_duration_seconds_sum 0.466
http_request_duration_seconds_count 7
";
                assert_eq!(
                    response.text(),
                    expected,
                    "three different ids are ONE series (the route template); paths that \
             match nothing share the label `unmatched`"
                );
                assert_eq!(
                    send(&app, http_get("/metrics")).await.text(),
                    expected,
                    "scraping /metrics must not change the numbers"
                );
                assert_eq!(
                    metrics.render(),
                    expected,
                    "the endpoint shows the shared registry"
                );
            },
        )
        .await
    }

    async fn memory_pool() -> SqlitePool {
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn e_liveness_and_readiness_answer_different_questions() {
        crate::support::deadline(
            120,
            "the test `e_liveness_and_readiness_answer_different_questions`",
            async {
                let lifecycle = Lifecycle::new();
                let pool = memory_pool().await;
                // A generous limit: nothing in this test is about speed, and a
                // closed pool fails at once.
                let app = health_routes(lifecycle.clone(), pool.clone(), Duration::from_secs(10));

                let ready = |code: u16, body: serde_json::Value| (code, body);
                let check = async |app: &Router| {
                    let response = send(app, http_get("/readyz")).await;
                    (response.code(), response.json())
                };
                let live = async |app: &Router| {
                    let response = send(app, http_get("/healthz")).await;
                    (response.code(), response.text())
                };

                assert_eq!(live(&app).await, (200, "ok".to_owned()));
                assert_eq!(
                    check(&app).await,
                    ready(503, json!({ "ready": false, "reason": "starting" }))
                );
                lifecycle.set(Phase::Ready);
                assert_eq!(check(&app).await, ready(200, json!({ "ready": true })));
                lifecycle.set(Phase::Draining);
                assert_eq!(
                    check(&app).await,
                    ready(503, json!({ "ready": false, "reason": "draining" }))
                );
                assert_eq!(
                    live(&app).await,
                    (200, "ok".to_owned()),
                    "a draining process is alive; restarting it would cut off its requests"
                );

                lifecycle.set(Phase::Ready);
                pool.close().await;
                assert_eq!(
                    check(&app).await,
                    ready(503, json!({ "ready": false, "reason": "database" }))
                );
                assert_eq!(
                    live(&app).await,
                    (200, "ok".to_owned()),
                    "liveness must not depend on the database"
                );
            },
        )
        .await
    }

    #[tokio::test]
    async fn e_readiness_answers_in_time_when_the_pool_is_exhausted() {
        crate::support::deadline(
            120,
            "the test `e_readiness_answers_in_time_when_the_pool_is_exhausted`",
            async {
                let lifecycle = Lifecycle::new();
                lifecycle.set(Phase::Ready);
                let pool = memory_pool().await;
                let app =
                    health_routes(lifecycle.clone(), pool.clone(), Duration::from_millis(100));
                // The pool's own acquire timeout is 30 s; the check must not wait for it.
                let held = pool.acquire().await.unwrap();
                let start = Instant::now();
                let response = deadline(
                    10,
                    "/readyz with an exhausted pool",
                    send(&app, http_get("/readyz")),
                )
                .await;
                assert_eq!(response.code(), 503);
                assert_eq!(response.json()["reason"], "database");
                assert!(
                    start.elapsed() < Duration::from_secs(5),
                    "the readiness check has its own, short timeout"
                );
                drop(held);
                // With the connection back, the check succeeds. This router has a
                // generous limit, so a busy test machine cannot fail the assertion.
                let patient = health_routes(lifecycle, pool.clone(), Duration::from_secs(10));
                assert_eq!(send(&patient, http_get("/readyz")).await.code(), 200);
            },
        )
        .await
    }

    /// A router with one route that reports when a request arrives and then
    /// waits for a permit.
    fn slow_app(entered: mpsc::UnboundedSender<()>, gate: Arc<Semaphore>) -> Router {
        Router::new().route(
            "/work",
            get(move || {
                let (entered, gate) = (entered.clone(), Arc::clone(&gate));
                async move {
                    let _ = entered.send(());
                    let _permit = gate.acquire().await.expect("gate open");
                    "done"
                }
            }),
        )
    }

    /// Polls an async condition for at most ten seconds.
    async fn eventually<F, Fut>(what: &str, mut condition: F)
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = bool>,
    {
        let poll = async {
            while !condition().await {
                tokio::time::sleep(Duration::from_millis(2)).await;
            }
        };
        deadline(10, what, poll).await;
    }

    #[tokio::test]
    async fn f_shutdown_lets_in_flight_requests_finish() {
        crate::support::deadline(
            120,
            "the test `f_shutdown_lets_in_flight_requests_finish`",
            async {
                let (entered_tx, mut entered) = mpsc::unbounded_channel();
                let gate = Arc::new(Semaphore::new(0));
                let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
                let addr = listener.local_addr().unwrap();
                let lifecycle = Lifecycle::new();
                lifecycle.set(Phase::Ready);
                let (stop, stopped) = oneshot::channel::<()>();
                let server = tokio::spawn(serve_until(
                    listener,
                    slow_app(entered_tx, Arc::clone(&gate)),
                    lifecycle.clone(),
                    Duration::from_secs(30),
                    async {
                        let _ = stopped.await;
                    },
                ));

                let in_flight = tokio::spawn(http(http_get(&format!("http://{addr}/work"))));
                deadline(10, "the request reaching the handler", entered.recv()).await;
                assert_eq!(
                    lifecycle.phase(),
                    Phase::Ready,
                    "serving does not change the phase"
                );

                stop.send(()).unwrap();
                eventually("the phase becoming Draining", || async {
                    lifecycle.phase() == Phase::Draining
                })
                .await;
                eventually("new connections being refused", || async {
                    TcpStream::connect(addr).await.is_err()
                })
                .await;
                assert!(
                    !server.is_finished(),
                    "the server must wait for the request that is still in progress"
                );

                gate.add_permits(1);
                let response = deadline(10, "the in-flight request", in_flight)
                    .await
                    .unwrap()
                    .expect("the in-flight request must get its response, not a reset connection");
                assert_eq!((response.code(), response.text().as_str()), (200, "done"));
                let outcome = deadline(10, "serve_until", server).await.unwrap().unwrap();
                assert_eq!(outcome, DrainOutcome::Clean);
            },
        )
        .await
    }

    #[tokio::test]
    async fn f_the_drain_time_is_an_upper_bound() {
        crate::support::deadline(
            120,
            "the test `f_the_drain_time_is_an_upper_bound`",
            async {
                let (entered_tx, mut entered) = mpsc::unbounded_channel();
                // This gate never opens: the request cannot finish.
                let gate = Arc::new(Semaphore::new(0));
                let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
                let addr = listener.local_addr().unwrap();
                let lifecycle = Lifecycle::new();
                let (stop, stopped) = oneshot::channel::<()>();
                let drain = Duration::from_millis(150);
                let server = tokio::spawn(serve_until(
                    listener,
                    slow_app(entered_tx, gate),
                    lifecycle.clone(),
                    drain,
                    async {
                        let _ = stopped.await;
                    },
                ));
                let stuck = tokio::spawn(http(http_get(&format!("http://{addr}/work"))));
                deadline(10, "the request reaching the handler", entered.recv()).await;

                let asked = Instant::now();
                stop.send(()).unwrap();
                let outcome = deadline(10, "serve_until with a stuck request", server)
                    .await
                    .unwrap()
                    .unwrap();
                assert_eq!(outcome, DrainOutcome::TimedOut);
                assert!(
                    asked.elapsed() >= drain,
                    "in-flight requests get the whole drain time"
                );
                assert_eq!(lifecycle.phase(), Phase::Draining);
                stuck.abort();
            },
        )
        .await
    }

    #[tokio::test]
    async fn f_shutdown_without_traffic_is_immediate_and_clean() {
        crate::support::deadline(
            120,
            "the test `f_shutdown_without_traffic_is_immediate_and_clean`",
            async {
                let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
                let lifecycle = Lifecycle::new();
                let outcome = deadline(
                    10,
                    "serve_until with nothing to drain",
                    serve_until(
                        listener,
                        Router::new(),
                        lifecycle.clone(),
                        Duration::from_secs(30),
                        async {},
                    ),
                )
                .await
                .unwrap();
                assert_eq!(outcome, DrainOutcome::Clean);
                assert_eq!(lifecycle.phase(), Phase::Draining);
            },
        )
        .await
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[tokio::test]
    async fn p_predictions() {
        crate::support::deadline(120, "the test `p_predictions`", async {
            // P1. A handler returns the `MatchedPath` it sees. What is the response
            // body for `GET /orders/1234/lines/7`?
            let app = Router::new().route(
                "/orders/{order}/lines/{line}",
                get(|matched: MatchedPath| async move { matched.as_str().to_owned() }),
            );
            let p1: &str = "/orders/{order}/lines/{line}";
            assert_eq!(
                p1,
                send(&app, http_get("/orders/1234/lines/7")).await.text(),
                "P1: MatchedPath is the template"
            );

            // P2. This struct expects a TOML integer. Does a quoted number parse?
            #[derive(Deserialize)]
            struct Probe {
                #[allow(dead_code)]
                workers: u32,
            }
            let p2: bool = false;
            assert_eq!(
                p2,
                toml::from_str::<Probe>("workers = \"4\"").is_ok(),
                "P2: TOML values have types"
            );

            // P3. A listener is bound to port 0. What is true of the port that
            // `local_addr()` reports: is it still 0?
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let p3: bool = false;
            assert_eq!(
                p3,
                listener.local_addr().unwrap().port() == 0,
                "P3: port 0 asks the operating system to choose"
            );

            // P4. `Lifecycle` is cloned and the clone is set to Draining. Which phase
            // does the original report?
            let original = Lifecycle::new();
            original.clone().set(Phase::Draining);
            let p4: Phase = Phase::Draining;
            assert_eq!(p4, original.phase(), "P4: clones share the Arc");
        })
        .await
    }
}
