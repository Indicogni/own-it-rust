//! LESSON E09 — An HTTP JSON service and its client, on loopback (after E01, E02, and E06)
//! Run: cargo test --manifest-path ecosystem/Cargo.toml e09_
//! Try: cargo run --manifest-path ecosystem/Cargo.toml --example e09_serve
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: A client asks for an item that does not
//! exist. Is that an error for the HTTP library, for your client type, or for neither?
//! What should happen to a request that is half done when the service is told to stop?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! The tests never leave your machine. The `a_`, `b_`, and `c_` tests hand requests
//! straight to the router with no socket at all. The `d_`, `e_`, and `f_` tests bind
//! `127.0.0.1:0`, so the system picks a free port, and a server and a client talk
//! over the loopback interface.
//! EXPLAIN afterward: Follow one `POST /stock/bolt/take` from the client call to the
//! handler and back. Name each place where it can fail and what the caller sees in
//! each case. Why must the handler check and subtract under one lock? What does
//! graceful shutdown promise to a request that is already in flight?
//! JUDGMENT: axum is the usual choice for services on tokio. It is a thin layer over
//! hyper (the HTTP implementation) and tower (middleware), so what you learn carries
//! over to tonic and to hyper itself. reqwest is the usual client. For a quick script,
//! ureq (blocking, small) is enough. actix-web is a mature alternative with its own
//! runtime setup. Using hyper directly is for proxies and other special cases.

use axum::Router;
use axum::response::{IntoResponse, Response};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use thiserror::Error;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

/// The stock level of one item, as sent on the wire: `{"sku":"bolt","quantity":7}`.
/// (A "sku" is a stock-keeping unit: the shop's name for one kind of item.)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stock {
    pub sku: String,
    pub quantity: u32,
}

/// The request body of `PUT /stock/{sku}` and `POST /stock/{sku}/take`: `{"quantity":7}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Quantity {
    pub quantity: u32,
}

/// The body of every error response:
/// ``{"error":"not_found","message":"no stock for `bolt`"}``.
/// `error` is a stable code for programs; `message` is text for people.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorBody {
    pub error: String,
    pub message: String,
}

/// Everything the service can refuse to do. (Supplied: you practiced thiserror in E01.)
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ApiError {
    #[error("no stock for `{0}`")]
    NotFound(String),
    #[error("only {available} left")]
    Insufficient { available: u32 },
    #[error("{0}")]
    BadRequest(String),
    #[error("the request took too long")]
    Timeout,
}

/// A. Turn an `ApiError` into an HTTP response: the status from this table, and a JSON
/// `ErrorBody` whose `error` is the code from the table and whose `message` is the
/// error's Display text.
///
/// | Variant | Status | `error` |
/// | --- | --- | --- |
/// | `NotFound` | 404 Not Found | `not_found` |
/// | `Insufficient` | 409 Conflict | `insufficient` |
/// | `BadRequest` | 400 Bad Request | `bad_request` |
/// | `Timeout` | 503 Service Unavailable | `timeout` |
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        todo!("Implement this task")
    }
}

/// The service's shared state. A clone is another handle to the same map; axum gives
/// each handler call its own clone.
#[derive(Debug, Clone, Default)]
pub struct AppState {
    stock: Arc<Mutex<BTreeMap<String, u32>>>,
}

impl AppState {
    /// The stored quantity for `sku`. The tests use this to look at the state
    /// without going through HTTP.
    pub fn quantity(&self, sku: &str) -> Option<u32> {
        self.stock
            .lock()
            .expect("a handler panicked while holding the stock lock")
            .get(sku)
            .copied()
    }
}

/// B. Build the router for `state`. You write the handler functions too. Routes:
///
/// - `GET /health` → 200 with `{"status":"ok"}`.
/// - `PUT /stock/{sku}` with a `Quantity` body: store that quantity. → 201 Created
///   when the sku is new and 200 OK when it replaces an older value. The body is the
///   stored `Stock`.
/// - `GET /stock/{sku}` → 200 with the `Stock`, or `ApiError::NotFound(sku)`.
/// - `POST /stock/{sku}/take` with a `Quantity` body: remove that many. → 200 with
///   the remaining `Stock`. An unknown sku gives `ApiError::NotFound(sku)`. Asking for
///   more than is left gives `ApiError::Insufficient { available }` and changes nothing.
///   Check and subtract while holding the lock ONCE: with two separate lock calls, two
///   requests can both take the last item.
///
/// A body that is not a valid `Quantity` (broken JSON, a wrong shape, or a missing
/// `Content-Type: application/json` header) gives `ApiError::BadRequest(text)`, where
/// `text` is the rejection's `body_text()`. Take the body as
/// `Result<Json<Quantity>, JsonRejection>`: then your handler sees the rejection, and
/// the client gets your JSON error instead of axum's plain-text answer. Look at the
/// body before you look at the stock: a bad body is a 400 even when the sku is unknown.
pub fn app(state: AppState) -> Router {
    todo!("Implement this task")
}

/// C. Wrap `app` so that no request runs longer than `limit`. If the inner service has
/// not produced a response by then, answer with `ApiError::Timeout` and drop the
/// handler's future (it is cancelled at its current `.await`, as in E06).
/// Tip: `axum::middleware::from_fn` with `tokio::time::timeout` around `next.run(request)`.
pub fn with_deadline(app: Router, limit: Duration) -> Router {
    todo!("Implement this task")
}

/// D. Serve `app` on `listener` until `shutdown` is cancelled. Then shut down
/// gracefully: stop accepting connections, let every request that is already in flight
/// finish and send its response, and only then return.
pub async fn serve(
    listener: TcpListener,
    app: Router,
    shutdown: CancellationToken,
) -> std::io::Result<()> {
    todo!("Implement this task")
}

/// What a `StockClient` call can fail with.
#[derive(Debug, Error)]
pub enum ClientError {
    /// No complete response arrived within the client's timeout.
    #[error("the request timed out")]
    Timeout,
    /// The server answered with a status that is not a success.
    #[error("the server answered {status}: {message}")]
    Api {
        status: u16,
        code: String,
        message: String,
    },
    /// Anything else: connection refused or closed, or a body that is not the
    /// expected JSON.
    #[error("could not talk to the server")]
    Transport(#[source] reqwest::Error),
}

/// E. A typed client for the stock service. Rules for every call:
///
/// - A `reqwest::Error` for which `is_timeout()` is true becomes `ClientError::Timeout`.
///   Every other `reqwest::Error` becomes `ClientError::Transport`.
/// - A response whose status is not a success becomes `ClientError::Api`. reqwest does
///   NOT turn 404 or 500 into an error for you. When the body is an `ErrorBody`, `code`
///   and `message` come from it. Otherwise `code` is `unknown` and `message` is the
///   body text (a proxy in front of the service may answer with HTML).
/// - The sku is data, not URL syntax. Add it as one path segment with
///   `Url::path_segments_mut`, which percent-encodes `/`, spaces, `?`, and `%`.
///   That method returns `Err` only for a URL that cannot have a path, such as
///   `mailto:a@example.com`. An `http://` base always can, so `expect` with a
///   message is fine there.
#[derive(Debug, Clone)]
pub struct StockClient {
    base: Url,
    http: reqwest::Client,
}

impl StockClient {
    /// E. `base` is the root URL of the service, for example `http://127.0.0.1:8080`.
    /// Build ONE `reqwest::Client` here and use it for every call: it owns the
    /// connection pool. Configure it with `timeout` as the limit for a whole request
    /// and with `no_proxy()`: by default reqwest obeys proxy variables such as
    /// `http_proxy`, and a request meant for loopback must never go to a proxy.
    pub fn new(base: Url, timeout: Duration) -> Result<StockClient, ClientError> {
        todo!("Implement this task")
    }

    /// E. `GET /stock/{sku}`. `Ok(Some(quantity))` on success. `Ok(None)` when the server
    /// answers 404 with the code `not_found`. Any other 404 (a wrong base URL, for
    /// example) stays an error.
    pub async fn get(&self, sku: &str) -> Result<Option<u32>, ClientError> {
        todo!("Implement this task")
    }

    /// E. `PUT /stock/{sku}`. `Ok(true)` when the server created the sku (201) and
    /// `Ok(false)` when it replaced an older value (200).
    pub async fn set(&self, sku: &str, quantity: u32) -> Result<bool, ClientError> {
        todo!("Implement this task")
    }

    /// E. `POST /stock/{sku}/take`. `Ok(remaining)` on success.
    pub async fn take(&self, sku: &str, quantity: u32) -> Result<u32, ClientError> {
        todo!("Implement this task")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{paused_time, real_time, run, worker_threads};
    use axum::Json;
    use axum::body::Body;
    use axum::http::{Request, StatusCode, header};
    use axum::routing::{get, put};
    use serde_json::{Value, json};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use tokio::sync::Notify;
    use tokio::time::Instant;
    use tower::ServiceExt as _;

    const MS: Duration = Duration::from_millis(1);
    /// Generous real-time limit for steps that are quick when the solution is right.
    const PATIENCE: Duration = Duration::from_secs(10);
    /// Rounds of the race in `b_concurrent_takes_never_oversell`.
    const ROUNDS: usize = 300;

    /// Splits a response into its status and its JSON body (`Null` for an empty body).
    async fn parts(response: Response) -> (StatusCode, Value) {
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or_else(|_| {
                panic!(
                    "the body must be JSON, got {:?}",
                    String::from_utf8_lossy(&bytes)
                )
            })
        };
        (status, body)
    }

    /// Sends one request straight into the router: no socket, no port, no client.
    async fn call(
        app: &Router,
        method: &str,
        path: &str,
        body: Option<&str>,
    ) -> (StatusCode, Value) {
        let mut request = Request::builder().method(method).uri(path);
        if body.is_some() {
            request = request.header(header::CONTENT_TYPE, "application/json");
        }
        let body = body.map_or_else(Body::empty, |text| Body::from(text.to_owned()));
        let response = app
            .clone()
            .oneshot(request.body(body).unwrap())
            .await
            .unwrap();
        parts(response).await
    }

    #[test]
    fn a_errors_become_json_responses() {
        run(real_time, || async {
            let response = ApiError::NotFound("bolt".into()).into_response();
            assert_eq!(
                response.headers()[header::CONTENT_TYPE],
                "application/json",
                "error bodies are JSON, like every other body of this service"
            );
            assert_eq!(
                parts(response).await,
                (
                    StatusCode::NOT_FOUND,
                    json!({"error": "not_found", "message": "no stock for `bolt`"})
                )
            );
            assert_eq!(
                parts(ApiError::Insufficient { available: 3 }.into_response()).await,
                (
                    StatusCode::CONFLICT,
                    json!({"error": "insufficient", "message": "only 3 left"})
                )
            );
            assert_eq!(
                parts(ApiError::BadRequest("not a number".into()).into_response()).await,
                (
                    StatusCode::BAD_REQUEST,
                    json!({"error": "bad_request", "message": "not a number"})
                )
            );
            assert_eq!(
                parts(ApiError::Timeout.into_response()).await,
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    json!({"error": "timeout", "message": "the request took too long"})
                )
            );
        });
    }

    #[test]
    fn b_put_then_get() {
        run(real_time, || async {
            let state = AppState::default();
            let app = app(state.clone());
            assert_eq!(
                call(&app, "GET", "/health", None).await,
                (StatusCode::OK, json!({"status": "ok"}))
            );
            assert_eq!(
                call(&app, "PUT", "/stock/bolt", Some(r#"{"quantity":7}"#)).await,
                (StatusCode::CREATED, json!({"sku": "bolt", "quantity": 7})),
                "a new sku is 201 Created"
            );
            assert_eq!(
                state.quantity("bolt"),
                Some(7),
                "the handler must write to the state it was given, not to a copy of the map"
            );
            assert_eq!(
                call(&app, "PUT", "/stock/bolt", Some(r#"{"quantity":9}"#)).await,
                (StatusCode::OK, json!({"sku": "bolt", "quantity": 9})),
                "replacing a value is 200 OK"
            );
            assert_eq!(
                call(&app, "GET", "/stock/bolt", None).await,
                (StatusCode::OK, json!({"sku": "bolt", "quantity": 9}))
            );
            assert_eq!(
                call(&app, "GET", "/stock/nut", None).await,
                (
                    StatusCode::NOT_FOUND,
                    json!({"error": "not_found", "message": "no stock for `nut`"})
                )
            );
        });
    }

    #[test]
    fn b_take_checks_and_subtracts() {
        run(real_time, || async {
            let state = AppState::default();
            let app = app(state.clone());
            call(&app, "PUT", "/stock/bolt", Some(r#"{"quantity":5}"#)).await;
            assert_eq!(
                call(&app, "POST", "/stock/bolt/take", Some(r#"{"quantity":2}"#)).await,
                (StatusCode::OK, json!({"sku": "bolt", "quantity": 3})),
                "the body is the stock that remains"
            );
            assert_eq!(
                call(&app, "POST", "/stock/bolt/take", Some(r#"{"quantity":4}"#)).await,
                (
                    StatusCode::CONFLICT,
                    json!({"error": "insufficient", "message": "only 3 left"})
                )
            );
            assert_eq!(
                state.quantity("bolt"),
                Some(3),
                "a refused request must change nothing"
            );
            assert_eq!(
                call(&app, "POST", "/stock/bolt/take", Some(r#"{"quantity":3}"#)).await,
                (StatusCode::OK, json!({"sku": "bolt", "quantity": 0})),
                "taking exactly what is left is allowed"
            );
            assert_eq!(
                call(&app, "POST", "/stock/nut/take", Some(r#"{"quantity":1}"#))
                    .await
                    .0,
                StatusCode::NOT_FOUND
            );
        });
    }

    #[test]
    fn b_bad_bodies_get_the_json_error_format() {
        run(real_time, || async {
            let state = AppState::default();
            let app = app(state.clone());
            call(&app, "PUT", "/stock/bolt", Some(r#"{"quantity":5}"#)).await;
            for body in [
                r#"{"quantity":"#,
                r#"{"quantity":"many"}"#,
                r#"{"quantity":-1}"#,
                r#"{"quantity":1,"force":true}"#,
                r#"{}"#,
            ] {
                for (method, path) in [("PUT", "/stock/bolt"), ("POST", "/stock/bolt/take")] {
                    let (status, json) = call(&app, method, path, Some(body)).await;
                    assert_eq!(status, StatusCode::BAD_REQUEST, "{method} {path} {body}");
                    assert_eq!(json["error"], "bad_request", "{method} {path} {body}");
                    assert!(
                        json["message"].as_str().is_some_and(|m| !m.is_empty()),
                        "the message should say what is wrong with {body}: {json}"
                    );
                }
            }
            // The body is checked first, so the answer does not depend on the stock.
            assert_eq!(
                call(&app, "POST", "/stock/nut/take", Some("{}")).await.0,
                StatusCode::BAD_REQUEST,
                "a bad body is a 400 even for an unknown sku: check the body before the stock"
            );
            // The right JSON, but no `Content-Type: application/json` header.
            let request = Request::builder()
                .method("PUT")
                .uri("/stock/bolt")
                .body(Body::from(r#"{"quantity":1}"#))
                .unwrap();
            let (status, json) = parts(app.clone().oneshot(request).await.unwrap()).await;
            assert_eq!(
                (status, &json["error"]),
                (StatusCode::BAD_REQUEST, &json!("bad_request"))
            );
            assert_eq!(
                state.quantity("bolt"),
                Some(5),
                "bad requests change nothing"
            );
        });
    }

    #[test]
    fn b_path_segments_arrive_decoded() {
        run(real_time, || async {
            let state = AppState::default();
            let app = app(state.clone());
            let (status, json) = call(
                &app,
                "PUT",
                "/stock/m8%20bolt%2Fzinc",
                Some(r#"{"quantity":1}"#),
            )
            .await;
            assert_eq!(status, StatusCode::CREATED);
            assert_eq!(json["sku"], "m8 bolt/zinc");
            assert_eq!(
                state.quantity("m8 bolt/zinc"),
                Some(1),
                "store the decoded sku that the `Path` extractor gives you"
            );
        });
    }

    #[test]
    fn b_concurrent_takes_never_oversell() {
        const BUYERS: usize = 4;
        let state = AppState::default();
        let app = app(state.clone());
        // `round` is the starting gun: every buyer thread spins until it changes, so
        // that all of them call the handler at (nearly) the same instant.
        let round = Arc::new(AtomicUsize::new(0));
        // If an assertion below fails, this guard tells the buyer threads to stop.
        struct Finish(Arc<AtomicUsize>);
        impl Drop for Finish {
            fn drop(&mut self) {
                self.0.store(usize::MAX, Ordering::Release);
            }
        }
        let _finish = Finish(Arc::clone(&round));
        let (report, reports) = std::sync::mpsc::channel();
        for _ in 0..BUYERS {
            let (app, round, report) = (app.clone(), Arc::clone(&round), report.clone());
            std::thread::spawn(move || {
                let runtime = real_time();
                for this_round in 1..=ROUNDS {
                    let mut spins = 0_u32;
                    loop {
                        let now = round.load(Ordering::Acquire);
                        if now == this_round {
                            break;
                        }
                        if now == usize::MAX {
                            return;
                        }
                        spins += 1;
                        if spins.is_multiple_of(10_000) {
                            // Be polite on machines with fewer cores than buyers.
                            std::thread::yield_now();
                        } else {
                            std::hint::spin_loop();
                        }
                    }
                    let path = format!("/stock/item{this_round}/take");
                    let (status, _) =
                        runtime.block_on(call(&app, "POST", &path, Some(r#"{"quantity":1}"#)));
                    if report.send(status).is_err() {
                        return;
                    }
                }
            });
        }
        let mut oversold = 0;
        for this_round in 1..=ROUNDS {
            // One item in stock, and every buyer wants it.
            state
                .stock
                .lock()
                .unwrap()
                .insert(format!("item{this_round}"), 1);
            round.store(this_round, Ordering::Release);
            let sold = (0..BUYERS)
                .map(|_| {
                    reports
                        .recv_timeout(PATIENCE)
                        .expect("a buyer got no answer: its handler panicked (see above) or waits for a lock")
                })
                .filter(|&status| status == StatusCode::OK)
                .count();
            oversold += usize::from(sold != 1);
        }
        assert_eq!(
            oversold, 0,
            "in {oversold} of {ROUNDS} rounds the single item was sold more than once: \
             check and subtract while holding the lock once"
        );
    }

    /// Sets a flag when it is dropped: shows that a handler's future was dropped.
    struct SetOnDrop(Arc<AtomicBool>);
    impl Drop for SetOnDrop {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    #[test]
    fn c_deadline_answers_for_a_stuck_handler() {
        run(paused_time, || async {
            let dropped = Arc::new(AtomicBool::new(false));
            let flag = Arc::clone(&dropped);
            let stuck = move || {
                let guard = SetOnDrop(Arc::clone(&flag));
                async move {
                    let _guard = guard;
                    std::future::pending::<()>().await;
                }
            };
            let app = with_deadline(Router::new().route("/stuck", get(stuck)), MS * 50);

            let start = Instant::now();
            let answer = tokio::time::timeout(PATIENCE, call(&app, "GET", "/stuck", None))
                .await
                .expect("with_deadline must answer for a handler that never finishes");
            assert_eq!(
                answer,
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    json!({"error": "timeout", "message": "the request took too long"})
                )
            );
            assert_eq!(
                start.elapsed(),
                MS * 50,
                "the answer must come exactly at the limit (time is virtual here)"
            );
            assert!(
                dropped.load(Ordering::SeqCst),
                "on a timeout the handler's future must be dropped, not left running"
            );
        });
    }

    #[test]
    fn c_deadline_leaves_fast_requests_alone() {
        run(paused_time, || async {
            let nap = |ms: u32| {
                get(move || async move {
                    tokio::time::sleep(MS * ms).await;
                    Json(json!({"slept_ms": ms}))
                })
            };
            let inner = Router::new()
                .route("/nap49", nap(49))
                .route("/nap51", nap(51));
            let app = with_deadline(inner, MS * 50);
            assert_eq!(
                call(&app, "GET", "/nap49", None).await,
                (StatusCode::OK, json!({"slept_ms": 49})),
                "a response that is ready before the limit passes through unchanged"
            );
            assert_eq!(
                call(&app, "GET", "/nap51", None).await.0,
                StatusCode::SERVICE_UNAVAILABLE
            );
            let (status, _) = call(&app, "GET", "/missing", None).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "other answers are untouched");
        });
    }

    /// A plain reqwest client for the tests that do not use your `StockClient`.
    fn http() -> reqwest::Client {
        reqwest::Client::builder()
            .no_proxy()
            .timeout(PATIENCE)
            .build()
            .unwrap()
    }

    /// Binds a free loopback port and returns the listener with its base URL.
    async fn bind() -> (TcpListener, Url) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = Url::parse(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
        (listener, url)
    }

    /// Serves `router` in the background with plain `axum::serve` (not your `serve`).
    async fn spawn(router: Router) -> Url {
        let (listener, url) = bind().await;
        tokio::spawn(async move { axum::serve(listener, router).await });
        url
    }

    #[test]
    fn d_serves_until_cancelled() {
        run(real_time, || async {
            let (listener, url) = bind().await;
            let shutdown = CancellationToken::new();
            let server = tokio::spawn(serve(listener, app(AppState::default()), shutdown.clone()));
            let response = http()
                .get(url.join("health").unwrap())
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(
                response.json::<Value>().await.unwrap(),
                json!({"status": "ok"})
            );
            assert!(
                !server.is_finished(),
                "serve must run until the token is cancelled"
            );
            shutdown.cancel();
            tokio::time::timeout(PATIENCE, server)
                .await
                .expect("serve must return soon after the token is cancelled")
                .unwrap()
                .expect("a clean shutdown is Ok(())");
        });
    }

    #[test]
    fn d_shutdown_lets_requests_in_flight_finish() {
        run(real_time, || async {
            // A handler that reports when it has started and then waits for the test.
            let started = Arc::new(Notify::new());
            let release = Arc::new(Notify::new());
            let (has_started, wait_for_release) = (Arc::clone(&started), Arc::clone(&release));
            let slow = move || {
                let (has_started, wait_for_release) =
                    (Arc::clone(&has_started), Arc::clone(&wait_for_release));
                async move {
                    has_started.notify_one();
                    wait_for_release.notified().await;
                    Json(json!({"finished": true}))
                }
            };
            let (listener, url) = bind().await;
            let shutdown = CancellationToken::new();
            let router = Router::new().route("/slow", get(slow));
            let mut server = tokio::spawn(serve(listener, router, shutdown.clone()));

            let request = tokio::spawn(http().get(url.join("slow").unwrap()).send());
            tokio::select! {
                () = started.notified() => {}
                early = &mut server => {
                    panic!("serve returned before it was cancelled: {early:?}")
                }
                () = tokio::time::sleep(PATIENCE) => {
                    panic!("the request never reached the slow handler")
                }
            }

            // The request is now in flight. Ask the server to stop.
            shutdown.cancel();
            tokio::time::sleep(MS * 100).await;
            assert!(
                !server.is_finished(),
                "serve returned while a request was still in flight: that request was cut off"
            );

            release.notify_one();
            let response = request
                .await
                .unwrap()
                .expect("the request in flight must still get its response after cancellation");
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(
                response.json::<Value>().await.unwrap(),
                json!({"finished": true})
            );
            tokio::time::timeout(PATIENCE, server)
                .await
                .expect("serve must return once the last request has finished")
                .unwrap()
                .expect("a clean shutdown is Ok(())");
        });
    }

    #[test]
    fn e_client_round_trip() {
        run(real_time, || async {
            let state = AppState::default();
            let url = spawn(app(state.clone())).await;
            let client = StockClient::new(url, PATIENCE).unwrap();

            assert!(
                matches!(client.get("bolt").await, Ok(None)),
                "a sku that is not stocked is an answer (None), not an error"
            );
            assert!(matches!(client.set("bolt", 5).await, Ok(true)), "created");
            assert!(matches!(client.set("bolt", 6).await, Ok(false)), "replaced");
            assert_eq!(state.quantity("bolt"), Some(6));
            assert!(matches!(client.get("bolt").await, Ok(Some(6))));
            assert!(matches!(client.take("bolt", 2).await, Ok(4)), "4 remain");

            match client.take("bolt", 9).await {
                Err(ClientError::Api {
                    status,
                    code,
                    message,
                }) => assert_eq!(
                    (status, code.as_str(), message.as_str()),
                    (409, "insufficient", "only 4 left"),
                    "the code and the message come from the server's ErrorBody"
                ),
                other => panic!("expected ClientError::Api for a 409, got {other:?}"),
            }
            match client.take("nut", 1).await {
                Err(ClientError::Api { status, code, .. }) => {
                    assert_eq!((status, code.as_str()), (404, "not_found"));
                }
                other => panic!("expected ClientError::Api for a 404, got {other:?}"),
            }
        });
    }

    #[test]
    fn e_sku_is_data_not_url_syntax() {
        run(real_time, || async {
            let state = AppState::default();
            let url = spawn(app(state.clone())).await;
            let client = StockClient::new(url, PATIENCE).unwrap();
            let sku = "m8 bolt/zinc 100%?";
            assert!(
                matches!(client.set(sku, 3).await, Ok(true)),
                "`/`, `?`, `%`, and spaces in a sku must be percent-encoded as one path segment"
            );
            assert_eq!(
                state.quantity(sku),
                Some(3),
                "the server sees the sku unchanged"
            );
            assert!(matches!(client.get(sku).await, Ok(Some(3))));
            assert!(matches!(client.take(sku, 1).await, Ok(2)));
        });
    }

    #[test]
    fn e_silent_server_is_a_timeout() {
        run(real_time, || async {
            // This "server" accepts connections and then never answers.
            let (listener, url) = bind().await;
            tokio::spawn(async move {
                let mut open = Vec::new();
                while let Ok((stream, _)) = listener.accept().await {
                    open.push(stream);
                }
            });
            let client = StockClient::new(url, MS * 200).unwrap();
            let result = tokio::time::timeout(PATIENCE, client.get("bolt"))
                .await
                .expect("the client must give up after its own timeout (200 ms here)");
            assert!(
                matches!(result, Err(ClientError::Timeout)),
                "expected ClientError::Timeout, got {result:?}"
            );
        });
    }

    #[test]
    fn e_other_failures_are_classified() {
        run(real_time, || async {
            // 1. A server that closes every connection without answering.
            let (listener, url) = bind().await;
            tokio::spawn(async move {
                while let Ok((stream, _)) = listener.accept().await {
                    drop(stream);
                }
            });
            let result = StockClient::new(url, PATIENCE).unwrap().get("bolt").await;
            assert!(
                matches!(result, Err(ClientError::Transport(_))),
                "a closed connection is a transport error, got {result:?}"
            );

            // 2. An error status whose body is not an `ErrorBody`.
            let broken = Router::new().route(
                "/stock/{sku}",
                get(|| async { (StatusCode::INTERNAL_SERVER_ERROR, "kaboom") }),
            );
            let client = StockClient::new(spawn(broken).await, PATIENCE).unwrap();
            match client.get("bolt").await {
                Err(ClientError::Api {
                    status,
                    code,
                    message,
                }) => assert_eq!(
                    (status, code.as_str(), message.as_str()),
                    (500, "unknown", "kaboom")
                ),
                other => panic!("expected ClientError::Api for a 500, got {other:?}"),
            }

            // 3. A 404 that does not come from the stock service: this router has no routes.
            let client = StockClient::new(spawn(Router::new()).await, PATIENCE).unwrap();
            let result = client.get("bolt").await;
            assert!(
                matches!(&result, Err(ClientError::Api { status: 404, code, .. }) if code == "unknown"),
                "only a 404 with the code `not_found` means \"no such sku\", got {result:?}"
            );

            // 4. A success status with a body that is not the expected JSON.
            let confused =
                Router::new().route("/stock/{sku}", get(|| async { "<html>hello</html>" }));
            let client = StockClient::new(spawn(confused).await, PATIENCE).unwrap();
            assert!(matches!(
                client.get("bolt").await,
                Err(ClientError::Transport(_))
            ));
        });
    }

    #[test]
    fn f_end_to_end_on_worker_threads() {
        run(worker_threads, || async {
            let state = AppState::default();
            let (listener, url) = bind().await;
            let shutdown = CancellationToken::new();
            let service = with_deadline(app(state.clone()), PATIENCE);
            let server = tokio::spawn(serve(listener, service, shutdown.clone()));

            let client = StockClient::new(url, PATIENCE).unwrap();
            assert!(matches!(client.set("bolt", 10).await, Ok(true)));

            // 25 buyers at once, each on its own task, want one of the 10 bolts.
            let mut buyers = tokio::task::JoinSet::new();
            for _ in 0..25 {
                let client = client.clone();
                buyers.spawn(async move { client.take("bolt", 1).await });
            }
            let (mut sold, mut refused) = (0, 0);
            for result in buyers.join_all().await {
                match result {
                    Ok(_) => sold += 1,
                    Err(ClientError::Api { status: 409, .. }) => refused += 1,
                    Err(other) => panic!("unexpected failure: {other:?}"),
                }
            }
            assert_eq!(
                (sold, refused),
                (10, 15),
                "exactly the 10 bolts in stock may be sold: check and subtract under one lock"
            );
            assert_eq!(state.quantity("bolt"), Some(0));

            shutdown.cancel();
            tokio::time::timeout(PATIENCE, server)
                .await
                .expect("serve must return after the token is cancelled")
                .unwrap()
                .expect("a clean shutdown is Ok(())");
        });
    }

    /// P. Predict BEFORE running: replace each `crate::guess()` call with your answer.
    /// These use their own routes, so they work before your tasks are done.
    #[test]
    fn p_predictions() {
        run(real_time, || async {
            // This handler takes `Json<Quantity>` directly, so axum itself answers
            // when the body cannot be extracted.
            let echo = Router::new().route(
                "/echo",
                put(|Json(body): Json<Quantity>| async move { Json(body) }),
            );
            let status = |method: &'static str, body: &'static str| {
                let echo = echo.clone();
                async move { call_raw(&echo, method, "/echo", body).await.as_u16() }
            };

            // P1. The body is valid JSON with the wrong shape. axum's status code?
            let p1: u16 = crate::guess();
            assert_eq!(
                p1,
                status("PUT", r#"{"quantity":"many"}"#).await,
                "P1: well-formed JSON that does not fit the type"
            );

            // P2. The body is not JSON at all (it stops in the middle). The status code?
            let p2: u16 = crate::guess();
            assert_eq!(
                p2,
                status("PUT", r#"{"quantity":"#).await,
                "P2: a syntax error in the body"
            );

            // P3. A good body, but sent with DELETE to a route that only has PUT?
            let p3: u16 = crate::guess();
            assert_eq!(
                p3,
                status("DELETE", r#"{"quantity":1}"#).await,
                "P3: the path exists, the method does not"
            );

            // P4. A reqwest client sends GET to a path with no route and gets a 404.
            // Is the `Result` of `send().await` an `Err`?
            let url = spawn(echo.clone()).await;
            let sent = http().get(url.join("nowhere").unwrap()).send().await;
            let p4: bool = crate::guess();
            assert_eq!(
                p4,
                sent.is_err(),
                "P4: for reqwest, a response with an error status is still a response"
            );
        });
    }

    /// Like `call`, but returns only the status and accepts any body.
    async fn call_raw(app: &Router, method: &str, path: &str, body: &str) -> StatusCode {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_owned()))
            .unwrap();
        app.clone().oneshot(request).await.unwrap().status()
    }
}
