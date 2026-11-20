# Mastery M11 concepts — Rust as a service

[Module guide](README.md) · [Hints](HINTS.md) · [Notes workbook](NOTES.md)

Read one section, then do the lesson with the same number. Each section gives you the mental model, one worked example that is *not* the exercise, the mistakes people really make, and links to primary sources.

The code blocks in this file are compiled and run by `cargo test --manifest-path mastery/m11_services/Cargo.toml --doc`. Blocks marked as failing must fail to compile. Compiler messages are copied from Rust 1.99.0 with the crate versions pinned in this module, and shortened.

## What you will be able to do

Build a network service in Rust that a team could run: an HTTP API with typed errors and a consistent contract, a middleware stack with timeouts and limits, a SQL database with migrations and transactions, password login with tokens, CPU-heavy work that does not freeze the server, a binary protocol between two programs, logs, metrics, health checks and a clean shutdown, and tests for all of it.

## Before you start

The whole core course, and from the ecosystem track: E01 (errors), E02 (serde), E04 (tracing) and E06 (tokio). This module does not explain `async`/`await`, `Result`, traits or serde again. If you want to know what a runtime does inside, mastery module M4 builds one; here you use tokio as a tool.

## Time

These are estimates for a learner who meets the prerequisites. Measure your own pace on lesson 01 and scale the rest.

| Lesson | Reading | Exercise |
| --- | --- | --- |
| 01 · Service anatomy | 25 min | 2 h |
| 02 · Middleware with tower | 25 min | 2.5 h |
| 03 · Persistence | 30 min | 3 h |
| 04 · Authentication | 30 min | 2.5 h |
| 05 · Validation and API design | 25 min | 3.5 h |
| 06 · CPU-bound work | 25 min | 2.5 h |
| 07 · A binary protocol (two parts) | 35 min | 4.5 h |
| 08 · Observability and operations | 35 min | 4 h, plus 1.5 h for the container lab |
| 09 · Testing services | 20 min | 3 h |
| C01 · Code review | none | 1.5 h |
| Capstone · beacon | 15 min | 10 to 16 h |

## Lesson 01 · Service anatomy

A web service is a function from a request to a response, called many times at once. Everything else is plumbing around that function. In axum the plumbing has four parts, and a request passes them in this order:

```text
bytes on a socket
   │  hyper parses HTTP/1.1
   ▼
Router        picks a route by path, then by method      → 404 or 405 if nothing fits
   ▼
extractors    one per handler parameter, left to right   → each may reject (400, 415, 422, ...)
   ▼
handler       your async fn; runs only if every extractor succeeded
   ▼
IntoResponse  turns the return value into status, headers and body
```

**Handlers** are ordinary `async fn`s. Their parameters are **extractors**: types that know how to take something out of the request. `Path` reads path segments, `Query` the query string, `Json` the body, `State` the shared state. An extractor that fails answers the request itself, and your handler body does not run. That is why half of the status codes of a service are produced by code you did not write, and why this lesson's predictions ask about them.

**State** is whatever all requests share: a database pool, a cache, configuration. You give it to the router once with `with_state`, and axum clones it for every request. So the state type must be cheap to clone, which in practice means "an `Arc` inside". Data that changes sits behind a lock inside that `Arc`.

**Errors** deserve one type per service. A handler returns `Result<T, E>` where both sides implement `IntoResponse`. If `E` is your own enum, the mapping from "what went wrong" to "which status and which body" lives in exactly one place, and `?` works in every handler.

### Worked example

A greeting service with a visit counter and one typed error:

```rust
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tower::ServiceExt; // for `oneshot`

#[derive(Clone, Default)]
struct Visits(Arc<AtomicU64>);

enum GreetError {
    NameTooLong,
}

impl IntoResponse for GreetError {
    fn into_response(self) -> Response {
        match self {
            GreetError::NameTooLong => (StatusCode::BAD_REQUEST, "name too long").into_response(),
        }
    }
}

async fn greet(
    State(visits): State<Visits>,
    Path(name): Path<String>,
) -> Result<String, GreetError> {
    if name.chars().count() > 20 {
        return Err(GreetError::NameTooLong);
    }
    let number = visits.0.fetch_add(1, Ordering::Relaxed) + 1;
    Ok(format!("Hello {name}, you are visitor {number}"))
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let app = Router::new()
        .route("/greet/{name}", get(greet))
        .with_state(Visits::default());

    // Call the router in memory. No socket is involved.
    let request = axum::http::Request::get("/greet/Ada")
        .body(axum::body::Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
    assert_eq!(&body[..], b"Hello Ada, you are visitor 1");
}
```

The router is a value. You can call it without a network, which is how most tests in this module work. `axum::serve(listener, app)` is the one line that connects it to a socket.

### Mistakes people really make

**The body extractor is not the last parameter.** The request body can be read only once, so the extractor that consumes it (`Json`, `String`, `Bytes`) must come last. The error does not say so:

```rust,compile_fail,E0277
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};

#[derive(Clone)]
struct AppState;

async fn create(Json(name): Json<String>, State(_state): State<AppState>) -> String {
    name
}

fn main() {
    let _app: Router = Router::new().route("/users", post(create)).with_state(AppState);
}
```

```text
error[E0277]: the trait bound `fn(Json<String>, State<AppState>) -> impl Future<Output = String> {create}: Handler<_, _>` is not satisfied
   --> src/main.rs:13:59
    |
 13 |     let _app: Router = Router::new().route("/users", post(create)).with_state(AppState);
    |                                                      ---- ^^^^^^ the trait `Handler<_, _>` is not implemented for fn item `fn(Json<String>, State<AppState>) -> impl Future<Output = String> {create}`
    |                                                      |
    |                                                      required by a bound introduced by this call
    |
    = note: Consider using `#[axum::debug_handler]` to improve the error message
```

Follow the note. With `#[axum::debug_handler(state = AppState)]` on the function, the compiler points at the real cause:

```text
error: `Json<_>` consumes the request body and thus must be the last argument to the handler function
 --> src/main.rs:9:29
  |
9 | async fn create(Json(name): Json<String>, State(_state): State<AppState>) -> String {
  |                             ^^^^
```

You get the same E0277 when a handler returns a type that does not implement `IntoResponse`, or when its future is not `Send`. `debug_handler` explains each of them. Remove the attribute again when the code compiles.

**A standard-library lock is held across `.await`.** tokio may move a task to another thread at every `.await`, so everything alive at that point must be `Send`. A `std::sync::MutexGuard` is not:

```rust,compile_fail
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[tokio::main]
async fn main() {
    let hits = Arc::new(Mutex::new(0u64));
    let task = tokio::spawn(async move {
        let mut guard = hits.lock().unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        *guard += 1;
    });
    task.await.unwrap();
}
```

```text
error: future cannot be sent between threads safely
   --> src/main.rs:7:16
    |
    = help: within `{async block@src/main.rs:7:29: 7:39}`, the trait `Send` is not implemented for `std::sync::MutexGuard<'_, u64>`
note: future is not `Send` as this value is used across an await
   --> src/main.rs:9:55
    |
  8 |         let mut guard = hits.lock().unwrap();
    |             --------- has type `std::sync::MutexGuard<'_, u64>` which is not `Send`
  9 |         tokio::time::sleep(Duration::from_millis(10)).await;
    |                                                       ^^^^^ await occurs here, with `mut guard` maybe used later
```

The fix is almost never "use tokio's `Mutex`". The fix is to end the guard's scope before the `.await`: lock, copy out or change what you need, unlock, then wait. Lesson C01 shows what the other choice costs.

**State is created inside the handler.** A handler that builds its own `HashMap` or opens its own database connection works in a demo and shares nothing. State is built once, in `main`, and handed to the router.

### When to use which

- `std::sync::Mutex` for short critical sections without `.await` (almost always). `RwLock` when reads greatly outnumber writes and are slow enough to matter. `tokio::sync::Mutex` only when you truly must hold the lock while waiting.
- `State<T>` for what every handler of a router needs. Request extensions for data that a middleware computes per request.
- Return `impl IntoResponse` for simple handlers, and `Result<T, YourError>` as soon as a handler can fail in more than one way.

### Read more

- [axum: handlers, extractors, responses](https://docs.rs/axum/0.8.9/axum/#handlers)
- [axum: the order of extractors](https://docs.rs/axum/0.8.9/axum/extract/index.html#the-order-of-extractors)
- [`Send` and `Sync` in the Rustonomicon](https://doc.rust-lang.org/nomicon/send-and-sync.html)
- [`std::sync::Mutex`](https://doc.rust-lang.org/std/sync/struct.Mutex.html)

## Lesson 02 · Middleware with tower

Many jobs are the same for every route: give up after two seconds, refuse huge bodies, attach an id, compress the answer. Such code does not belong in handlers. It belongs in **middleware**: code that wraps the whole request-to-response function.

The tower crate defines the shape. A `Service` is a value with a `call` method that takes a request and returns a future of a response. A `Layer` is a function from a service to a new service that wraps it. A router with three layers looks like an onion:

```text
request ─► ┌ request id ──────────────────────────────┐
           │ ┌ timeout ─────────────────────────────┐ │
           │ │ ┌ body limit ──────────────────────┐ │ │
           │ │ │            handler               │ │ │
           │ │ └──────────────────────────────────┘ │ │
           │ └──────────────────────────────────────┘ │
response ◄─└──────────────────────────────────────────┘
```

A request goes in through every layer and the response comes back out through the same layers in reverse. A layer can act before the inner service (check a header), after it (add a header), around it (measure time), or instead of it (answer 503 without calling it). Two consequences decide most questions about order:

1. A layer sees only responses that pass through it. If the timeout layer is *outside* the request-id layer, a timed-out response has no id.
2. A layer that gives up drops the inner future. Rust then runs the destructors of everything inside. Cleanup that must always happen belongs in a `Drop` impl, not in code after an `.await`.

There are three ways to write middleware, from little work to much:

- Use a ready-made layer from **tower-http** (`TimeoutLayer`, `RequestBodyLimitLayer`, `CompressionLayer`, `SetRequestIdLayer`, `TraceLayer`, ...).
- Write an async function and wrap it with `axum::middleware::from_fn`. It receives the request and a `Next` that stands for the rest of the stack.
- Implement `tower::Service` by hand. You need that for libraries, or when you care about `poll_ready`.

### Worked example

A maintenance switch as a `from_fn` middleware with state. While the switch is on, every request gets 503 and the handler is not called:

```rust
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tower::ServiceExt;

async fn maintenance(State(on): State<Arc<AtomicBool>>, request: Request, next: Next) -> Response {
    if on.load(Ordering::Relaxed) {
        return (StatusCode::SERVICE_UNAVAILABLE, "back soon").into_response();
    }
    let mut response = next.run(request).await; // the rest of the stack
    response.headers_mut().insert("x-served-by", "example".parse().unwrap());
    response
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let switch = Arc::new(AtomicBool::new(false));
    let app = Router::new()
        .route("/", get(|| async { "hello" }))
        .layer(middleware::from_fn_with_state(Arc::clone(&switch), maintenance));
    let get_root = || axum::http::Request::get("/").body(axum::body::Body::empty()).unwrap();

    let open = app.clone().oneshot(get_root()).await.unwrap();
    assert_eq!(open.status(), StatusCode::OK);
    assert_eq!(open.headers()["x-served-by"], "example");

    switch.store(true, Ordering::Relaxed);
    let closed = app.clone().oneshot(get_root()).await.unwrap();
    assert_eq!(closed.status(), StatusCode::SERVICE_UNAVAILABLE);
}
```

### Mistakes people really make

**The order is the reverse of what was meant.** `Router::layer` wraps everything added so far, so the *last* call is the *outermost* layer. `tower::ServiceBuilder` lists layers the other way round: the *first* is outermost, which reads like the picture above. Two layers `a` and `b` that log on the way in and on the way out give, for `.layer(a).layer(b)`:

```text
b>  a>  handler  <a  <b
```

and for `ServiceBuilder::new().layer(a).layer(b)`:

```text
a>  b>  handler  <b  <a
```

Use one `ServiceBuilder` per router and read it top to bottom.

**A route is added after the layer.** `.layer` applies to the routes that exist at that moment. A route added later is not wrapped, and nothing warns you. Add routes first, layers last.

**A hand-written service calls a clone that was never asked whether it is ready.** tower's contract is: call `poll_ready` on a service, and then `call` on *the same* value. A `call` method that needs to move the inner service into a future must therefore swap: leave a fresh clone in `self` and take the instance that reported ready. The supplied `TagService` in the lesson shows the two lines.

**Cleanup after `.await`.** Code like "add one, `.await` the inner service, subtract one" loses the subtraction whenever the future is dropped: a client disconnects, an outer timeout fires. Task E exists to make you feel this.

### When to use which

- `.layer` wraps every route and the fallback. `.route_layer` wraps only matched routes; use it for authentication, so that an unknown path still answers 404 and not 401.
- A timeout that answers the client belongs near the outside. A limit on what one request may cost (body size, concurrency) belongs near the handler.
- Reject (load shedding) when late answers are useless to the caller. Queue when the caller would rather wait.

### Read more

- [`tower::Service`](https://docs.rs/tower/0.5.3/tower/trait.Service.html) and [`tower::ServiceBuilder`](https://docs.rs/tower/0.5.3/tower/struct.ServiceBuilder.html)
- [axum: middleware, with the ordering rules](https://docs.rs/axum/0.8.9/axum/middleware/index.html)
- [tower-http: the list of layers](https://docs.rs/tower-http/0.7.1/tower_http/)
- [`Drop`](https://doc.rust-lang.org/std/ops/trait.Drop.html)

## Lesson 03 · Persistence

A service keeps its facts in a database, and the database outlives every version of the service. Five ideas carry this lesson.

**A pool.** Opening a database connection is slow, and a database serves only so many. A pool keeps a few connections open and lends one to each query. If all are lent out, the next caller waits, and after the acquire timeout it gets an error. The pool size is therefore a limit on how much database work your service does at once, and an exhausted pool is a normal overload signal, not a crash.

**Statements with parameters.** SQL text and values travel separately: the text has placeholders (`?1`, `?2`), and `.bind(value)` supplies each value. The database never parses a value as SQL, so no input can change the meaning of a statement. For values, this is the whole defence against SQL injection, and it is free. Names of tables and columns cannot be bound; when one must vary, choose it from a fixed list in your code.

**Rows become types.** `query_as::<_, T>` maps columns to the fields of a struct by name (`#[derive(sqlx::FromRow)]`), and `Row::try_get` reads a single column. The mapping can fail at run time (a missing column, a wrong type) because this module's queries are checked when they run.

**A transaction** makes several statements one unit: all of them happen, or none. It runs on one connection, from `begin` to `commit`. In sqlx, a transaction value that is dropped without `commit` is rolled back. An early return with `?` therefore undoes the work, and you do not have to write the rollback.

**Migrations** change the schema in ordered steps. Each step is a file that is applied once and never edited afterwards; a new change is a new file. sqlx records applied steps in a table of its own, so running the migrator on an up-to-date database does nothing. `sqlx::migrate!` reads the files at compile time and stores them in the binary, so the program that needs a schema also carries it.

### Worked example

A ledger with a transfer that must be all or nothing. The table has a `CHECK` constraint, and the second transfer breaks it on purpose:

```rust
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;

#[derive(Debug, PartialEq, sqlx::FromRow)]
struct Account {
    owner: String,
    cents: i64,
}

async fn transfer(pool: &SqlitePool, from: &str, to: &str, cents: i64) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE account SET cents = cents + ?1 WHERE owner = ?2")
        .bind(cents)
        .bind(to)
        .execute(&mut *tx)
        .await?;
    // If this statement fails, `?` returns, `tx` is dropped, and the first
    // UPDATE is rolled back.
    sqlx::query("UPDATE account SET cents = cents - ?1 WHERE owner = ?2")
        .bind(cents)
        .bind(from)
        .execute(&mut *tx)
        .await?;
    tx.commit().await
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), sqlx::Error> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;
    sqlx::query(
        "CREATE TABLE account (owner TEXT PRIMARY KEY, cents INTEGER NOT NULL CHECK (cents >= 0))",
    )
    .execute(&pool)
    .await?;
    sqlx::query("INSERT INTO account VALUES ('ada', 500), ('lin', 100)")
        .execute(&pool)
        .await?;

    transfer(&pool, "ada", "lin", 200).await?;
    assert!(transfer(&pool, "ada", "lin", 9_999).await.is_err());

    let accounts: Vec<Account> = sqlx::query_as("SELECT owner, cents FROM account ORDER BY owner")
        .fetch_all(&pool)
        .await?;
    // The failed transfer left no trace: lin did not keep the 9999 cents.
    assert_eq!(accounts[0], Account { owner: "ada".into(), cents: 300 });
    assert_eq!(accounts[1], Account { owner: "lin".into(), cents: 300 });
    Ok(())
}
```

### Mistakes people really make

**SQL is built with `format!`.** sqlx 0.9 refuses a query text that is not a string literal, and says why:

```rust,compile_fail,E0277
async fn by_label(pool: &sqlx::SqlitePool, label: &str) -> Result<i64, sqlx::Error> {
    let sql = format!("SELECT count(*) FROM item WHERE label = '{label}'");
    sqlx::query_scalar(&sql).fetch_one(pool).await
}
```

```text
error[E0277]: dynamic SQL strings should be audited for possible injections
   --> src/main.rs:3:24
    |
  3 |     sqlx::query_scalar(&sql).fetch_one(pool).await
    |     ------------------ ^^^^ dynamic SQL string
    |     |
    |     required by a bound introduced by this call
    |
    = help: the trait `SqlSafeStr` is not implemented for `&String`
    = note: prefer literal SQL strings with bind parameters or `QueryBuilder` to add dynamic data to a query.

            To bypass this error, manually audit for potential injection vulnerabilities and wrap with `AssertSqlSafe()`.
            For details, see the docs for `SqlSafeStr`.
```

The fix is a literal with a placeholder and `.bind(label)`. `AssertSqlSafe` is the escape hatch for SQL that really is dynamic, for example a table name chosen from a fixed list. In review, treat every `AssertSqlSafe` as a claim that someone must check. Lesson C01 has one that is false.

**One query per row of another query ("N+1").** Loading 40 shelves and then asking for the items of each shelf in a loop costs 41 round trips. With 4000 shelves it costs 4001, and nobody notices before production. Ask for everything in one statement (a `JOIN`, or `WHERE shelf_id IN (...)`) and group the rows in memory. The lesson's test counts statements.

**The pool is used inside a transaction.** Code that holds a transaction and then runs another query on `&pool` needs a *second* connection. With a pool of one, or under load with any pool, it waits for a connection that its own caller is holding:

```text
pool timed out while waiting for an open connection
```

Pass the transaction (`&mut *tx`) to everything that belongs to the unit of work. Writing your functions against `impl SqliteExecutor` makes that possible.

**In-memory SQLite surprises.** In sqlx, each parse of `sqlite::memory:` names a new private database that the connections of that one pool share. Two pools are two databases, and the database is gone when its last connection closes. Also: do not combine sqlx with tokio's paused test clock. SQLite runs on a thread of its own, the runtime looks idle while it works, the paused clock jumps ahead, and timeouts fire at once. Checked for this module: with `start_paused = true`, even `connect` fails with `PoolTimedOut`.

### When to use which

- `query` for statements without a result you need as a type, `query_as` for rows as structs or tuples, `query_scalar` for one column.
- The `query!` macros check SQL against a real database at compile time. They are worth their setup cost (a database at build time, or checked-in metadata) in a long-lived project.
- SQLite when one process writes and you want zero operations. PostgreSQL when several writers or several application servers are involved.

### Read more

- [sqlx: `query`, binding and fetching](https://docs.rs/sqlx/0.9.1/sqlx/fn.query.html)
- [sqlx: `Pool` and its options](https://docs.rs/sqlx/0.9.1/sqlx/struct.Pool.html)
- [sqlx: `migrate!`](https://docs.rs/sqlx/0.9.1/sqlx/macro.migrate.html)
- [SQLite: transactions](https://www.sqlite.org/lang_transaction.html) and [write-ahead logging](https://www.sqlite.org/wal.html)

## Lesson 04 · Authentication

Authentication answers "who is this?". This lesson builds the two usual parts: checking a password once, and recognising the same user on later requests. It also shows how small mistakes leak information. Read the limits at the end of this section before you reuse any of it.

### Storing passwords

A service must be able to check a password without being able to read it. So it stores a **hash**: a value computed from the password, from which the password cannot be computed back. Three properties matter.

- **Slow on purpose.** If the database leaks, the attacker tries candidate passwords against every hash. A general-purpose hash such as SHA-256 is built to be fast, so a modern graphics card can try a very large number of candidates per second. A password hash such as **Argon2id** has a cost you choose, in time and in memory, so each guess costs the attacker what a login costs you.
- **Salted.** A random value, different for every password, is mixed in and stored next to the hash. Equal passwords then give different hashes, and work done on one hash does not help with another.
- **Self-describing.** The stored text names the algorithm, the parameters and the salt. This is the PHC string format:

```text
$argon2id$v=19$m=19456,t=2,p=1$8BXwpM+bdhn3XkQpncQmXw$4D9ZoZKA2QzK4MQJX193xY96QGEr/1VaPDHfpx3Gq4s
 │         │    │               │                      └ the hash
 │         │    │               └ the salt, different for every password
 │         │    └ parameters: m = memory in KiB, t = passes, p = lanes
 │         └ version of the algorithm (19 is 0x13)
 └ algorithm
```

Verification reads the parameters from the stored string, not from your current configuration. So you can raise the cost later: old hashes still verify, and you re-hash a password with the new cost when its owner next logs in.

Choosing parameters: the `argon2` crate's defaults are Argon2id with 19 MiB, 2 passes and 1 lane. At the time of writing, that matches a minimum configuration in the OWASP Password Storage Cheat Sheet. Treat it as a floor, read the current guidance, and measure on the machine that will run the service. The tests in this module use far weaker values only so that they finish quickly.

A hash with the default parameters took about 16 ms in a release build and about 1.1 s in a debug build on the machine used to check this module. That is pure CPU time, and it must not run on an async worker thread. Lesson 06 explains why; here you move it with `spawn_blocking`.

That cost is also one that anyone can make you pay: each login attempt holds the configured memory and a core until it is done, and `spawn_blocking` does not limit how many run at once. A real service adds that limit (lesson 06 shows how).

### Tokens

After a successful login, the server gives the client something to show on later requests. There are two designs.

- A **session**: a long random identifier. The server stores what it means. Every request costs a lookup, and deleting the stored row ends the session at once.
- A **signed token**: the claims themselves (who, until when) plus a **MAC**, a keyed checksum that only someone with the server's secret key can compute. The server verifies the MAC and needs no lookup. The price: a token cannot be taken back before it expires, unless you add a lookup again.

The exercise builds a signed token with HMAC-SHA-256 so that you see every part. In a real system use an established format and library for the same idea.

### Worked example

A MAC over a message, and three ways it can fail to verify:

```rust
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

fn tag(key: &[u8], message: &[u8]) -> Vec<u8> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(message);
    mac.finalize().into_bytes().to_vec()
}

fn verify(key: &[u8], message: &[u8], tag: &[u8]) -> bool {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(message);
    // Compares all bytes in constant time; a wrong length is a mismatch.
    mac.verify_slice(tag).is_ok()
}

fn main() {
    let key = b"an example key: use 32 random bytes";
    let good = tag(key, b"seat=14A;until=1800000000");
    assert_eq!(good.len(), 32);
    assert!(verify(key, b"seat=14A;until=1800000000", &good));

    assert!(!verify(key, b"seat=1A;until=1800000000", &good), "changed message");
    assert!(!verify(b"another key", b"seat=14A;until=1800000000", &good), "wrong key");
    assert!(!verify(key, b"seat=14A;until=1800000000", &good[..16]), "shortened tag");
}
```

### Mistakes people really make

**Comparing secrets with `==`.** An ordinary comparison stops at the first byte that differs, so its duration says how many leading bytes were right. Measured over many requests, that can let an attacker find a valid MAC byte by byte. `verify_slice` (and `verify_password` in the `argon2` crate) take the same time wherever the difference is. Constant-time comparison removes this one signal. It does not make a whole request constant-time.

**Reading data before checking its MAC.** A token's payload is attacker-controlled until the MAC has been verified. Verify first, parse second, check the expiry third. Then a bug in the parser cannot be reached by someone without the key.

**Telling the client what was wrong.** "Unknown user" and "wrong password" as different answers let anyone list your users. The two cases must have the same status, headers and body. They should also cost the same work: if an unknown user is rejected before any hashing happens, the response time gives the same information away. So for an unknown user you verify against a decoy hash and ignore the result. This makes the two cases much harder to tell apart. It is not a proof that they are indistinguishable: the lookup itself, caches and the network still vary.

**Putting secrets in logs.** A derived `Debug` prints every field. Give types that hold passwords, tokens or keys a hand-written `Debug` that prints a placeholder.

### What this lesson does not give you

No transport encryption (use TLS in front of any real login), no rate limiting or lockout (without them, online guessing is limited only by the hash cost), no password reset, no multi-factor login, no key rotation, no token revocation. Each is a requirement for a real system.

### Read more

- [`argon2` crate documentation](https://docs.rs/argon2/0.6.0/argon2/)
- [`hmac` crate: `Mac::verify_slice`](https://docs.rs/hmac/0.13.0/hmac/trait.Mac.html#tymethod.verify_slice)
- [OWASP Password Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)
- [axum: `FromRequestParts`](https://docs.rs/axum/0.8.9/axum/extract/trait.FromRequestParts.html)

## Lesson 05 · Validation and API design

An API is a promise to programs you will never see. This lesson is about making the promise precise, and about four places where a vague promise hurts: input, lists, retries and change.

**Validate at the boundary, by changing the type.** A request arrives as loose data: any string, any number. The first thing your code does is convert it to a type that *cannot* hold a bad value, or reject it. After that conversion, no other function needs to check again, and the compiler makes sure that nobody skips the check, because the only way to get the strict type is through the validating function. Report all problems of a request at once. A client that fixes one field per round trip will not thank you.

**One error shape.** A client should be able to handle every failure of your API with one piece of code. That needs a machine-readable code, a human-readable message, and the same JSON structure everywhere, including for errors that the framework generates (a body that is not JSON, an unknown path). In axum that means wrapping the built-in extractors and setting fallbacks.

**Pagination by position, not by count.** `?offset=40&limit=20` means "skip 40 rows". If a row near the start is deleted between two requests, every later row moves up by one and the client never sees one of them:

```text
rows:            1 2 3 4 5 6 7        page 1 (offset 0, limit 3) → 1 2 3
row 2 deleted:   1 3 4 5 6 7          page 2 (offset 3, limit 3) → 5 6 7     4 is skipped
                                      page 2 (after id 3)        → 4 5 6     correct
```

A **cursor** says "continue after this item" (keyset pagination). It is stable under inserts and deletes, and the database can answer it from an index however deep the page is. Give the client an opaque string and reserve the right to change what is inside.

**Idempotency for retries.** A client that gets no answer to a `POST` cannot know whether the request was processed. If it retries, you may create two orders. The fix is a key chosen by the client and sent in a header: the server remembers the key together with the response, and a repeated key returns the stored response instead of doing the work again. Two details matter. The check and the work must be atomic, or two concurrent retries both pass the check. And the same key with a *different* request is a client bug that you report, not something to replay. A real service also deletes stored keys after a documented time, often a day; the exercise keeps them for ever.

**Versions for breaking changes.** Adding an optional field breaks nobody. Removing a field, renaming it or changing its type breaks every client that uses it. When you must do that, keep the old representation under its old path and offer the new one under a new prefix. Both read the same stored data.

### Worked example

A percentage that can only be built through validation, also when it is deserialized:

```rust
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(try_from = "i64")]
struct Percent(u8);

impl TryFrom<i64> for Percent {
    type Error = String;
    fn try_from(raw: i64) -> Result<Percent, String> {
        match u8::try_from(raw) {
            Ok(value) if value <= 100 => Ok(Percent(value)),
            _ => Err(format!("{raw} is not a percentage (0 to 100)")),
        }
    }
}

#[derive(Debug, Deserialize)]
struct Discount {
    code: String,
    percent: Percent,
}

fn main() {
    let ok: Discount = serde_json::from_str(r#"{"code": "SPRING", "percent": 15}"#).unwrap();
    assert_eq!(ok.percent, Percent(15));
    assert_eq!(ok.code, "SPRING");

    let bad = serde_json::from_str::<Discount>(r#"{"code": "OOPS", "percent": 150}"#);
    assert!(bad.unwrap_err().to_string().contains("150 is not a percentage"));
    // There is no way to hold a `Percent(150)`: every later function can rely on that.
}
```

`#[serde(try_from = "...")]` is the short form for one field. The lesson's `validate` function is the long form, for when you want to collect several problems.

### Mistakes people really make

**Validation is spread through the handlers.** Each handler checks what it remembers to check. The symptom is the same `if name.is_empty()` in four places and missing in the fifth. Put the rules where the type is made.

**The framework's errors look different from yours.** Your handlers answer JSON, and a malformed body gets axum's plain-text message. Clients then need two parsers. Test the error shape with broken input, not only with valid input.

**`limit` is unbounded.** A client that asks for `limit=1000000` gets a response that neither side can afford. Always set a maximum, and decide whether to reject or to clamp.

**A full last page promises another page.** If "is there more?" is computed as `items.len() == limit`, a last page that happens to be full sends the client to an empty page. Fetch one row more than the page size and look whether it exists.

### When to use which

- Reject unknown JSON fields (`deny_unknown_fields`) in configuration files. Accept them in requests between services, so that an old server can talk to a newer client.
- Status 400 for a request that cannot be understood, 422 for one that is understood and not acceptable, 409 for a conflict with the current state. Whatever you choose, document it and keep to it.
- Path versions (`/v1/...`) are visible and easy to route. Version headers keep URLs stable and are easy to forget.

### Read more

- [serde: container attributes (`try_from`)](https://serde.rs/container-attrs.html#try_from)
- [axum: customising extractor errors](https://docs.rs/axum/0.8.9/axum/extract/index.html#customizing-extractor-responses)
- [`TryFrom`](https://doc.rust-lang.org/std/convert/trait.TryFrom.html)
- [RFC 9457: Problem Details for HTTP APIs](https://www.rfc-editor.org/rfc/rfc9457)

## Lesson 06 · CPU-bound work in a service

An async runtime is cooperative. A worker thread runs one task until that task reaches an `.await` that is not ready, and only then picks up another task. A handler that computes for 200 ms without an `.await` holds its worker thread for 200 ms. Every other task that the thread could have served waits: health checks, other users' requests, timers. With four workers, four such requests stop the whole service, although the machine may have idle cores. The symptom in production is not "the slow endpoint is slow". It is "everything is sometimes slow".

So CPU-heavy work must leave the async threads. Two tools do that, and they solve different problems.

**rayon** is for *data parallelism*: one big computation over many items, spread over the cores. `par_iter()` turns a loop into a parallel one. rayon has a fixed pool of worker threads, and idle workers steal work from busy ones. `rayon::scope` lets parallel tasks borrow from the calling stack frame, because the scope does not return until all its tasks are done. `ThreadPool::install` runs a closure inside a pool of your choice.

**The bridge.** A handler cannot simply call rayon and wait: waiting for the result would block the async thread just the same. The pattern is: send the job to the pool, and `.await` a channel on which the job sends its result.

```text
async handler                rayon pool thread
     │ pool.spawn(job) ───────────► runs job()
     │ rx.await  (thread is free          │
     │            for other tasks)        │
     ◄──────── tx.send(result) ───────────┘
```

`tokio::task::spawn_blocking` is the same idea with tokio's own blocking pool. That pool is made for calls that *wait* (a blocking file API, a synchronous client library). It may start hundreds of threads, so it does not by itself limit how many computations run at once.

Two more things belong to the pattern. First, a **bound**: a pool has an unbounded queue, and under overload an unbounded queue turns into unbounded latency and then into an out-of-memory kill. Count the jobs that are queued or running, and refuse the next one early. Second, **cancellation does not reach the pool**: when the client disconnects, the handler's future is dropped, but the job keeps running to its end. The slot it occupies is free only when the job is done.

### Worked example

Word counts for many documents with rayon, then the same computation called from async code:

```rust
use rayon::prelude::*;
use std::collections::BTreeMap;
use tokio::sync::oneshot;

fn count_words(documents: &[String]) -> BTreeMap<String, usize> {
    documents
        .par_iter()
        // Each worker builds its own map: no lock, no sharing.
        .fold(BTreeMap::new, |mut counts, document| {
            for word in document.split_whitespace() {
                *counts.entry(word.to_lowercase()).or_insert(0) += 1;
            }
            counts
        })
        // The partial maps are merged pairwise.
        .reduce(BTreeMap::new, |mut left, right| {
            for (word, count) in right {
                *left.entry(word).or_insert(0) += count;
            }
            left
        })
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let documents = vec!["the cat sat".to_owned(), "The dog sat down".to_owned()];

    let (tx, rx) = oneshot::channel();
    rayon::spawn(move || {
        let _ = tx.send(count_words(&documents));
    });
    // While the pool computes, this thread can run other tasks.
    let counts = rx.await.expect("the job did not panic");

    assert_eq!(counts["the"], 2);
    assert_eq!(counts["sat"], 2);
    assert_eq!(counts["dog"], 1);
}
```

The merge order in `reduce` is not fixed, but adding counts gives the same result in any order. Where the order matters (the "best item, lowest index on ties" task in the lesson), make the order part of the value you compare.

### Mistakes people really make

**Calling the computation directly in the handler.** It works on your machine with one request at a time. The lesson's test makes the damage visible: a job waits for a message from another task on the same single-threaded runtime, and if the job runs on the async thread, that message can never be sent.

**Calling a parallel iterator from async code and waiting for it.** `items.par_iter().map(f).collect()` inside an `async fn` uses all cores, and still blocks the async thread it was called on until the last item is done.

**A panic in a spawned rayon job.** A panic inside `ThreadPool::spawn` has no caller to return to. rayon passes it to the pool's panic handler, and without a handler it aborts the process. Catch the panic inside the job (`std::panic::catch_unwind`) and turn it into an error value for the waiting handler.

**Using one pool for everything.** The global rayon pool is shared with every library in your process that uses rayon. A dedicated, named pool for request work can be sized, measured and limited.

### When to use which

- Work under roughly a millisecond: just do it in the handler.
- A blocking call that mostly waits: `spawn_blocking`.
- Computation: a dedicated rayon pool with a bounded entrance; or `spawn_blocking` plus a semaphore if you do not need parallelism inside one job.
- Jobs of many seconds, or jobs that must survive a restart: a separate worker process and a queue.

### Read more

- [rayon: parallel iterators](https://docs.rs/rayon/1.12.0/rayon/iter/index.html), [`scope`](https://docs.rs/rayon/1.12.0/rayon/fn.scope.html), [`ThreadPoolBuilder`](https://docs.rs/rayon/1.12.0/rayon/struct.ThreadPoolBuilder.html)
- [tokio: CPU-bound tasks and blocking code](https://docs.rs/tokio/1.54.1/tokio/index.html#cpu-bound-tasks-and-blocking-code)
- [`std::panic::catch_unwind`](https://doc.rust-lang.org/std/panic/fn.catch_unwind.html)
- [`std::thread::scope`](https://doc.rust-lang.org/std/thread/fn.scope.html), the standard library's version of scoped threads

## Lesson 07 · A binary protocol between two programs

HTTP with JSON is the right default between services. Sometimes it is not enough: a device sends a reading every second over a slow link, or two of your own programs exchange millions of small messages. Then you design the bytes yourself. This lesson does that once, completely, so that you know what the ready-made tools do.

**TCP delivers bytes, not messages.** If one side writes three messages, the other side may read them as one chunk, as seven chunks, or as two and a half messages now and the rest later. The first job of a protocol is **framing**: marking where one message ends. The usual way is a header with the length of what follows.

**Every header field has a job.** The frame of this lesson:

```text
magic (2)   "is this my protocol at all, and am I at a frame boundary?"
version (1) "do we agree on the rules?"            lets the protocol change later
kind (1)    "which message is this?"
length (4)  "where does the message end?"          checked against a maximum
payload (n)
CRC-32 (4)  "did the bytes arrive undamaged?"
```

**A decoder has three answers.** Given the bytes received so far, it says "here is a message, and it used this many bytes", or "not enough bytes yet", or "this can never be a valid message". The third answer ends the connection, because once a length cannot be trusted, nobody knows where the next frame starts.

**Lengths from the network are requests, not facts.** A header that says "4 GiB follow" must be refused when the header arrives. A decoder that answers "not enough bytes yet" makes the caller buffer until memory is gone. The same holds inside a payload: a count of four billion items in a ten-byte payload is a lie, and must not reach `Vec::with_capacity`.

**What a checksum is for.** CRC-32 detects accidental damage: a bad cable, a buggy proxy, a truncated write. It detects nothing done on purpose, because anyone can compute a new CRC for changed bytes. Protection against an attacker needs a MAC (lesson 04) or TLS.

**Compact encoding.** JSON writes numbers as text and repeats every field name. A binary encoding can use what it knows about the data. A *varint* stores a small number in one byte and a large one in up to ten. Sample times that are one second apart are stored as differences. In this lesson's test, 100 samples take 437 bytes as a binary frame and 3762 bytes as JSON. The price is that you cannot read the traffic with your eyes, and that every change needs both programs to agree.

**Backpressure.** A receiver that reads from the socket faster than it can process must put the surplus somewhere. If that place is an unbounded queue, memory grows until the process dies. If the queue is bounded and the receiver *waits* when it is full, the receiver stops reading, the kernel's socket buffers fill, and the sender's `write` waits too. The slowness travels back to its source, and nothing is lost. An **acknowledgement** makes this explicit at the level of messages: the sender keeps a batch until the receiver says "I have it", and the receiver says so only when that is true.

### Worked example

A different, smaller protocol: text messages with a one-byte length. The decoder has the same three answers, and the loop shows how a receiver uses it:

```rust
#[derive(Debug, PartialEq)]
enum Bad {
    Empty,
    NotUtf8,
}

/// `Ok(Some((text, used)))`, `Ok(None)` for "need more bytes", or an error.
fn decode(buf: &[u8]) -> Result<Option<(String, usize)>, Bad> {
    let Some((&len, rest)) = buf.split_first() else {
        return Ok(None);
    };
    if len == 0 {
        return Err(Bad::Empty); // decided from the header alone
    }
    let len = usize::from(len);
    if rest.len() < len {
        return Ok(None);
    }
    let text = String::from_utf8(rest[..len].to_vec()).map_err(|_| Bad::NotUtf8)?;
    Ok(Some((text, 1 + len)))
}

fn main() {
    // Two messages, delivered by the network in three awkward pieces.
    let pieces: [&[u8]; 3] = [b"\x02h", b"i\x05wor", b"ld"];
    let mut buffer = Vec::new();
    let mut messages = Vec::new();
    for piece in pieces {
        buffer.extend_from_slice(piece);
        // One read can complete zero, one or several messages.
        while let Some((text, used)) = decode(&buffer).unwrap() {
            buffer.drain(..used);
            messages.push(text);
        }
    }
    assert_eq!(messages, ["hi", "world"]);
    assert!(buffer.is_empty());
    assert_eq!(decode(b"\x00"), Err(Bad::Empty));
}
```

### Mistakes people really make

**One read is treated as one message.** It works in every local test, because loopback usually delivers small writes whole. It fails under load and on real networks. Test with one-byte chunks.

**The length is believed.** See above. The lesson's test sends only a header that announces almost 4 GiB and expects an error at once.

**The queue is unbounded "for now".** An unbounded channel between the reader and the consumer hides overload until it is an outage. The test fills a small pipe and checks that the sender really stops.

**The acknowledgement is sent on arrival.** If the receiver acknowledges a batch before the next stage has it, a crash in between loses data that the sender has already deleted. Decide what your `Ack` promises and send it only when the promise holds.

### When to use which

- JSON over HTTP: the default between services and for anything a human debugs.
- A schema format with tooling (Protocol Buffers with `prost`, or `postcard` through serde for Rust-only data): when messages are many or links are thin. Before you depend on a format crate, check that it still gets releases: `bincode`, long the usual choice for Rust-only data, is no longer maintained.
- A hand-written format: when you must control the exact bytes, or want to learn what the others do.
- For framing in real code, `tokio_util::codec` has `LengthDelimitedCodec` and the `Decoder` and `Encoder` traits. Your `decode_frame` has the shape of `Decoder::decode`.

### Read more

- [`tokio_util::codec`](https://docs.rs/tokio-util/latest/tokio_util/codec/index.html)
- [`tokio::sync::mpsc`: bounded channels](https://docs.rs/tokio/1.54.1/tokio/sync/mpsc/index.html)
- [`u32::to_be_bytes`](https://doc.rust-lang.org/std/primitive.u32.html#method.to_be_bytes) and [`std::io::Read::read`](https://doc.rust-lang.org/std/io/trait.Read.html#method.read), which documents that a read may return fewer bytes than asked
- [Protocol Buffers: the varint encoding](https://protobuf.dev/programming-guides/encoding/#varints)

## Lesson 08 · Observability and operations

A service is finished when someone else can run it at three in the morning. That person cannot read your code. They can read what the service says about itself, and they can start and stop it. This lesson builds the five things they need.

**Logs with context.** A log line alone ("lookup failed") is useless when a thousand requests run at once. A **span** is a period of work with fields: a request, with its method, path and id. Every event that happens inside the span carries those fields. In async code a span must follow its task across `.await` points and from thread to thread. `future.instrument(span)` does that: the span is entered whenever the future is polled and left when the poll returns.

**Metrics.** Logs describe single events; metrics count. A **counter** only goes up (requests handled). A **gauge** goes up and down (requests in flight). A **histogram** counts observations in buckets (how many requests took at most 10 ms, at most 100 ms, ...). A metric has **labels**, and every distinct combination of label values is a separate time series that the monitoring system stores. That is why a label must have few possible values: the route *template* `/items/{id}`, never the path `/items/8812`.

The Prometheus text format is what a `/metrics` endpoint usually returns:

```text
# HELP http_requests_total Requests handled, by method, route and status.
# TYPE http_requests_total counter
http_requests_total{method="GET",route="/items/{id}",status="200"} 2
```

Histogram buckets in this format are cumulative: the bucket `le="0.1"` counts every observation up to 0.1 seconds, including those already counted in `le="0.01"`. Serve `/metrics` to your monitoring system only, not to the public: it describes your routes and your load.

**Health and readiness.** Two questions, asked by whatever supervises the process. *Liveness*: "is this process hopelessly stuck, so that I should restart it?" *Readiness*: "should I send traffic to this instance right now?" A service that is starting, draining or cut off from its database is alive and not ready. If liveness also checked the database, a database outage would make the supervisor restart every instance again and again, which helps nobody.

**Configuration.** A service reads its settings once, at start-up, from layers: built-in defaults, then a file, then environment variables, each overriding the one before. It validates everything before it opens a port, and it stops with a clear message when something is wrong. A typo in a setting name must be an error. A service that starts with a default because `DRIAN_SECONDS` was ignored is a slow accident. Secrets come from the environment or a secret store, never from the repository, and are never printed.

**Graceful shutdown.** Deployments stop your service many times a day. The polite sequence is:

```text
SIGTERM arrives
  1. report "not ready"            the load balancer stops sending new requests
  2. stop accepting connections
  3. finish requests in progress   up to a drain time limit
  4. close pools, flush logs, exit 0
after the supervisor's grace period: SIGKILL, whatever state you are in
```

The drain limit must be shorter than the supervisor's grace period, or step 4 never runs.

Step 1 takes time to have an effect. A load balancer or orchestrator notices "not ready" only at its next check, and for a moment it still sends new connections. Many deployments therefore wait a few seconds between step 1 and step 2, in the service or in a pre-stop hook of the orchestrator. The exercise goes from step 1 to step 2 at once, which is enough for a test and too abrupt behind a real load balancer; know which delay your platform needs.

### Worked example

Draining a set of workers with a `watch` channel as the stop signal. Workers finish the job they have and take no new one:

```rust
use std::time::Duration;
use tokio::sync::watch;
use tokio::task::JoinSet;

async fn worker(id: u32, stop: watch::Receiver<bool>) -> (u32, u32) {
    let mut done = 0;
    // Check for the stop signal between jobs, never in the middle of one.
    while !*stop.borrow() {
        tokio::time::sleep(Duration::from_millis(10)).await; // one job
        done += 1;
    }
    (id, done)
}

#[tokio::main(flavor = "current_thread", start_paused = true)]
async fn main() {
    let (stop_tx, stop_rx) = watch::channel(false);
    let mut workers = JoinSet::new();
    for id in 0..3 {
        workers.spawn(worker(id, stop_rx.clone()));
    }

    tokio::time::sleep(Duration::from_millis(35)).await;
    stop_tx.send(true).unwrap(); // the "SIGTERM"

    // Drain with an upper bound.
    let drained = tokio::time::timeout(Duration::from_secs(1), workers.join_all()).await;
    let results = drained.expect("workers stop within the drain time");
    // Each worker was in its fourth job when the signal came, and finished it.
    assert!(results.iter().all(|&(_, done)| done == 4));
}
```

The example uses tokio's paused clock (`start_paused`), so the durations are exact and the example takes no real time. In a real `main`, the signal comes from `tokio::signal`; the lesson supplies that function.

### Mistakes people really make

**A span guard is held across `.await`.** This compiles, and it is wrong:

```rust,no_run
async fn handler() -> &'static str {
    let span = tracing::info_span!("work");
    let _entered = span.enter();          // stays "entered" on this thread...
    tokio::task::yield_now().await;       // ...while other tasks run here
    "done"
}
```

While this task waits, the thread runs other tasks, and their events are logged inside *this* span. The log then shows nested spans that never were nested. The lesson's tracing test interleaves two requests to catch exactly this. Use `.instrument(span)` on the future, or `#[tracing::instrument]` on the function.

**A label has unbounded values.** User ids, full paths, error messages as labels: each new value is a new time series, kept in memory by your service and by the monitoring system. The usual outcome is that the monitoring system fails first.

**The server binds `127.0.0.1` in a container.** Inside a container, that address is reachable only from inside the same container. The port is published, the process runs, and every connection is refused. The lab makes you see it.

**Shutdown is "the process gets killed".** Requests in progress are cut off, clients see reset connections, and a transaction may be left to the database to clean up. If your process ignores SIGTERM, `docker stop` waits its grace period and then kills it; the exit code 137 is the trace.

### When to use which

- Log an event when a person will read it: errors, decisions, start-up and shutdown. Count with a metric when you want a rate or a distribution.
- Liveness: no dependencies. Readiness: the dependencies without which this instance cannot serve.
- Environment variables for what differs per deployment and for secrets. A file for structured settings. Flags for what an operator changes by hand.

### Read more

- [tracing: spans, and `Instrument`](https://docs.rs/tracing/0.1.44/tracing/trait.Instrument.html)
- [axum: `serve` and graceful shutdown](https://docs.rs/axum/0.8.9/axum/serve/struct.Serve.html#method.with_graceful_shutdown)
- [Prometheus: the text exposition format](https://prometheus.io/docs/instrumenting/exposition_formats/)
- [`std::env::set_var` and why it is unsafe](https://doc.rust-lang.org/std/env/fn.set_var.html)
- The container lab: [lab/README.md](lab/README.md)

## Lesson 09 · Testing services

Tests for a service fail in a special way: they become slow and unreliable, people stop trusting them, and then they stop running them. The craft is to get the most evidence from tests that are fast and exact, and to keep the slow kinds few.

**Three levels.**

```text
in memory         call the Router as a function        fast, exact, no ports   most tests
over loopback     real HTTP on 127.0.0.1, port 0       sees the HTTP layer     a few
child processes   the real binaries, from outside      sees main, env, exit    very few
```

An in-memory test hands a request value to the router and gets a response value back. It covers routing, extractors, handlers, middleware and error mapping. It cannot see what hyper adds or enforces on a real connection, how the server behaves when a connection closes, or anything in `main`. For those you need the next level, and for start-up, configuration and exit codes the third.

**Every test owns its world.** Tests run in parallel and in any order. A test that shares a database, a port or a file with another test will fail one day for a reason that has nothing to do with the code. So each test makes its own database (an in-memory SQLite database is created in microseconds), binds port 0 and asks which port it got, and creates temporary files with unique names.

**No sleeping.** A test that sleeps 100 ms "so the server is up" is slow on a fast machine and fails on a slow one. Wait for a *fact*: bind the listener before spawning the server (then connections queue in the kernel), read the line that the child process prints when it is ready, wait on a channel that the handler signals. Put a timeout around every wait, so that a bug gives a failure message and not a hung test run.

**Make it fail on purpose.** The interesting behaviour of a service is what it does when the database is down, when a dependency never answers, when the second of two statements fails. None of that happens by itself in a test. You inject it: close the pool, pass a dependency that never returns, add a database trigger that raises an error. Then you assert what the client sees *and* what state is left behind.

**Test the tests.** A check that cannot fail is worth nothing, and you cannot tell by reading it. Run it against a version of the code with a known defect (a *mutant*) and see whether it notices. The ecosystem track's E05 does this for properties; this lesson does it for service checks.

### Worked example

An in-memory test of a small router, written as a function that returns `Result`, so it can be pointed at a broken variant too:

```rust
use axum::http::{Request, StatusCode};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};
use tower::ServiceExt;

fn temperature_service(broken: bool) -> Router {
    Router::new().route(
        "/temperature",
        get(move || async move {
            let unit = if broken { "F" } else { "C" };
            Json(json!({ "value": 21.5, "unit": unit }))
        }),
    )
}

async fn check_units(app: &Router) -> Result<(), String> {
    let request = Request::get("/temperature").body(axum::body::Body::empty()).unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    if response.status() != StatusCode::OK {
        return Err(format!("expected 200, got {}", response.status()));
    }
    let bytes = axum::body::to_bytes(response.into_body(), 4096).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).map_err(|err| err.to_string())?;
    if body["unit"] != "C" {
        return Err(format!("the contract says Celsius, got {body}"));
    }
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    assert_eq!(check_units(&temperature_service(false)).await, Ok(()));
    // The same check must notice the defect, or it checks nothing.
    assert!(check_units(&temperature_service(true)).await.is_err());
}
```

### Mistakes people really make

**A fixed port.** `bind("127.0.0.1:8080")` in a test fails when two tests run at once, when the developer's own server is running, and on CI machines that use the port. Port 0 never collides.

**One database for all tests.** The first test's rows change the second test's counts. The usual repair, deleting everything at the start of each test, breaks parallel runs. Give every test a private database.

**Asserting only the status code.** A 500 with the SQL error text in the body has the right status and leaks your schema. Assert the whole body where the body is part of the contract.

**Virtual time with real threads.** tokio's paused clock is excellent for timeouts in pure async code. It is wrong as soon as real threads or real I/O take part, because the runtime looks idle while they work and the clock jumps ahead. In this module, the tests that touch SQLite or real sockets use real time with bounded waits, and the rest use the paused clock.

### When to use which

- A real SQLite database for every test that touches SQL. A fake behind a trait for dependencies you do not own (a mail service, a payment provider).
- In-memory calls for behaviour, loopback tests for the HTTP layer and shutdown, process tests for the contract of the binary.
- A trait and a failing implementation for most fault injection; the database itself for faults in the middle of a transaction.

### Read more

- [`tower::ServiceExt::oneshot`](https://docs.rs/tower/0.5.3/tower/trait.ServiceExt.html#method.oneshot)
- [tokio: `test` attribute and `start_paused`](https://docs.rs/tokio/1.54.1/tokio/attr.test.html)
- [The Cargo book: integration tests and `CARGO_BIN_EXE_<name>`](https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates)
- [`std::process::Command`](https://doc.rust-lang.org/std/process/struct.Command.html)

## Capstone · beacon

The capstone has no new concept. It asks you to combine the nine lessons in a system that you design, and the hard part is the design. Before you write code, answer these on paper.

**What does an acknowledgement promise?** The agent deletes samples when it gets an `Ack`. So an `Ack` must mean "these samples are on disk". That fixes the order on the server: store, commit, then acknowledge.

**What happens when a message is lost?** There are two cases: the batch never arrived, or the batch arrived and the `Ack` was lost. The agent cannot tell them apart, so it sends the batch again. The server will therefore see some samples twice. This is *at-least-once delivery*, and it is harmless if storing a sample twice has the same effect as storing it once. Decide what makes two samples "the same", and let the database enforce it.

```text
agent                          server
  │── Batch 7 ───────────────►   stored, committed
  │      x ◄──── Ack 7 ───────   (the Ack is lost)
  │   ...timeout, reconnect...
  │── Hello, Batch 1 (same samples) ──►  stored again? must change nothing
  │◄──── Ack 1 ────────────────
```

**Where is every bound?** List each place where data can pile up: the agent's buffer, the socket, the server's connection count, the database pool, the HTTP body, a page of results. Each needs a limit and a decision about what happens at the limit: wait, refuse, or drop. For the agent's buffer the contract decides: drop the oldest.

**Which tasks exist, and what do they share?** In the agent, sampling must not wait for shipping. That suggests two tasks and one shared structure between them. Decide who may remove a sample from it, and when. In the server, the ingest side and the HTTP side share only the database pool.

**How does each program stop?** The agent with `BEACON_COUNT` stops when its buffer is empty. The server stops on a signal, in the order from lesson 08.

Write the modules after you have these answers. Then let the acceptance tests tell you where the answers were incomplete.

## At a glance: gRPC and message queues

Two technologies come up in almost every discussion about services. This module does not teach them, but you have now built their main ideas by hand, and that makes them easy to place.

**gRPC** is remote procedure calls over HTTP/2, with messages defined in Protocol Buffers. You write a `.proto` file that lists services, methods and message types. A code generator produces a client and a server skeleton in each language, so both ends agree on the bytes by construction. That is the job your shared `beacon_wire` crate does, extended to many languages. gRPC adds deadlines that travel with a call, streaming in both directions with flow control (backpressure, as in lesson 07), and standard status codes. In Rust, [`tonic`](https://docs.rs/tonic) is the usual implementation and [`prost`](https://docs.rs/prost) generates the message types; tonic services are tower services, so the middleware ideas of lesson 02 apply. Choose gRPC for calls between services of different teams or languages. Stay with JSON over HTTP when browsers or people are the clients.

**A message queue** (or message broker) sits between a sender and a receiver, so that they need not be up at the same time or work at the same speed. The sender publishes a message; the broker stores it; a consumer takes it, processes it and acknowledges it; an unacknowledged message is delivered again. You know every word of that sentence from lesson 07 and the capstone: bounded buffering, acknowledgements that mean something, at-least-once delivery, and consumers that must be idempotent. Brokers differ in what they are good at: RabbitMQ routes messages to work queues, Apache Kafka keeps an ordered log that many consumers read at their own position, and NATS is a light publish-subscribe system with an optional persistent layer. Rust clients exist for each ([`lapin`](https://docs.rs/lapin), [`rdkafka`](https://docs.rs/rdkafka), [`async-nats`](https://docs.rs/async-nats)). Choose a queue when work can be done later, must survive a restart of the worker, or must be spread over many workers. Do not choose one to make a slow synchronous call look fast: the caller still needs the answer.
