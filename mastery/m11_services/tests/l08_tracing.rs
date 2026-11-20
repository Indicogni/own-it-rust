//! Lesson 08, task D: request spans.
//! Run: cargo test --manifest-path mastery/m11_services/Cargo.toml --test l08_tracing
//!
//! This file holds exactly one test, on purpose. `tracing` remembers per call
//! site whether any subscriber is interested, and that memory is shared by the
//! whole process. A second test with another subscriber, running in parallel,
//! could make events disappear from this one. One test per process avoids that.

use axum::Router;
use axum::extract::Path;
use axum::routing::get;
use m11_services::l08_operations::trace_requests;
use m11_services::support::{
    Capture, deadline, get as http_get, plain_subscriber, send, with_header,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Semaphore, mpsc};

#[tokio::test]
async fn request_logs_carry_their_own_span() {
    deadline(120, "the test `request_logs_carry_their_own_span`", async {
    let capture = Capture::default();
    // Thread-local default subscriber; the test runtime has one thread, so every
    // task spawned below logs into `capture`.
    let _subscriber = tracing::subscriber::set_default(plain_subscriber(&capture));

    // Each request reports its arrival, logs, waits for its own gate, logs again.
    let (arrived_tx, mut arrived) = mpsc::unbounded_channel::<String>();
    let gates: Arc<HashMap<&str, Arc<Semaphore>>> = Arc::new(
        ["a", "b"]
            .into_iter()
            .map(|name| (name, Arc::new(Semaphore::new(0))))
            .collect(),
    );
    let handler_gates = Arc::clone(&gates);
    let app = Router::new()
        .route(
            "/jobs/{name}",
            get(move |Path(name): Path<String>| {
                let (arrived, gates) = (arrived_tx.clone(), Arc::clone(&handler_gates));
                async move {
                    tracing::info!(job = %name, "started");
                    arrived.send(name.clone()).expect("test is listening");
                    let gate = gates.get(name.as_str()).expect("job a or b");
                    let _permit = gate.acquire().await.expect("gate open");
                    tracing::info!(job = %name, "resumed");
                    "ok"
                }
            }),
        )
        .layer(axum::middleware::from_fn(trace_requests));

    // Request A starts and waits. Request B starts and waits. Then A resumes
    // while B is still waiting, and at last B resumes.
    let call = |uri: &'static str, id: &'static str| {
        let app = app.clone();
        tokio::spawn(async move { send(&app, with_header(http_get(uri), "x-request-id", id)).await })
    };
    let mut first = call("/jobs/a?token=do-not-log-me", "id-A");
    tokio::select! {
        name = deadline(10, "request A arriving", arrived.recv()) => assert_eq!(name.as_deref(), Some("a")),
        early = &mut first => panic!("request A ended before it reached the handler: {early:?}"),
    }
    let mut second = call("/jobs/b", "id-B");
    tokio::select! {
        name = deadline(10, "request B arriving", arrived.recv()) => assert_eq!(name.as_deref(), Some("b")),
        early = &mut second => panic!("request B ended before it reached the handler: {early:?}"),
    }
    gates["a"].add_permits(1);
    assert_eq!(deadline(10, "request A", first).await.unwrap().code(), 200);
    gates["b"].add_permits(1);
    assert_eq!(deadline(10, "request B", second).await.unwrap().code(), 200);

    // A request without an id, to a path that matches no route.
    assert_eq!(send(&app, http_get("/nowhere")).await.code(), 404);

    let lines = capture.lines();
    let text = lines.join("\n");
    println!("captured log:\n{text}");

    let span_a = "request{method=GET path=/jobs/a request_id=id-A}";
    let span_b = "request{method=GET path=/jobs/b request_id=id-B}";
    let line_with = |parts: &[&str]| {
        lines
            .iter()
            .find(|line| parts.iter().all(|part| line.contains(part)))
            .unwrap_or_else(|| panic!("no log line contains all of {parts:?}. Log:\n{text}"))
    };

    for (span, job) in [(span_a, "job=a"), (span_b, "job=b")] {
        line_with(&[span, "started", job]);
        line_with(&[span, "resumed", job]);
        let finished = line_with(&[span, "request finished", "status=200"]);
        assert!(finished.contains("INFO"), "the summary is an INFO event: {finished}");
    }
    line_with(&["request{method=GET path=/nowhere request_id=-}", "request finished", "status=404"]);

    for line in &lines {
        assert_eq!(
            line.matches("request{").count(),
            1,
            "every line belongs to exactly one request span, but this one does not:\n  {line}\n\
             A span guard from `enter()` that is held across `.await` stays active on \
             the thread while another request runs. Use `.instrument(span)`."
        );
    }
    assert!(
        !text.contains("do-not-log-me"),
        "the query string must not be logged; it can carry secrets. Log:\n{text}"
    );
    assert_eq!(
        lines.len(),
        7,
        "two handler events and one summary per job request, one summary for the 404. Log:\n{text}"
    );
    })
    .await
}
