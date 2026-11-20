//! CODE REVIEW 01 — A teammate's pull request: the field-notes service
//! Run: cargo test --manifest-path mastery/m11_services/Cargo.toml c01_
//! Then: cargo clippy --manifest-path mastery/m11_services/Cargo.toml --all-targets
//!
//! This is the kind of code you will be asked to review at work. It compiles, the
//! demo worked, and one of its tests already passes. It also has six defects of
//! the kind that pass a demo and fail in production. Your job:
//! 1. Before running anything, read the code and write a review in NOTES.md: for
//!    each defect, what an attacker or a busy day would do with it.
//! 2. Run the tests. Compare what they found with your review. (Three tests wait
//!    about five seconds each before they fail; that is the defect showing.)
//! 3. Fix it. You MAY change everything except the items marked "Keep": the
//!    private fields of `NotesState`, the handlers, the error type, the layers.
//! 4. Run clippy and resolve its warnings for this file.
//!
//! The contract the reviewer expects (the pull request description promised it):
//!
//! | Method and path            | Success                                          |
//! | -------------------------- | ------------------------------------------------ |
//! | `POST /notes`              | 201, `{"id", "author", "text"}`                  |
//! | `GET /notes`               | 200, all notes in id order                       |
//! | `GET /notes?author=NAME`   | 200, the notes of exactly that author            |
//! | `GET /notes/{id}/summary`  | 200, `{"id", "summary"}` from the summariser     |
//! | `GET /notes/export`        | 200, the exporter's text for all notes           |
//! | `GET /stats`               | 200, `{"summaries": n}`: summaries served so far |
//!
//! - Errors are JSON `{"error": "<code>"}`: 404 `not_found`, 504
//!   `upstream_timeout`, 500 `internal`. No other detail leaves the server.
//! - A request body over [`MAX_BODY_BYTES`] is refused with 413.
//! - The summariser is another service. It may be slow or never answer; a request
//!   waits for it at most [`SUMMARY_TIMEOUT`] and then answers 504.
//! - The exporter is an old synchronous library call that may take a long time.
//! - Requests do not wait for each other without need.

use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::SqlitePool;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Largest accepted request body. Keep.
pub const MAX_BODY_BYTES: usize = 64 * 1024;
/// Longest wait for the summariser. Keep.
pub const SUMMARY_TIMEOUT: Duration = Duration::from_millis(300);

/// The summariser: an async call to another service. Keep.
pub type Summarizer =
    Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = String> + Send>> + Send + Sync>;
/// The exporter: a synchronous, possibly slow library call. Keep.
pub type Exporter = Arc<dyn Fn(Vec<String>) -> String + Send + Sync>;

#[derive(Debug, Default)]
struct Stats {
    summaries: u64,
}

/// Shared state. Keep the name and `new`; the private fields are yours.
#[derive(Clone)]
pub struct NotesState {
    pool: SqlitePool,
    summarize: Summarizer,
    export: Exporter,
    // The lock is only ever held for one increment or one read, never across an
    // `.await`, so the standard library's mutex is the right one.
    stats: Arc<Mutex<Stats>>,
}

impl NotesState {
    /// Keep this signature.
    pub fn new(pool: SqlitePool, summarize: Summarizer, export: Exporter) -> NotesState {
        NotesState {
            pool,
            summarize,
            export,
            stats: Arc::new(Mutex::new(Stats::default())),
        }
    }
}

/// Creates the `note` table. Keep.
pub async fn create_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS note (
            id     INTEGER PRIMARY KEY,
            author TEXT NOT NULL,
            text   TEXT NOT NULL
        ) STRICT",
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(Debug, Deserialize)]
struct NewNote {
    author: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct NoteFilter {
    author: Option<String>,
}

/// What a client may learn about a failure: a code, and nothing else.
#[derive(Debug)]
enum Failure {
    NotFound,
    UpstreamTimeout,
    Internal,
}

impl IntoResponse for Failure {
    fn into_response(self) -> Response {
        let (status, code) = match self {
            Failure::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            Failure::UpstreamTimeout => (StatusCode::GATEWAY_TIMEOUT, "upstream_timeout"),
            Failure::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "internal"),
        };
        (status, Json(json!({ "error": code }))).into_response()
    }
}

/// The details go to the log, where operators can read them; the client gets
/// only `internal`.
fn failed(error: impl std::fmt::Display) -> Failure {
    tracing::error!(%error, "request failed");
    Failure::Internal
}

async fn create_note(
    State(state): State<NotesState>,
    Json(new): Json<NewNote>,
) -> Result<(StatusCode, Json<Value>), Failure> {
    let done = sqlx::query("INSERT INTO note (author, text) VALUES (?1, ?2)")
        .bind(&new.author)
        .bind(&new.text)
        .execute(&state.pool)
        .await
        .map_err(failed)?;
    let body = json!({
        "id": done.last_insert_rowid(),
        "author": new.author,
        "text": new.text,
    });
    Ok((StatusCode::CREATED, Json(body)))
}

async fn list_notes(
    State(state): State<NotesState>,
    Query(filter): Query<NoteFilter>,
) -> Result<Json<Value>, Failure> {
    // One fixed SQL text. The author travels as a bind parameter, so no value
    // can ever change the meaning of the statement. `?1 IS NULL` makes the
    // filter optional without building SQL at run time.
    let rows: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT id, author, text FROM note WHERE ?1 IS NULL OR author = ?1 ORDER BY id",
    )
    .bind(filter.author)
    .fetch_all(&state.pool)
    .await
    .map_err(failed)?;
    let notes = rows
        .into_iter()
        .map(|(id, author, text)| json!({ "id": id, "author": author, "text": text }))
        .collect();
    Ok(Json(Value::Array(notes)))
}

async fn summarize_note(
    State(state): State<NotesState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, Failure> {
    let text: Option<String> = sqlx::query_scalar("SELECT text FROM note WHERE id = ?1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(failed)?;
    let text = text.ok_or(Failure::NotFound)?;
    // Another service may never answer; this request must.
    let summary = tokio::time::timeout(SUMMARY_TIMEOUT, (state.summarize)(text))
        .await
        .map_err(|_| Failure::UpstreamTimeout)?;
    // Lock, add one, unlock: no `.await` while the guard is alive.
    state.stats.lock().map_err(failed)?.summaries += 1;
    Ok(Json(json!({ "id": id, "summary": summary })))
}

async fn export_notes(State(state): State<NotesState>) -> Result<String, Failure> {
    let texts: Vec<String> = sqlx::query_scalar("SELECT text FROM note ORDER BY id")
        .fetch_all(&state.pool)
        .await
        .map_err(failed)?;
    let export = Arc::clone(&state.export);
    // A synchronous call of unknown length belongs on the blocking pool, not on
    // a thread that also serves every other request.
    tokio::task::spawn_blocking(move || export(texts))
        .await
        .map_err(failed)
}

async fn stats(State(state): State<NotesState>) -> Result<Json<Value>, Failure> {
    let summaries = state.stats.lock().map_err(failed)?.summaries;
    Ok(Json(json!({ "summaries": summaries })))
}

/// The router. Keep the name and signature.
pub fn app(state: NotesState) -> Router {
    Router::new()
        .route("/notes", post(create_note).get(list_notes))
        .route("/notes/export", get(export_notes))
        .route("/notes/{id}/summary", get(summarize_note))
        .route("/stats", get(stats))
        // A limit that was "in the way" needs a number, not removal.
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::{deadline, get as http_get, post_json, send};
    use axum::body::Body;
    use axum::http::Request;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::sync::mpsc as std_mpsc;
    use std::time::Instant;
    use tokio::sync::{Semaphore, mpsc};

    /// A summariser that answers at once with the first word.
    fn quick_summarizer() -> Summarizer {
        Arc::new(|text: String| {
            Box::pin(async move { text.split_whitespace().next().unwrap_or("").to_owned() })
        })
    }

    /// An exporter that is fast.
    fn quick_exporter() -> Exporter {
        Arc::new(|texts: Vec<String>| format!("{} notes: {}", texts.len(), texts.join(" | ")))
    }

    async fn pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        create_schema(&pool).await.unwrap();
        pool
    }

    /// The service with three notes: two by ada, one by O'Brien.
    async fn seeded(summarize: Summarizer, export: Exporter) -> (Router, SqlitePool) {
        let pool = pool().await;
        let app = app(NotesState::new(pool.clone(), summarize, export));
        for (author, text) in [
            ("ada", "Valve 3 sticks when cold"),
            ("O'Brien", "Replaced the north gasket"),
            ("ada", "Gauge 7 reads high"),
        ] {
            let body = json!({ "author": author, "text": text });
            let response = send(&app, post_json("/notes", &body)).await;
            assert_eq!(response.code(), 201, "seeding failed: {}", response.text());
        }
        (app, pool)
    }

    fn ids(body: &Value) -> Vec<i64> {
        body.as_array()
            .expect("a JSON array of notes")
            .iter()
            .map(|note| note["id"].as_i64().expect("numeric id"))
            .collect()
    }

    #[tokio::test]
    async fn a_the_demo_works() {
        crate::support::deadline(120, "the test `a_the_demo_works`", async {
            let (app, _pool) = seeded(quick_summarizer(), quick_exporter()).await;
            let created = send(
                &app,
                post_json(
                    "/notes",
                    &json!({ "author": "lin", "text": "Pump 2 is loud" }),
                ),
            )
            .await;
            assert_eq!(created.code(), 201);
            assert_eq!(
                created.json(),
                json!({ "id": 4, "author": "lin", "text": "Pump 2 is loud" })
            );

            let all = send(&app, http_get("/notes")).await;
            assert_eq!(ids(&all.json()), [1, 2, 3, 4]);
            let by_ada = send(&app, http_get("/notes?author=ada")).await;
            assert_eq!(ids(&by_ada.json()), [1, 3]);
            assert_eq!(by_ada.json()[0]["text"], "Valve 3 sticks when cold");

            let summary = send(&app, http_get("/notes/3/summary")).await;
            assert_eq!(summary.json(), json!({ "id": 3, "summary": "Gauge" }));
            assert_eq!(
                send(&app, http_get("/stats")).await.json(),
                json!({ "summaries": 1 })
            );
            assert_eq!(send(&app, http_get("/notes/99/summary")).await.code(), 404);

            let export = send(&app, http_get("/notes/export")).await;
            assert_eq!(export.code(), 200);
            assert!(
                export.text().starts_with("4 notes: Valve 3"),
                "{}",
                export.text()
            );
        })
        .await
    }

    #[tokio::test]
    async fn b_the_author_filter_treats_its_value_as_data() {
        crate::support::deadline(
            120,
            "the test `b_the_author_filter_treats_its_value_as_data`",
            async {
                let (app, _pool) = seeded(quick_summarizer(), quick_exporter()).await;

                // The classic: a value that would be SQL if it were pasted into the query.
                let attack = send(&app, http_get("/notes?author=x%27%20OR%20%271%27%3D%271")).await;
                assert_eq!(attack.code(), 200, "{}", attack.text());
                assert_eq!(
                    ids(&attack.json()),
                    [] as [i64; 0],
                    "nobody is called `x' OR '1'='1`, so the result is empty. This request \
             got other people's notes: the author value was read as SQL"
                );

                // And the honest user who has an apostrophe in the name.
                let honest = send(&app, http_get("/notes?author=O%27Brien")).await;
                assert_eq!(
                    honest.code(),
                    200,
                    "an apostrophe is a normal character: {}",
                    honest.text()
                );
                assert_eq!(ids(&honest.json()), [2]);

                let nobody = send(&app, http_get("/notes?author=")).await;
                assert_eq!(
                    ids(&nobody.json()),
                    [] as [i64; 0],
                    "an empty name matches no author"
                );
            },
        )
        .await
    }

    #[tokio::test]
    async fn c_failures_tell_the_client_a_code_and_nothing_else() {
        crate::support::deadline(
            120,
            "the test `c_failures_tell_the_client_a_code_and_nothing_else`",
            async {
                let (app, pool) = seeded(quick_summarizer(), quick_exporter()).await;
                let missing = send(&app, http_get("/notes/99/summary")).await;
                assert_eq!(missing.code(), 404);
                assert_eq!(missing.json(), json!({ "error": "not_found" }));

                // The database goes away.
                pool.close().await;
                let requests = [
                    http_get("/notes"),
                    http_get("/notes/1/summary"),
                    http_get("/notes/export"),
                    post_json("/notes", &json!({ "author": "ada", "text": "x" })),
                ];
                for request in requests {
                    let what = format!("{} {}", request.method(), request.uri());
                    let response = send(&app, request).await;
                    assert_eq!(response.code(), 500, "{what}");
                    assert_eq!(
                        response.text(),
                        r#"{"error":"internal"}"#,
                        "{what}: the body shows internal details. Log them; do not send them"
                    );
                }
            },
        )
        .await
    }

    #[tokio::test]
    async fn d_oversized_bodies_are_refused() {
        crate::support::deadline(120, "the test `d_oversized_bodies_are_refused`", async {
            let (app, _pool) = seeded(quick_summarizer(), quick_exporter()).await;
            let note_of_size = |bytes: usize| {
                let frame = json!({ "author": "ada", "text": "" }).to_string();
                let text = "x".repeat(bytes - frame.len());
                let body = json!({ "author": "ada", "text": text }).to_string();
                assert_eq!(body.len(), bytes);
                Request::builder()
                    .method("POST")
                    .uri("/notes")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap()
            };
            let at_limit = send(&app, note_of_size(MAX_BODY_BYTES)).await;
            assert_eq!(at_limit.code(), 201, "a body of exactly the limit is fine");
            let over = send(&app, note_of_size(MAX_BODY_BYTES + 1)).await;
            assert_eq!(
                over.code(),
                413,
                "one byte over the limit must be refused; without a limit, one client \
             can fill the server's memory"
            );
            let huge = send(&app, note_of_size(4 * 1024 * 1024)).await;
            assert_eq!(huge.code(), 413);
            assert_eq!(ids(&send(&app, http_get("/notes")).await.json()).len(), 4);
        })
        .await
    }

    #[tokio::test]
    async fn e_the_export_does_not_block_the_async_thread() {
        crate::support::deadline(
            120,
            "the test `e_the_export_does_not_block_the_async_thread`",
            async {
                // This test has one runtime thread. The exporter says that it has started
                // and then waits (at most 5 s) for an answer from another async task. If
                // the handler calls the exporter directly, that task never gets to run.
                let (started_tx, mut started) = mpsc::unbounded_channel::<()>();
                let (answer_tx, answer) = std_mpsc::channel::<()>();
                let answer = std::sync::Mutex::new(answer);
                let exporter: Exporter = Arc::new(move |texts: Vec<String>| {
                    let _ = started_tx.send(());
                    let alive = answer
                        .lock()
                        .unwrap()
                        .recv_timeout(Duration::from_secs(5))
                        .is_ok();
                    if alive {
                        format!("exported {}", texts.len())
                    } else {
                        "the runtime was starved".to_owned()
                    }
                });
                let (app, _pool) = seeded(quick_summarizer(), exporter).await;
                let helper = tokio::spawn(async move {
                    if started.recv().await.is_some() {
                        let _ = answer_tx.send(());
                    }
                });
                let export = deadline(
                    30,
                    "GET /notes/export",
                    send(&app, http_get("/notes/export")),
                )
                .await;
                assert_eq!(
                    export.text(),
                    "exported 3",
                    "while the exporter ran, no other task could run: a slow synchronous \
             call must leave the async thread"
                );
                helper.await.unwrap();
            },
        )
        .await
    }

    #[tokio::test]
    async fn f_a_summariser_that_never_answers_gives_504() {
        crate::support::deadline(
            120,
            "the test `f_a_summariser_that_never_answers_gives_504`",
            async {
                let hanging: Summarizer =
                    Arc::new(|_text: String| Box::pin(std::future::pending()));
                let (app, _pool) = seeded(hanging, quick_exporter()).await;
                let start = Instant::now();
                let request = tokio::time::timeout(
                    Duration::from_secs(5),
                    send(&app, http_get("/notes/1/summary")),
                );
                let response = request.await.expect(
                    "the request was still waiting after 5 s: a call to another service \
             needs a timeout, or one stuck dependency stops this service too",
                );
                assert_eq!(response.code(), 504);
                assert_eq!(response.json(), json!({ "error": "upstream_timeout" }));
                assert!(
                    start.elapsed() >= SUMMARY_TIMEOUT,
                    "wait for the full timeout first"
                );
                assert_eq!(
                    send(&app, http_get("/stats")).await.json(),
                    json!({ "summaries": 0 }),
                    "a summary that was never delivered does not count"
                );
            },
        )
        .await
    }

    #[tokio::test]
    async fn g_slow_summaries_do_not_queue_behind_each_other() {
        crate::support::deadline(
            120,
            "the test `g_slow_summaries_do_not_queue_behind_each_other`",
            async {
                // A summariser that reports each call and then waits for a permit.
                let (entered_tx, mut entered) = mpsc::unbounded_channel::<()>();
                let gate = Arc::new(Semaphore::new(0));
                let summarizer: Summarizer = {
                    let gate = Arc::clone(&gate);
                    Arc::new(move |text: String| {
                        let (entered, gate) = (entered_tx.clone(), Arc::clone(&gate));
                        Box::pin(async move {
                            let _ = entered.send(());
                            let _permit = gate.acquire().await.expect("gate open");
                            text.len().to_string()
                        })
                    })
                };
                let (app, _pool) = seeded(summarizer, quick_exporter()).await;

                let call = |uri: &'static str| {
                    let app = app.clone();
                    tokio::spawn(async move { send(&app, http_get(uri)).await })
                };
                let first = call("/notes/1/summary");
                deadline(5, "the first summary call", entered.recv()).await;
                let second = call("/notes/2/summary");
                let both_inside =
                    tokio::time::timeout(Duration::from_secs(5), entered.recv()).await;
                assert!(
                    both_inside.is_ok(),
                    "the second request did not reach the summariser while the first was \
             waiting there: something makes requests wait for each other. Look \
             for a lock that is held across an `.await`"
                );
                // While both wait, an unrelated endpoint must still answer.
                let stats =
                    tokio::time::timeout(Duration::from_secs(5), send(&app, http_get("/stats")));
                let stats = stats
                    .await
                    .expect("GET /stats hung while summaries were in progress");
                assert_eq!(stats.json(), json!({ "summaries": 0 }));

                gate.add_permits(2);
                assert_eq!(deadline(5, "summary one", first).await.unwrap().code(), 200);
                assert_eq!(
                    deadline(5, "summary two", second).await.unwrap().code(),
                    200
                );
                assert_eq!(
                    send(&app, http_get("/stats")).await.json(),
                    json!({ "summaries": 2 })
                );
            },
        )
        .await
    }
}
