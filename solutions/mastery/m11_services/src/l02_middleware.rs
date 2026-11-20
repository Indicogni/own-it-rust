//! LESSON 02 — Middleware with tower: timeouts, request ids, limits, compression, order
//! Read first: CONCEPTS.md, "Lesson 02". Start after: lesson 01.
//! Run: cargo test --manifest-path mastery/m11_services/Cargo.toml l02_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: you call `.layer(a)` and then
//! `.layer(b)` on a router. Which of the two sees the request first, and which sees
//! the response first?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! The tests use virtual time (`start_paused = true`), so a "ten second" handler
//! costs nothing and a timeout fires at an exact instant.
//! EXPLAIN afterward: Draw your `hardened` stack as nested boxes. For a request
//! that times out, list which layers still see the response, and say why the
//! response still carries a request id. Then explain why `InFlightLimit` needs a
//! value with a `Drop` impl and cannot just subtract after `.await`.
//! JUDGMENT: Use the ready-made layers from tower-http for standard jobs; they are
//! well tested and handle details you will forget (streaming bodies, header
//! casing). Write `axum::middleware::from_fn` for logic that is specific to your
//! service and needs async code. Write a `tower::Service` by hand only when you
//! need to be generic over the inner service or need `poll_ready`, as in a
//! library. A limit that rejects (this lesson) keeps latency flat under overload;
//! a limit that queues (`tower::limit::ConcurrencyLimitLayer`) keeps success rate
//! high until the queue itself becomes the problem.

use axum::Router;
use axum::extract::Request;
use axum::http::{HeaderName, HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;
use tower::{Layer, Service, ServiceBuilder};
use tower_http::compression::CompressionLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{
    MakeRequestId, PropagateRequestIdLayer, RequestId, SetRequestIdLayer,
};
use tower_http::timeout::TimeoutLayer;

/// The header that carries the request id.
pub const X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

/// A. A source of request ids: `req-1`, `req-2`, ... Clones share one counter, so
/// ids are unique across all clones. (Production services use random ids, for
/// example UUIDs; a counter makes the tests exact.)
#[derive(Debug, Clone, Default)]
pub struct CountingId {
    issued: Arc<AtomicU64>,
}

impl MakeRequestId for CountingId {
    fn make_request_id<B>(&mut self, _request: &axum::http::Request<B>) -> Option<RequestId> {
        let number = self.issued.fetch_add(1, Ordering::Relaxed) + 1;
        let value = HeaderValue::from_str(&format!("req-{number}")).ok()?;
        Some(RequestId::new(value))
    }
}

/// Limits applied by [`hardened`].
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Longest time one request may take, from the first layer to the last byte
    /// of the handler's response head.
    pub timeout: Duration,
    /// Largest accepted request body in bytes.
    pub max_body: usize,
}

/// B. Wrap `routes` with standard tower-http layers:
///
/// - Request id: if the request has no `x-request-id` header, add one from a
///   [`CountingId`]. An id sent by the client is kept. The handler must be able to
///   read the header, and **every** response must carry it, including the 503 and
///   413 responses made by the layers below.
/// - Timeout: a request that takes longer than `limits.timeout` is answered with
///   `503 Service Unavailable`.
/// - Body limit: a request body larger than `limits.max_body` bytes is answered
///   with `413 Payload Too Large`.
/// - Compression: responses are gzip-compressed when the client sends
///   `accept-encoding: gzip`.
///
/// The order of the layers decides whether the second half of the first rule holds.
pub fn hardened(routes: Router, limits: Limits) -> Router {
    // ServiceBuilder lists layers from the outside in: a request passes them top
    // to bottom and the response passes them bottom to top. The two request-id
    // layers are outermost, so they also see responses made by the timeout and
    // the body limit.
    routes.layer(
        ServiceBuilder::new()
            .layer(SetRequestIdLayer::new(X_REQUEST_ID, CountingId::default()))
            .layer(PropagateRequestIdLayer::new(X_REQUEST_ID))
            .layer(CompressionLayer::new())
            .layer(TimeoutLayer::with_status_code(
                StatusCode::SERVICE_UNAVAILABLE,
                limits.timeout,
            ))
            .layer(RequestBodyLimitLayer::new(limits.max_body)),
    )
}

/// A shared, ordered record of what happened. Supplied.
pub type Log = Arc<Mutex<Vec<String>>>;

/// A supplied middleware, written by hand as a `tower::Layer`, that records
/// `"name>"` when the request passes inward and `"<name"` when the response passes
/// outward. Read it before task E: it shows the shape of a hand-written service.
#[derive(Debug, Clone)]
pub struct Tag {
    name: &'static str,
    log: Log,
}

impl Tag {
    pub fn new(name: &'static str, log: &Log) -> Tag {
        Tag {
            name,
            log: Arc::clone(log),
        }
    }
}

impl<S> Layer<S> for Tag {
    type Service = TagService<S>;
    fn layer(&self, inner: S) -> TagService<S> {
        TagService {
            inner,
            tag: self.clone(),
        }
    }
}

/// The service that [`Tag`] produces.
#[derive(Debug, Clone)]
pub struct TagService<S> {
    inner: S,
    tag: Tag,
}

impl<S> Service<Request> for TagService<S>
where
    S: Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Response, S::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), S::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        // `poll_ready` was called on `self.inner`, so that is the instance that
        // must take the call. Leave a fresh clone behind for the next request.
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        let tag = self.tag.clone();
        Box::pin(async move {
            tag.log.lock().expect("log").push(format!("{}>", tag.name));
            let response = inner.call(request).await;
            tag.log.lock().expect("log").push(format!("<{}", tag.name));
            response
        })
    }
}

/// C. Wrap `routes` in three [`Tag`] layers so that a request passes `"trace"`
/// first, then `"auth"`, then `"audit"`, then the handler. Use exactly one
/// `.layer(...)` call with a `tower::ServiceBuilder`.
pub fn ordered_with_builder(routes: Router, log: &Log) -> Router {
    routes.layer(
        ServiceBuilder::new()
            .layer(Tag::new("trace", log))
            .layer(Tag::new("auth", log))
            .layer(Tag::new("audit", log)),
    )
}

/// C. The same order as [`ordered_with_builder`], but with three separate
/// `Router::layer` calls and no `ServiceBuilder`.
pub fn ordered_with_layer_calls(routes: Router, log: &Log) -> Router {
    // Each `.layer` call wraps everything added so far, so the last call is the
    // outermost layer.
    routes
        .layer(Tag::new("audit", log))
        .layer(Tag::new("auth", log))
        .layer(Tag::new("trace", log))
}

/// D. A middleware as a plain async function, for `axum::middleware::from_fn`.
/// Measure how long the rest of the stack takes with `tokio::time::Instant` (the
/// tests use virtual time) and add the header `x-elapsed-ms` with the whole
/// milliseconds as decimal text, for example `x-elapsed-ms: 250`. The header is how
/// a caller sees server time separately from network time.
pub async fn elapsed_header(request: Request, next: Next) -> Response {
    let start = tokio::time::Instant::now();
    let mut response = next.run(request).await;
    let millis = start.elapsed().as_millis();
    response.headers_mut().insert(
        HeaderName::from_static("x-elapsed-ms"),
        HeaderValue::from_str(&millis.to_string()).expect("digits are a valid header value"),
    );
    response
}

/// E. Load shedding, written by hand. At most `max` requests may be inside the
/// inner service at once. A request that arrives when `max` are in flight is not
/// queued: it gets `503 Service Unavailable` with the header `retry-after: 1`
/// at once, and the inner service is not called.
///
/// A request stops being "in flight" when its response future finishes **or is
/// dropped**. Futures are dropped when a client disconnects or an outer timeout
/// fires, so a count that is only decremented after `.await` leaks slots.
///
/// You may change the private fields of both types.
#[derive(Debug, Clone)]
pub struct InFlightLimitLayer {
    max: usize,
    in_flight: Arc<AtomicUsize>,
}

impl InFlightLimitLayer {
    pub fn new(max: usize) -> InFlightLimitLayer {
        InFlightLimitLayer {
            max,
            in_flight: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// How many requests are inside the inner service right now.
    pub fn in_flight(&self) -> usize {
        self.in_flight.load(Ordering::SeqCst)
    }
}

impl<S> Layer<S> for InFlightLimitLayer {
    type Service = InFlightLimit<S>;
    fn layer(&self, inner: S) -> InFlightLimit<S> {
        InFlightLimit {
            inner,
            max: self.max,
            in_flight: Arc::clone(&self.in_flight),
        }
    }
}

/// The service that [`InFlightLimitLayer`] produces.
#[derive(Debug, Clone)]
pub struct InFlightLimit<S> {
    inner: S,
    max: usize,
    in_flight: Arc<AtomicUsize>,
}

/// Holds one slot. Dropping it gives the slot back, whether the request
/// finished, failed, or was cancelled.
struct Slot(Arc<AtomicUsize>);

impl Slot {
    fn try_take(count: &Arc<AtomicUsize>, max: usize) -> Option<Slot> {
        // A compare-and-swap loop: take a slot only if one is free, without a
        // window in which the count exceeds `max`.
        count
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                (current < max).then_some(current + 1)
            })
            .ok()
            .map(|_| Slot(Arc::clone(count)))
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

impl<S> Service<Request> for InFlightLimit<S>
where
    S: Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Response, S::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), S::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let Some(slot) = Slot::try_take(&self.in_flight, self.max) else {
            return Box::pin(async {
                Ok((
                    StatusCode::SERVICE_UNAVAILABLE,
                    [(header::RETRY_AFTER, "1")],
                    "overloaded, try again",
                )
                    .into_response())
            });
        };
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        Box::pin(async move {
            // The slot lives in the future. If the future is dropped half way,
            // the slot is dropped with it.
            let _slot = slot;
            inner.call(request).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::{deadline, get, request, send, with_header};
    use axum::body::{Body, Bytes};
    use axum::http::{HeaderMap, Method};
    use axum::routing::{get as route_get, post};
    use tokio::sync::{Semaphore, mpsc};
    use tokio::time::Instant;

    const LIMITS: Limits = Limits {
        timeout: Duration::from_secs(2),
        max_body: 64,
    };

    /// Routes used to observe the layers.
    fn probe_routes() -> Router {
        Router::new()
            .route(
                "/id",
                route_get(|headers: HeaderMap| async move {
                    headers
                        .get("x-request-id")
                        .and_then(|value| value.to_str().ok())
                        .unwrap_or("handler saw no id")
                        .to_owned()
                }),
            )
            .route(
                "/slow",
                route_get(|| async {
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    "finally"
                }),
            )
            .route(
                "/upload",
                post(|body: Bytes| async move { body.len().to_string() }),
            )
            .route("/big", route_get(|| async { "tick tock ".repeat(400) }))
    }

    fn new_log() -> Log {
        Arc::new(Mutex::new(Vec::new()))
    }

    fn logged(log: &Log) -> Vec<String> {
        std::mem::take(&mut *log.lock().unwrap())
    }

    fn logging_handler(log: &Log) -> Router {
        let log = Arc::clone(log);
        Router::new().route(
            "/",
            route_get(move || {
                let log = Arc::clone(&log);
                async move {
                    log.lock().unwrap().push("handler".to_owned());
                    "ok"
                }
            }),
        )
    }

    #[test]
    fn a_ids_count_up_and_clones_share_the_counter() {
        let mut first = CountingId::default();
        let mut second = first.clone();
        let probe = get("/");
        let next = |maker: &mut CountingId| {
            maker
                .make_request_id(&probe)
                .expect("an id")
                .header_value()
                .to_str()
                .unwrap()
                .to_owned()
        };
        assert_eq!(next(&mut first), "req-1");
        assert_eq!(
            next(&mut second),
            "req-2",
            "a clone must continue the same sequence, or two requests share an id"
        );
        assert_eq!(next(&mut first), "req-3");
    }

    #[tokio::test(start_paused = true)]
    async fn b_request_ids_reach_the_handler_and_the_response() {
        crate::support::deadline(600, "the test `b_request_ids_reach_the_handler_and_the_response`", async {
        let app = hardened(probe_routes(), LIMITS);
        let first = send(&app, get("/id")).await;
        assert_eq!(first.header("x-request-id"), Some("req-1"));
        assert_eq!(
            first.text(),
            "req-1",
            "the handler must see the id in the request headers"
        );
        let second = send(&app, get("/id")).await;
        assert_eq!(second.header("x-request-id"), Some("req-2"));

        let given = send(&app, with_header(get("/id"), "x-request-id", "abc-123")).await;
        assert_eq!(
            given.header("x-request-id"),
            Some("abc-123"),
            "an id sent by the client is kept, so one id follows a call through several services"
        );
        assert_eq!(given.text(), "abc-123");
        })
        .await
    }

    #[tokio::test(start_paused = true)]
    async fn b_slow_requests_time_out_with_503_and_keep_their_id() {
        crate::support::deadline(600, "the test `b_slow_requests_time_out_with_503_and_keep_their_id`", async {
        let app = hardened(probe_routes(), LIMITS);
        let start = Instant::now();
        let response = send(&app, get("/slow")).await;
        assert_eq!(response.code(), 503);
        assert_eq!(
            start.elapsed(),
            Duration::from_secs(2),
            "the timeout must fire at the limit, not when the handler finishes"
        );
        assert_eq!(
            response.header("x-request-id"),
            Some("req-1"),
            "the request-id layers must wrap the timeout layer, or timeouts cannot be traced"
        );
        })
        .await
    }

    #[tokio::test(start_paused = true)]
    async fn b_large_bodies_are_rejected_with_413() {
        crate::support::deadline(
            600,
            "the test `b_large_bodies_are_rejected_with_413`",
            async {
                let app = hardened(probe_routes(), LIMITS);
                let upload = |size: usize, declare_length: bool| {
                    let mut builder = axum::http::Request::builder()
                        .method(Method::POST)
                        .uri("/upload");
                    if declare_length {
                        builder = builder.header(header::CONTENT_LENGTH, size);
                    }
                    builder.body(Body::from(vec![b'x'; size])).unwrap()
                };
                let small = send(&app, upload(64, true)).await;
                assert_eq!((small.code(), small.text().as_str()), (200, "64"));

                let declared = send(&app, upload(65, true)).await;
                assert_eq!(declared.code(), 413, "content-length over the limit");
                assert_eq!(
                    declared.header("x-request-id"),
                    Some("req-2"),
                    "413 responses carry the request id too"
                );

                let undeclared = send(&app, upload(65, false)).await;
                assert_eq!(
                    undeclared.code(),
                    413,
                    "the limit must also hold when the client does not announce the length"
                );
            },
        )
        .await
    }

    #[tokio::test(start_paused = true)]
    async fn b_responses_are_compressed_on_request() {
        crate::support::deadline(
            600,
            "the test `b_responses_are_compressed_on_request`",
            async {
                let app = hardened(probe_routes(), LIMITS);
                let plain = send(&app, get("/big")).await;
                assert_eq!(plain.header("content-encoding"), None);
                assert_eq!(plain.body.len(), 4000);

                let zipped = send(&app, with_header(get("/big"), "accept-encoding", "gzip")).await;
                assert_eq!(zipped.code(), 200);
                assert_eq!(zipped.header("content-encoding"), Some("gzip"));
                assert_eq!(&zipped.body[..2], &[0x1f, 0x8b], "gzip magic number");
                assert!(
                    zipped.body.len() < 400,
                    "4000 repetitive bytes must shrink a lot, got {}",
                    zipped.body.len()
                );
                assert_eq!(zipped.header("x-request-id"), Some("req-2"));
            },
        )
        .await
    }

    const EXPECTED_ORDER: [&str; 7] = [
        "trace>", "auth>", "audit>", "handler", "<audit", "<auth", "<trace",
    ];

    #[tokio::test]
    async fn c_builder_order() {
        crate::support::deadline(120, "the test `c_builder_order`", async {
            let log = new_log();
            let app = ordered_with_builder(logging_handler(&log), &log);
            assert_eq!(send(&app, get("/")).await.code(), 200);
            assert_eq!(logged(&log), EXPECTED_ORDER);
        })
        .await
    }

    #[tokio::test]
    async fn c_layer_call_order() {
        crate::support::deadline(120, "the test `c_layer_call_order`", async {
            let log = new_log();
            let app = ordered_with_layer_calls(logging_handler(&log), &log);
            assert_eq!(send(&app, get("/")).await.code(), 200);
            assert_eq!(logged(&log), EXPECTED_ORDER);
        })
        .await
    }

    #[tokio::test(start_paused = true)]
    async fn d_elapsed_header_reports_whole_milliseconds() {
        crate::support::deadline(
            600,
            "the test `d_elapsed_header_reports_whole_milliseconds`",
            async {
                let app = Router::new()
                    .route(
                        "/nap",
                        route_get(|| async {
                            tokio::time::sleep(Duration::from_millis(250)).await;
                            "rested"
                        }),
                    )
                    .route("/now", route_get(|| async { "instant" }))
                    .layer(axum::middleware::from_fn(elapsed_header));
                let nap = send(&app, get("/nap")).await;
                assert_eq!(nap.header("x-elapsed-ms"), Some("250"));
                assert_eq!(nap.text(), "rested", "the body must pass through unchanged");
                assert_eq!(
                    send(&app, get("/now")).await.header("x-elapsed-ms"),
                    Some("0")
                );
            },
        )
        .await
    }

    /// A route that reports when a request has entered and then waits for a permit.
    fn holding_routes(entered: mpsc::UnboundedSender<()>, gate: Arc<Semaphore>) -> Router {
        Router::new().route(
            "/hold",
            route_get(move || {
                let (entered, gate) = (entered.clone(), Arc::clone(&gate));
                async move {
                    entered.send(()).expect("test is listening");
                    let _permit = gate.acquire().await.expect("gate open");
                    "done"
                }
            }),
        )
    }

    #[tokio::test(start_paused = true)]
    async fn e_extra_requests_are_shed_not_queued() {
        crate::support::deadline(
            600,
            "the test `e_extra_requests_are_shed_not_queued`",
            async {
                let (entered_tx, mut entered) = mpsc::unbounded_channel();
                let gate = Arc::new(Semaphore::new(0));
                let limit = InFlightLimitLayer::new(2);
                let app = holding_routes(entered_tx, Arc::clone(&gate)).layer(limit.clone());

                let mut held = tokio::task::JoinSet::new();
                for _ in 0..2 {
                    let app = app.clone();
                    held.spawn(async move { send(&app, get("/hold")).await });
                }
                for _ in 0..2 {
                    deadline(5, "a held request entering the handler", entered.recv()).await;
                }
                assert_eq!(limit.in_flight(), 2);

                let start = Instant::now();
                let shed = deadline(5, "the third request", send(&app, get("/hold"))).await;
                assert_eq!(shed.code(), 503, "the third request must be rejected");
                assert_eq!(shed.header("retry-after"), Some("1"));
                assert_eq!(
                    start.elapsed(),
                    Duration::ZERO,
                    "rejected at once, not queued"
                );
                assert!(
                    entered.try_recv().is_err(),
                    "a rejected request must not reach the handler"
                );
                assert_eq!(limit.in_flight(), 2, "a rejected request holds no slot");

                gate.add_permits(2);
                let finished = deadline(5, "the held requests", held.join_all()).await;
                assert!(finished.iter().all(|response| response.code() == 200));
                assert_eq!(
                    limit.in_flight(),
                    0,
                    "finished requests give their slots back"
                );

                gate.add_permits(1);
                assert_eq!(
                    send(&app, get("/hold")).await.code(),
                    200,
                    "capacity is back"
                );
            },
        )
        .await
    }

    #[tokio::test(start_paused = true)]
    async fn e_cancelled_requests_release_their_slot() {
        crate::support::deadline(600, "the test `e_cancelled_requests_release_their_slot`", async {
        let (entered_tx, mut entered) = mpsc::unbounded_channel();
        let gate = Arc::new(Semaphore::new(0));
        let limit = InFlightLimitLayer::new(1);
        let app = holding_routes(entered_tx, Arc::clone(&gate)).layer(limit.clone());

        let request_task = {
            let app = app.clone();
            tokio::spawn(async move { send(&app, get("/hold")).await })
        };
        deadline(5, "the request entering the handler", entered.recv()).await;
        assert_eq!(limit.in_flight(), 1);

        // The client goes away: the response future is dropped before it finishes.
        request_task.abort();
        assert!(request_task.await.unwrap_err().is_cancelled());
        assert_eq!(
            limit.in_flight(),
            0,
            "a dropped request must free its slot: release in a Drop impl, not after `.await`"
        );

        gate.add_permits(1);
        let next = deadline(5, "the next request", send(&app, get("/hold"))).await;
        assert_eq!(next.code(), 200, "the service must not stay full forever");
        })
        .await
    }

    #[tokio::test(start_paused = true)]
    async fn e_works_under_an_outer_timeout() {
        crate::support::deadline(600, "the test `e_works_under_an_outer_timeout`", async {
            let (entered_tx, _entered) = mpsc::unbounded_channel();
            let gate = Arc::new(Semaphore::new(0));
            let limit = InFlightLimitLayer::new(1);
            let app = holding_routes(entered_tx, gate).layer(
                ServiceBuilder::new()
                    .layer(TimeoutLayer::with_status_code(
                        StatusCode::SERVICE_UNAVAILABLE,
                        Duration::from_secs(1),
                    ))
                    .layer(limit.clone()),
            );
            let timed_out = send(&app, request(Method::GET, "/hold")).await;
            assert_eq!(timed_out.code(), 503);
            assert_eq!(
                limit.in_flight(),
                0,
                "the timeout dropped the inner future; its slot must be free again"
            );
        })
        .await
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[tokio::test]
    async fn p_predictions() {
        crate::support::deadline(120, "the test `p_predictions`", async {
            // P1. Two `.layer` calls. Write the four log entries in order, for example
            // vec!["a>", "b>", "<b", "<a"].
            let log = new_log();
            let app = Router::new()
                .route("/", route_get(|| async { "ok" }))
                .layer(Tag::new("a", &log))
                .layer(Tag::new("b", &log));
            send(&app, get("/")).await;
            let p1: Vec<&str> = vec!["b>", "a>", "<a", "<b"];
            assert_eq!(
                p1,
                logged(&log),
                "P1: the last .layer call is the outermost"
            );

            // P2. `.layer` wraps the routes that exist when it is called. How many log
            // entries does a request to "/late" produce?
            let app = Router::new()
                .route("/early", route_get(|| async { "ok" }))
                .layer(Tag::new("a", &log))
                .route("/late", route_get(|| async { "ok" }));
            send(&app, get("/late")).await;
            let p2: usize = 0;
            assert_eq!(p2, logged(&log).len(), "P2: a route added after .layer");

            // P3. No route matches "/missing". With `.layer`, how many log entries does
            // that request produce?
            let app = Router::new()
                .route("/", route_get(|| async { "ok" }))
                .layer(Tag::new("a", &log));
            let response = send(&app, get("/missing")).await;
            assert_eq!(response.code(), 404);
            let p3: usize = 2;
            assert_eq!(p3, logged(&log).len(), "P3: .layer and the 404 fallback");

            // P4. The same request, but the router uses `.route_layer` instead.
            let app = Router::new()
                .route("/", route_get(|| async { "ok" }))
                .route_layer(Tag::new("a", &log));
            let response = send(&app, get("/missing")).await;
            assert_eq!(response.code(), 404);
            let p4: usize = 0;
            assert_eq!(
                p4,
                logged(&log).len(),
                "P4: .route_layer and the 404 fallback"
            );
        })
        .await
    }
}
