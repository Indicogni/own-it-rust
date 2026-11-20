//! LESSON 09 — Testing services: without a socket, over loopback, fixtures, faults
//! Read first: CONCEPTS.md, "Lesson 09". Start after: lessons 01, 03 and 08;
//! ecosystem E05 (testing your tests with mutants) helps.
//! Run: cargo test --manifest-path mastery/m11_services/Cargo.toml l09_
//!
//! In lessons 01 to 08 you used helpers from `support.rs` to call routers. Now you
//! write such helpers yourself, and then you write the tests. The service under
//! test is supplied: a "tool crib" that lends tools, backed by SQLite. It can be
//! built correctly or with one of four realistic defects. Your checks pass this
//! lesson when they accept the correct service **and** catch the defect they are
//! meant for. A check that cannot fail proves nothing.
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: a handler test calls the router in
//! memory and passes. Name two kinds of bug that it cannot see and that a test
//! over a real socket can.
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! EXPLAIN afterward: Which of your tests need a socket, and why do the others
//! not? Why does every test get its own database? How did you make the database
//! fail on purpose, and what did that test tell you that no "happy path" test
//! could?
//! JUDGMENT: Most service tests should call the router in memory: they are fast,
//! exact and need no ports. Keep a few full-stack tests for what only a socket
//! shows (HTTP parsing, headers the server adds, timeouts, shutdown). Use the
//! real database engine in tests when you can, as here; a hand-written fake of a
//! database tends to agree with your code and disagree with production. For
//! PostgreSQL, `#[sqlx::test]` gives each test its own database, and
//! `testcontainers` starts a server in Docker. Fault injection through a trait
//! (a failing clock, a failing mail sender) is simpler than through the database;
//! use the simplest switch that makes the failure real.
//!
//! The crib's HTTP contract (supplied, correct when built with `Flaw::None`):
//!
//! | Method and path           | Success                 | Errors                                      |
//! | ------------------------- | ----------------------- | ------------------------------------------- |
//! | `POST /tools`             | 201, the tool           | 422 `invalid` (blank name), 409 `duplicate` |
//! | `GET /tools/{id}`         | 200, the tool           | 404 `not_found`                             |
//! | `POST /tools/{id}/borrow` | 200, the tool, borrowed | 404 `not_found`, 409 `already_borrowed`     |
//!
//! A tool is `{"id": 1, "name": "drill", "borrower": null}`; `borrower` is the
//! name from the body `{"who": "..."}` while it is lent out. Error bodies are
//! `{"error": "<code>"}`. If the database fails, the answer is 500 with exactly
//! `{"error": "internal"}`: no detail leaves the server.

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::SqlitePool;
use sqlx::sqlite::SqlitePoolOptions;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// The service under test. Supplied: read it, do not edit it.
// ---------------------------------------------------------------------------

/// The crib's schema, embedded from `migrations/l09/`.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations/l09");

/// One deliberate defect, or none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flaw {
    /// The correct service.
    None,
    /// `POST /tools` answers 200 instead of 201.
    CreatedIs200,
    /// `POST /tools` stores a tool whose name is blank.
    AcceptsBlankNames,
    /// A database failure puts the database's own error text in the response.
    LeaksErrorDetails,
    /// Borrowing runs its two statements without a transaction.
    BorrowWithoutTransaction,
}

#[derive(Clone)]
struct Crib {
    pool: SqlitePool,
    flaw: Flaw,
}

enum CribError {
    Client(StatusCode, &'static str),
    Database(sqlx::Error),
}

impl From<sqlx::Error> for CribError {
    fn from(error: sqlx::Error) -> CribError {
        CribError::Database(error)
    }
}

#[derive(Deserialize)]
struct NewTool {
    name: String,
}

#[derive(Deserialize)]
struct Borrow {
    who: String,
}

fn tool_json(row: (i64, String, Option<String>)) -> Value {
    json!({ "id": row.0, "name": row.1, "borrower": row.2 })
}

/// Builds the crib on `pool`, which must have [`MIGRATOR`] applied.
pub fn crib(pool: SqlitePool, flaw: Flaw) -> Router {
    Router::new()
        .route("/tools", post(create_tool))
        .route("/tools/{id}", get(get_tool))
        .route("/tools/{id}/borrow", post(borrow_tool))
        .with_state(Crib { pool, flaw })
}

fn respond(crib: &Crib, result: Result<Response, CribError>) -> Response {
    match result {
        Ok(response) => response,
        Err(CribError::Client(status, code)) => {
            (status, Json(json!({ "error": code }))).into_response()
        }
        Err(CribError::Database(error)) => {
            let body = if crib.flaw == Flaw::LeaksErrorDetails {
                json!({ "error": "internal", "detail": error.to_string() })
            } else {
                json!({ "error": "internal" })
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(body)).into_response()
        }
    }
}

async fn create_tool(State(crib): State<Crib>, Json(new): Json<NewTool>) -> Response {
    let result = async {
        let name = new.name.trim().to_owned();
        if name.is_empty() && crib.flaw != Flaw::AcceptsBlankNames {
            return Err(CribError::Client(
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid",
            ));
        }
        let inserted =
            sqlx::query_as("INSERT INTO tool (name) VALUES (?1) RETURNING id, name, borrower")
                .bind(&name)
                .fetch_one(&crib.pool)
                .await;
        let row: (i64, String, Option<String>) = match inserted {
            Ok(row) => row,
            Err(sqlx::Error::Database(db)) if db.is_unique_violation() => {
                return Err(CribError::Client(StatusCode::CONFLICT, "duplicate"));
            }
            Err(other) => return Err(other.into()),
        };
        let status = if crib.flaw == Flaw::CreatedIs200 {
            StatusCode::OK
        } else {
            StatusCode::CREATED
        };
        Ok((status, Json(tool_json(row))).into_response())
    }
    .await;
    respond(&crib, result)
}

async fn fetch_tool(pool: &SqlitePool, id: i64) -> Result<Value, CribError> {
    let row = sqlx::query_as("SELECT id, name, borrower FROM tool WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    row.map(tool_json)
        .ok_or(CribError::Client(StatusCode::NOT_FOUND, "not_found"))
}

async fn get_tool(State(crib): State<Crib>, Path(id): Path<i64>) -> Response {
    let result = fetch_tool(&crib.pool, id)
        .await
        .map(|tool| Json(tool).into_response());
    respond(&crib, result)
}

async fn borrow_tool(
    State(crib): State<Crib>,
    Path(id): Path<i64>,
    Json(borrow): Json<Borrow>,
) -> Response {
    const MARK: &str = "UPDATE tool SET borrower = ?1 WHERE id = ?2 AND borrower IS NULL";
    const LOG: &str = "INSERT INTO loan (tool_id, who) VALUES (?1, ?2)";
    let result = async {
        fetch_tool(&crib.pool, id).await?;
        let marked = if crib.flaw == Flaw::BorrowWithoutTransaction {
            let marked = sqlx::query(MARK)
                .bind(&borrow.who)
                .bind(id)
                .execute(&crib.pool)
                .await?;
            if marked.rows_affected() == 1 {
                sqlx::query(LOG)
                    .bind(id)
                    .bind(&borrow.who)
                    .execute(&crib.pool)
                    .await?;
            }
            marked
        } else {
            let mut tx = crib.pool.begin().await?;
            let marked = sqlx::query(MARK)
                .bind(&borrow.who)
                .bind(id)
                .execute(&mut *tx)
                .await?;
            if marked.rows_affected() == 1 {
                sqlx::query(LOG)
                    .bind(id)
                    .bind(&borrow.who)
                    .execute(&mut *tx)
                    .await?;
            }
            tx.commit().await?;
            marked
        };
        if marked.rows_affected() == 0 {
            return Err(CribError::Client(StatusCode::CONFLICT, "already_borrowed"));
        }
        Ok(Json(fetch_tool(&crib.pool, id).await?).into_response())
    }
    .await;
    respond(&crib, result)
}

/// Fault injection, supplied: after this call, every `INSERT INTO loan` on this
/// database fails with an error. Nothing else is affected.
pub async fn break_loan_log(pool: &SqlitePool) {
    sqlx::query(
        "CREATE TRIGGER loan_log_is_broken BEFORE INSERT ON loan \
         BEGIN SELECT RAISE(ABORT, 'injected fault: the loan log is broken'); END",
    )
    .execute(pool)
    .await
    .expect("installing the fault trigger");
}

// ---------------------------------------------------------------------------
// Your work starts here.
// ---------------------------------------------------------------------------

/// A. Call a router in memory, without a socket: send `request` to a clone of
/// `app` (`tower::ServiceExt::oneshot`), read the whole body, and return the
/// status with the body as JSON.
///
/// - An empty body gives `Value::Null`.
/// - A body that is not JSON gives `Value::String` with the body text, so a test
///   can still show what came back.
pub async fn call(app: &Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("a Router never returns an error");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("reading the body of an in-memory response");
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes)
            .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()))
    };
    (status, body)
}

/// A. A request with a JSON body and the right content type. `method` is for
/// example `"POST"`.
pub fn json_request(method: &str, uri: &str, body: &Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("a valid request")
}

/// A. A `GET` request without a body.
pub fn get_request(uri: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .body(Body::empty())
        .expect("a valid request")
}

/// B. A database fixture: a new, private, in-memory database with [`MIGRATOR`]
/// applied and exactly two tools in it: `drill` (id 1) and `saw` (id 2), both not
/// borrowed. Every call gives a database that no other test can see, so tests can
/// run in parallel and in any order. Use a pool with a single connection.
pub async fn fixture() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        // An in-memory database lives as long as its connection: keep it.
        .idle_timeout(None)
        .max_lifetime(None)
        .connect("sqlite::memory:")
        .await
        .expect("opening an in-memory database");
    MIGRATOR.run(&pool).await.expect("migrations");
    sqlx::query("INSERT INTO tool (name) VALUES ('drill'), ('saw')")
        .execute(&pool)
        .await
        .expect("seed rows");
    pool
}

/// C. A router that is really listening, for full-stack tests. You may change the
/// private fields.
pub struct Running {
    addr: SocketAddr,
    task: JoinHandle<()>,
}

impl Running {
    /// C. Bind `127.0.0.1:0` (the operating system picks a free port, so tests
    /// never fight over one) and serve `app` on a background task.
    pub async fn start(app: Router) -> Running {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("binding a loopback port");
        let addr = listener.local_addr().expect("the listener has an address");
        let task = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        Running { addr, task }
    }

    /// The address the server listens on.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// C. `GET http://ADDR{path}` over a real TCP connection, with the same
    /// result shape as [`call`]. Use `crate::support::http` as the client.
    pub async fn get(&self, path: &str) -> (StatusCode, Value) {
        let request = get_request(&format!("http://{}{path}", self.addr));
        let response = crate::support::http(request)
            .await
            .expect("the server is listening");
        let body = if response.body.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&response.body)
                .unwrap_or_else(|_| Value::String(response.text()))
        };
        (response.status, body)
    }
}

/// C. Stop the server when the value is dropped, so a failing test leaves no
/// task (and no open port) behind.
impl Drop for Running {
    fn drop(&mut self) {
        // Aborting the task drops the listener, which closes the port.
        self.task.abort();
    }
}

/// D. A check of the contract of `POST /tools`. Return `Ok(())` if `app` behaves
/// as the header says and `Err(description)` if not. Do not panic: the tests run
/// this check against broken services and expect an `Err`.
///
/// `app` is a crib on a fresh [`fixture`]. Cover at least: the success status and
/// body, a blank name, and a duplicate name.
pub async fn check_create(app: &Router) -> Result<(), String> {
    let (status, body) = call(
        app,
        json_request("POST", "/tools", &json!({"name": "plane"})),
    )
    .await;
    expect(status, 201, &body, "creating a tool")?;
    let wanted = json!({ "id": 3, "name": "plane", "borrower": null });
    if body != wanted {
        return Err(format!("created tool: expected {wanted}, got {body}"));
    }
    for blank in ["", "   "] {
        let request = json_request("POST", "/tools", &json!({ "name": blank }));
        let (status, body) = call(app, request).await;
        expect(status, 422, &body, "a blank name")?;
        expect_error(&body, "invalid")?;
    }
    let (status, body) = call(
        app,
        json_request("POST", "/tools", &json!({"name": "drill"})),
    )
    .await;
    expect(status, 409, &body, "a duplicate name")?;
    expect_error(&body, "duplicate")?;
    // Rejected requests must not have stored anything: the next id is still 4.
    let (status, body) = call(app, get_request("/tools/4")).await;
    expect(status, 404, &body, "a tool that was never created")
}

fn expect(status: StatusCode, wanted: u16, body: &Value, what: &str) -> Result<(), String> {
    if status.as_u16() == wanted {
        Ok(())
    } else {
        Err(format!(
            "{what}: expected status {wanted}, got {status} with body {body}"
        ))
    }
}

fn expect_error(body: &Value, code: &str) -> Result<(), String> {
    let wanted = json!({ "error": code });
    if *body == wanted {
        Ok(())
    } else {
        Err(format!("expected the body {wanted}, got {body}"))
    }
}

/// D. A check that a database failure stays opaque. `app` is a crib on `pool`.
/// Make the database unavailable (closing the pool does that: `pool.close()`),
/// call the service, and require status 500 with exactly `{"error": "internal"}`.
/// Same rules as [`check_create`]: `Ok` or `Err`, no panic.
pub async fn check_failures_are_opaque(app: &Router, pool: &SqlitePool) -> Result<(), String> {
    pool.close().await;
    let attempts = [
        get_request("/tools/1"),
        json_request("POST", "/tools", &json!({ "name": "plane" })),
        json_request("POST", "/tools/1/borrow", &json!({ "who": "ada" })),
    ];
    for request in attempts {
        let what = format!(
            "{} {} with the database down",
            request.method(),
            request.uri()
        );
        let (status, body) = call(app, request).await;
        expect(status, 500, &body, &what)?;
        // Comparing the whole body catches any extra field, whatever its name.
        expect_error(&body, "internal").map_err(|problem| format!("{what}: {problem}"))?;
    }
    Ok(())
}

/// D. A check that borrowing is all or nothing. `app` is a crib on `pool`, a
/// fresh [`fixture`]. Use [`break_loan_log`] to make the second statement of a
/// borrowing fail, try to borrow, and then look at the tool: a borrowing that
/// failed must leave it on its hook. Same rules: `Ok` or `Err`, no panic.
pub async fn check_borrow_is_atomic(app: &Router, pool: &SqlitePool) -> Result<(), String> {
    // First the normal case, so a broken service is not reported as "atomic".
    let borrow = |id: u32, who: &str| {
        json_request(
            "POST",
            &format!("/tools/{id}/borrow"),
            &json!({ "who": who }),
        )
    };
    let (status, body) = call(app, borrow(2, "lin")).await;
    expect(status, 200, &body, "borrowing a free tool")?;
    if body["borrower"] != "lin" {
        return Err(format!(
            "the borrowed tool should name its borrower, got {body}"
        ));
    }

    break_loan_log(pool).await;
    let (status, body) = call(app, borrow(1, "ada")).await;
    expect(status, 500, &body, "borrowing while the loan log is broken")?;
    let (status, body) = call(app, get_request("/tools/1")).await;
    expect(
        status,
        200,
        &body,
        "reading the tool after the failed borrowing",
    )?;
    if !body["borrower"].is_null() {
        return Err(format!(
            "the borrowing failed, but the tool is marked as borrowed: {body}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::deadline;
    use std::time::Duration;
    use tokio::net::TcpStream;

    async fn fresh(flaw: Flaw) -> (Router, SqlitePool) {
        let pool = deadline(10, "fixture()", fixture()).await;
        (crib(pool.clone(), flaw), pool)
    }

    #[tokio::test]
    async fn a_call_returns_status_and_json() {
        crate::support::deadline(120, "the test `a_call_returns_status_and_json`", async {
            let app = Router::new()
                .route("/json", get(|| async { Json(json!({ "n": 1 })) }))
                .route("/empty", get(|| async { StatusCode::NO_CONTENT }))
                .route(
                    "/text",
                    get(|| async { (StatusCode::IM_A_TEAPOT, "short and stout") }),
                )
                .route(
                    "/echo",
                    post(
                        |Json(body): Json<Value>| async move { (StatusCode::CREATED, Json(body)) },
                    ),
                );
            assert_eq!(
                call(&app, get_request("/json")).await,
                (StatusCode::OK, json!({ "n": 1 }))
            );
            assert_eq!(
                call(&app, get_request("/empty")).await,
                (StatusCode::NO_CONTENT, Value::Null),
                "an empty body is Null"
            );
            assert_eq!(
                call(&app, get_request("/text")).await,
                (StatusCode::IM_A_TEAPOT, json!("short and stout")),
                "a body that is not JSON comes back as a string"
            );
            let body = json!({ "who": "ada", "tags": [1, 2] });
            assert_eq!(
                call(&app, json_request("POST", "/echo", &body)).await,
                (StatusCode::CREATED, body),
                "json_request must send the body and the JSON content type"
            );
            assert_eq!(
                call(&app, get_request("/missing")).await.0,
                StatusCode::NOT_FOUND,
                "`app` is still usable: call works on a clone"
            );
        })
        .await
    }

    #[tokio::test]
    async fn b_fixture_is_seeded_and_private() {
        crate::support::deadline(120, "the test `b_fixture_is_seeded_and_private`", async {
            let first = deadline(10, "fixture()", fixture()).await;
            let second = deadline(10, "fixture()", fixture()).await;
            let tools = async |pool: &SqlitePool| -> Vec<(i64, String, Option<String>)> {
                sqlx::query_as("SELECT id, name, borrower FROM tool ORDER BY id")
                    .fetch_all(pool)
                    .await
                    .expect("the tool table exists: run the migrations in the fixture")
            };
            let seed = vec![(1, "drill".to_owned(), None), (2, "saw".to_owned(), None)];
            assert_eq!(tools(&first).await, seed);

            sqlx::query("DELETE FROM tool")
                .execute(&first)
                .await
                .unwrap();
            assert_eq!(
                tools(&second).await,
                seed,
                "another fixture must not see this change: every test gets its own database"
            );
            let loans: i64 = sqlx::query_scalar("SELECT count(*) FROM loan")
                .fetch_one(&second)
                .await
                .unwrap();
            assert_eq!(loans, 0);
            assert_eq!(second.options().get_max_connections(), 1);
        })
        .await
    }

    #[tokio::test]
    async fn c_running_serves_on_an_os_chosen_port_and_stops_on_drop() {
        crate::support::deadline(
            120,
            "the test `c_running_serves_on_an_os_chosen_port_and_stops_on_drop`",
            async {
                let (app, _pool) = fresh(Flaw::None).await;
                let one = deadline(10, "Running::start", Running::start(app.clone())).await;
                let two = deadline(10, "Running::start", Running::start(app)).await;
                assert!(one.addr().ip().is_loopback());
                assert_ne!(one.addr().port(), 0, "report the port the system chose");
                assert_ne!(
                    one.addr().port(),
                    two.addr().port(),
                    "two servers at once need two ports: bind port 0, never a fixed one"
                );

                let (status, body) = deadline(10, "GET over TCP", one.get("/tools/1")).await;
                assert_eq!(status, StatusCode::OK);
                assert_eq!(body, json!({ "id": 1, "name": "drill", "borrower": null }));
                let (status, _) = deadline(10, "GET over TCP", two.get("/tools/99")).await;
                assert_eq!(status, StatusCode::NOT_FOUND);

                let addr = one.addr();
                drop(one);
                let refused = async {
                    while TcpStream::connect(addr).await.is_ok() {
                        tokio::time::sleep(Duration::from_millis(2)).await;
                    }
                };
                deadline(10, "the port closing after drop", refused).await;
                let (status, _) = deadline(10, "the other server", two.get("/tools/2")).await;
                assert_eq!(
                    status,
                    StatusCode::OK,
                    "dropping one server leaves the other alone"
                );
            },
        )
        .await
    }

    /// Runs a check and insists on a useful failure text.
    fn caught(result: Result<(), String>, flaw: Flaw) {
        match result {
            Ok(()) => panic!("the check accepted a service with the defect {flaw:?}"),
            Err(text) => assert!(
                text.trim().len() >= 10,
                "the failure text should say what was wrong, got {text:?}"
            ),
        }
    }

    #[tokio::test]
    async fn d_check_create_accepts_the_correct_service() {
        crate::support::deadline(
            120,
            "the test `d_check_create_accepts_the_correct_service`",
            async {
                let (app, _pool) = fresh(Flaw::None).await;
                assert_eq!(
                    deadline(10, "check_create", check_create(&app)).await,
                    Ok(())
                );
            },
        )
        .await
    }

    #[tokio::test]
    async fn d_check_create_catches_its_mutants() {
        crate::support::deadline(
            120,
            "the test `d_check_create_catches_its_mutants`",
            async {
                for flaw in [Flaw::CreatedIs200, Flaw::AcceptsBlankNames] {
                    let (app, _pool) = fresh(flaw).await;
                    caught(deadline(10, "check_create", check_create(&app)).await, flaw);
                }
            },
        )
        .await
    }

    #[tokio::test]
    async fn d_check_failures_are_opaque_accepts_and_catches() {
        crate::support::deadline(
            120,
            "the test `d_check_failures_are_opaque_accepts_and_catches`",
            async {
                let (app, pool) = fresh(Flaw::None).await;
                let verdict =
                    deadline(10, "the check", check_failures_are_opaque(&app, &pool)).await;
                assert_eq!(verdict, Ok(()));

                let flaw = Flaw::LeaksErrorDetails;
                let (app, pool) = fresh(flaw).await;
                caught(
                    deadline(10, "the check", check_failures_are_opaque(&app, &pool)).await,
                    flaw,
                );
            },
        )
        .await
    }

    #[tokio::test]
    async fn d_check_borrow_is_atomic_accepts_and_catches() {
        crate::support::deadline(
            120,
            "the test `d_check_borrow_is_atomic_accepts_and_catches`",
            async {
                let (app, pool) = fresh(Flaw::None).await;
                let verdict = deadline(10, "the check", check_borrow_is_atomic(&app, &pool)).await;
                assert_eq!(verdict, Ok(()));
                let logged: i64 = sqlx::query_scalar("SELECT count(*) FROM loan")
                    .fetch_one(&pool)
                    .await
                    .unwrap();
                assert!(
                    logged <= 1,
                    "the check should make the loan log fail; use `break_loan_log`"
                );

                let flaw = Flaw::BorrowWithoutTransaction;
                let (app, pool) = fresh(flaw).await;
                caught(
                    deadline(10, "the check", check_borrow_is_atomic(&app, &pool)).await,
                    flaw,
                );
            },
        )
        .await
    }

    #[tokio::test]
    async fn d_checks_do_not_raise_false_alarms() {
        crate::support::deadline(
            120,
            "the test `d_checks_do_not_raise_false_alarms`",
            async {
                // A defect in one place must not make an unrelated check fail; otherwise
                // a red test does not tell you where to look.
                let (app, _pool) = fresh(Flaw::LeaksErrorDetails).await;
                assert_eq!(check_create(&app).await, Ok(()));
                let (app, pool) = fresh(Flaw::CreatedIs200).await;
                assert_eq!(check_borrow_is_atomic(&app, &pool).await, Ok(()));
                let (app, pool) = fresh(Flaw::AcceptsBlankNames).await;
                assert_eq!(check_failures_are_opaque(&app, &pool).await, Ok(()));
            },
        )
        .await
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// These use supplied helpers, so they do not depend on your code.
    #[tokio::test]
    async fn p_predictions() {
        crate::support::deadline(120, "the test `p_predictions`", async {
            use crate::support::{TestServer, get as support_get, http, send};
            let app = Router::new().route("/hi", get(|| async { "hi" })).route(
                "/len",
                post(|body: axum::body::Bytes| async move { body.len().to_string() }),
            );

            // P1. hyper adds a `date` header to responses it writes to a socket. Is
            // that header present (a) on a response from an in-memory call, (b) on a
            // response received over TCP?
            let in_memory = send(&app, support_get("/hi")).await;
            let server = TestServer::start(app.clone()).await;
            let over_tcp = http(support_get(&server.url("/hi"))).await.unwrap();
            let p1: (bool, bool) = (false, true);
            assert_eq!(
                p1,
                (
                    in_memory.headers.contains_key("date"),
                    over_tcp.headers.contains_key("date")
                ),
                "P1: what the HTTP layer adds"
            );

            // P2. axum's `Bytes` extractor refuses bodies over 2 MiB by default. The
            // request below carries 2 MiB + 1 byte and goes to the router in memory;
            // no HTTP parser is involved. Status code?
            let big = Request::builder()
                .method("POST")
                .uri("/len")
                .body(Body::from(vec![0u8; 2 * 1024 * 1024 + 1]))
                .unwrap();
            let p2: u16 = 413;
            assert_eq!(
                p2,
                send(&app, big).await.code(),
                "P2: the body limit lives in the extractor"
            );

            // P3. Two pools are opened from "sqlite::memory:", each with one
            // connection. A table is created through the first. Does a query on that
            // table through the second pool succeed?
            let open = async || {
                SqlitePoolOptions::new()
                    .max_connections(1)
                    .connect("sqlite::memory:")
                    .await
                    .unwrap()
            };
            let (first, second) = (open().await, open().await);
            sqlx::query("CREATE TABLE t (n INTEGER)")
                .execute(&first)
                .await
                .unwrap();
            let p3: bool = false;
            assert_eq!(
                p3,
                sqlx::query("SELECT n FROM t")
                    .fetch_all(&second)
                    .await
                    .is_ok(),
                "P3: in-memory databases are private to their pool"
            );
        })
        .await
    }
}
