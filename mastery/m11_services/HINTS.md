# Mastery M11 hints

[Module guide](README.md) · [Concepts](CONCEPTS.md) · [Notes workbook](NOTES.md)

Try the first hint before opening the second. Open *Check your explanation* only after you have written your own answer to the lesson's EXPLAIN prompt in [NOTES.md](NOTES.md). Complete model solutions are in `solutions/mastery/m11_services/` in the course repository; use them after your own version passes, or when the second hint was not enough.

## Lesson 01 · Service anatomy

<details><summary>Hint 1</summary>

For task A, `match self` to a tuple of `(StatusCode, &'static str, String)` and build the body with `serde_json::json!`. A tuple `(StatusCode, Json(body))` implements `IntoResponse`. For task B, lock the map, work on it, and let the guard go before you build the response. A `BTreeMap` iterates in key order. For task C, each handler lists the extractors it needs: `State` first, `Path` next, `Json` last.

</details>
<details><summary>Hint 2</summary>

Routes: `.route("/parts", get(list_parts).post(create_part))`, `.route("/parts/{sku}", get(..).delete(..))`, `.route("/parts/{sku}/adjust", post(..))`, then `.with_state(state)`. The created response can be `(StatusCode::CREATED, [(header::LOCATION, location)], Json(part)).into_response()`. In the adjust handler, take the lock once, `get_mut` the part, compute `i64::from(part.qty).checked_add(delta)` (the client chooses `delta`, so a plain `+` can overflow and panic while you hold the lock), and convert back with `u32::try_from(..)`, mapping both failures to `InsufficientStock`. Task D is one line: `axum::serve(listener, app(state)).await`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

hyper reads the request from the socket and hands it to the router. The router finds the route for `/parts` and, in it, the handler for `POST`; an unknown path gives 404 and a known path with another method gives 405. Then the extractors run in parameter order. `State` clones the `AppState`, which only clones an `Arc`, so the handler sees the same map as every other request. `Json` checks the content type, reads the body and deserializes it; if that fails, axum answers 415, 400 or 422 and the handler body never runs. The handler locks the mutex only while it checks and inserts, and returns a value or an `ApiError`. Whichever it is, `IntoResponse` turns it into status, headers and body, in one place for all handlers.

</details>

## Lesson 02 · Middleware with tower

<details><summary>Hint 1</summary>

Task A: `fetch_add` on the shared counter, format `req-{n}`, and wrap it as `RequestId::new(HeaderValue)`. Task B: build one `ServiceBuilder` and read it from top (outermost) to bottom (next to the handler). Ask for each layer: which responses must still pass through it? Task C is the same three layers written in two notations. Task D: take the time, `next.run(request).await`, then insert the header into the response.

</details>
<details><summary>Hint 2</summary>

Task B, from outside to inside: `SetRequestIdLayer::new(X_REQUEST_ID, CountingId::default())`, `PropagateRequestIdLayer::new(X_REQUEST_ID)`, `CompressionLayer::new()`, `TimeoutLayer::with_status_code(StatusCode::SERVICE_UNAVAILABLE, limits.timeout)`, `RequestBodyLimitLayer::new(limits.max_body)`. Task E: write a small struct that holds the `Arc<AtomicUsize>` and subtracts one in `Drop`. Take a slot with a compare-and-swap loop (`try_update` with a closure that returns `None` when the count is at `max`). If there is no slot, return a boxed future with the 503 response. Otherwise move the slot value into the boxed future together with the inner call, exactly as `TagService::call` moves its inner service.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

The stack is: request id (set, then propagate) around compression around timeout around body limit around the handler. When the handler is too slow, the timeout layer stops waiting, drops the handler's future and creates a 503 response itself. That response then travels outward through compression and through the propagate layer, which copies the id from the request to the response. Layers inside the timeout never see this response, which is why the id layers must be outside. `InFlightLimit` needs a value with a `Drop` impl because a future can end in two ways: it completes, or it is dropped half way by a timeout or a disconnect. Code after `.await` runs only in the first case. A destructor runs in both.

</details>

## Lesson 03 · Persistence

<details><summary>Hint 1</summary>

Task A: `SqliteConnectOptions::from_str(url)?` with `create_if_missing`, `foreign_keys` and `journal_mode`, then `SqlitePoolOptions::new()` with `max_connections` and `acquire_timeout`, then `MIGRATOR.run(&pool).await?`. Task B: `match &error` and use the methods of the database error (`is_unique_violation`, ...) in match guards. Task C: every value goes through `.bind(..)`; `RETURNING` lets an `INSERT` hand back the stored row. Task D: `pool.begin()`, run both statements on `&mut *tx`, `commit` at the end. Think about what an early `return Err(..)` does to `tx`.

</details>
<details><summary>Hint 2</summary>

Task D: `UPDATE item SET qty = qty - ?1 WHERE id = ?2`. The CHECK constraint turns an overdraft into an error that your `From` impl maps to `Insufficient`, and `rows_affected() == 0` tells you that the id does not exist. Returning early drops the transaction, which rolls it back. Task F: one statement with `FROM shelf LEFT JOIN item ON item.shelf_id = shelf.id ORDER BY shelf.name, item.id`. Walk the rows; start a new `ShelfListing` whenever the shelf name changes; read `item.id` as `Option<i64>` and skip the item when it is `None` (a shelf without items). Run the query with `.fetch_all(db)` so that it is counted. Two statements (all shelves, all items) and a `HashMap` work as well.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Each `UPDATE` is atomic, but the pair is not: if the process stops, or the second statement fails, after the first one, units have left one item and arrived nowhere. A transaction makes the pair one unit, and dropping it on any early return undoes the first statement. When the pool has no free connection, the caller waits for the acquire timeout and then gets `PoolTimedOut`. That is overload, not a bug, so the service should answer 503 and the client may retry. The listing with one query per shelf is correct and fast with three shelves. Its cost grows with the data: 4000 shelves mean 4001 round trips per request, each needing a connection from the pool, so the endpoint gets slower every month and takes the pool away from other requests.

</details>

## Lesson 04 · Authentication

<details><summary>Hint 1</summary>

Task A: build `Argon2::new(Algorithm::Argon2id, Version::V0x13, params.clone())`; `hash_password(bytes)` picks a random salt, and `.to_string()` gives the PHC string. To verify, parse with `PasswordHash::new(stored)` and call `verify_password`; treat `Error::PasswordInvalid` as `Ok(false)` and every other error as `Err`. Task B: the MAC covers the text `v1.<payload hex>`. Create it with `Hmac::<Sha256>::new_from_slice(key)`, `update`, `finalize().into_bytes()`. For verification, the crate has a method that compares in constant time.

</details>
<details><summary>Hint 2</summary>

Task B, verification: `token.split('.')`, require exactly three parts with `v1` first, decode both hex parts (`Malformed` if that fails), recompute the MAC over `b"v1."` plus the payload hex and call `verify_slice(&tag)` (`BadSignature`). Only then `serde_json::from_slice` the payload, and compare `now >= claims.expires_at`. Task C: before any `.await`, choose the hash to check: the user's, or `state.decoy_hash` when the user is unknown, and remember which. Run `checker.matches(..)` inside `tokio::task::spawn_blocking`. Succeed only if the user exists *and* the check matched. Task D: read `parts.headers`, `strip_prefix("Bearer ")`, verify with `state.key` and `state.clock.now()`, and map every failure to `Unauthorized`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A password is a low-entropy secret that people reuse, and the stored hash may leak. The hash must therefore be slow and memory-hard, so that each guess is expensive, and salted, so that guesses cannot be shared between users. A token MAC protects a value with a high-entropy server key that nobody can guess, so it only needs to be unforgeable, and it must be fast because it runs on every request. The MAC is checked first because, until it matches, the payload is input from an unknown sender; the parser should never see it. The decoy hash makes an unknown user cost the same hash computation as a known one, so the response time no longer shows which names exist. It does not hide differences from the user lookup or the network, it does not slow down guessing against a known user, and it is no substitute for rate limiting.

</details>

## Lesson 05 · Validation and API design

<details><summary>Hint 1</summary>

Task A: normalise first (trim, lower-case, sort, dedup), then check, and push a `FieldProblem` for each field that fails instead of returning at the first. Task B: `ApiJson::from_request` calls `Json::<T>::from_request(request, state).await` and converts the rejection with your `From<JsonRejection>` impl; match on the variants `JsonSyntaxError`, `JsonDataError` and `MissingJsonContentType`, and use `rejection.status()` for the rest. For the router: `Router::nest("/v1", ..)`, `.fallback(..)` for unknown paths, `.method_not_allowed_fallback(..)`, and `DefaultBodyLimit::max(MAX_BODY_BYTES)`.

</details>
<details><summary>Hint 2</summary>

State: one `Arc<Mutex<Store>>` with a `BTreeMap<u64, Ticket>`, the last id, and a `HashMap<String, (NewTicket, Value)>` for idempotency keys. Pagination: parse `after` as an id (default 0), take `store.tickets.range((std::ops::Bound::Excluded(after), std::ops::Bound::Unbounded))` (not `after + 1..`: the client chooses `after`, and `u64::MAX + 1` overflows), fetch `limit + 1` entries, and set `next` to the id of the last returned ticket only if the extra entry existed. Read the query as `Query<HashMap<String, String>>` so that you can produce your own error for a bad `limit`. Take the id as `Path<String>` and parse it yourself, so that `abc` becomes your 404. Idempotency: validate, then lock once; if the key is known, compare the stored `NewTicket` with the new one and either replay the stored body or return `idempotency_key_reused`; otherwise create the ticket and store the key before you unlock.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

The boundary is the call of `validate` in the create handler: before it there is a `TicketInput` that may hold anything, after it a `NewTicket` that cannot exist unless the rules hold, because its fields are private. Keyset pagination names a position in the data ("after id 3"), and that position stays where it is when earlier rows disappear. Offset pagination names a count of rows to skip, and the same count points at a different row after a delete, so one row is never shown. An idempotency key says "this is the same request again". If the body differs, that statement is false: the client has a bug, such as a key generated once and reused for every order. Replaying the old response would hide the bug and silently drop the new request, so the server reports it.

</details>

## Lesson 06 · CPU-bound work in a service

<details><summary>Hint 1</summary>

Task A: `items.par_iter().map(&score).collect()` keeps the order. For `best`, turn each item into a value whose ordinary `max()` is the answer; `enumerate()` gives you the index. Task B: inside `rayon::scope(|scope| ..)`, loop over `pixels.chunks_mut(width).enumerate()` and call `scope.spawn(move |_| edit(index, row))`; borrow `edit` before the loop so that every task can share it. Task C: `rayon::ThreadPoolBuilder::new().num_threads(n).thread_name(|i| ..).build()`.

</details>
<details><summary>Hint 2</summary>

`best`: map to `(score, std::cmp::Reverse(index))` and take `.max()`. Tuples compare from the left, and the largest `Reverse(index)` is the smallest index. Task D: create a `oneshot` channel, `pool.spawn(move || { if let Ok(value) = catch_unwind(AssertUnwindSafe(job)) { let _ = tx.send(value); } })`, then `rx.await.map_err(|_| JobLost)`. A dropped sender is how a panic becomes an error. Task E: a `tokio::sync::Semaphore` with `max_pending` permits; `Arc::clone(&slots).try_acquire_owned()` either gives a permit or means `Overloaded`. Move the permit *into the job closure* and drop it there, after the job has run and before the result is sent. `pending()` is `max_pending - available_permits()`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

An async worker thread serves many tasks by switching between them at `.await` points. While one task computes without awaiting, the thread cannot switch, so every other task assigned to it waits, whatever it wanted to do. The slow request steals time from fast ones. `CpuLimiter` exists to protect the CPU pool, and the pool is busy for as long as the job runs. When a caller is cancelled, its future is dropped, but the closure on the pool keeps running. If the slot were released with the caller, new jobs would be accepted while the old ones still occupy the threads, and the limit would mean nothing. `spawn_blocking` is enough for a few blocking or short CPU calls where you only need them off the async threads. A rayon pool is better when one request's work can be split over cores, or when you want a fixed number of CPU threads.

</details>

## Lesson 07 · A binary protocol between two programs

<details><summary>Hint 1</summary>

Part 1 (`wire/src/lib.rs`). Varint: while the value is `>= 0x80`, push the low seven bits with the top bit set and shift right by seven; then push the rest. Reading: at most ten bytes; add `(byte & 0x7f) << (7 * index)`; stop at a byte without the top bit; the tenth byte may only be 0 or 1. Zigzag is `(value << 1) ^ (value >> 63)`. Write two private helpers for strings. In `decode_payload`, keep a `&mut &[u8]` cursor and require it to be empty at the end. In `decode_frame`, check the magic as soon as two bytes are there, the version at three, the length at eight, and only then ask whether the buffer is long enough.

Part 2 (`src/l07_protocol.rs`). `next` is a loop: try `decode_frame` on the buffer first; if it gives a message, `drain` the used bytes and return; otherwise read more. A read of zero bytes is the end of the stream.

</details>
<details><summary>Hint 2</summary>

Part 1. For a batch, guard the sample count: each sample needs at least two bytes, so a count larger than `input.len() / 2` is `Truncated`. Add time differences with `checked_add`. In `encode_frame`, remember `out.len()` at the start, write a header with four zero bytes for the length, encode the payload, patch the length in, append `crc32fast::hash(&out[start..])`, and `truncate(start)` on any error.

Part 2. `receive`: `tokio::io::split(stream)`, a `FrameReader` on the read half; expect `Hello`; then for each `Batch`: `sink.send(received).await` (map a closed channel to `SinkClosed`), and only after that `write_message(.., &Message::Ack { seq })`. `send_batches`: the same split; write `Hello`; for each batch write it and then match `reader.next().await?` against `Some(Message::Ack { seq: got }) if got == seq`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

In `decode_frame`, the length check protects memory, the checksum detects damage, and the version byte (with the kind byte) lets the protocol change. The length must be judged when the header arrives because "need more bytes" tells the caller to keep buffering, and a peer that announces 4 GiB and then sends slowly would fill the receiver's memory. The CRC does not protect against a deliberate change: whoever alters the bytes can recompute it.

Backpressure means that a slow stage slows down the stages before it instead of letting work pile up. The line that creates it is `sink.send(..).await`: while the queue is full, `receive` does not read, the socket buffers fill, and the sender's write waits. If the receiver acknowledged on arrival, an `Ack` would only mean "I saw it"; the sender would discard a batch that a crash could still lose. An unbounded queue works perfectly as long as the consumer keeps up, which is every test you run by hand. Under load it grows until the process is killed, and the cause is far from the symptom.

</details>

## Lesson 08 · Observability and operations

<details><summary>Hint 1</summary>

Task A: collect the settings as optional text values first: from the file, then overwritten by `APP_` variables. Validate at the end, in one place. For the file, parse into a `toml::Table` and match each key yourself; that way you decide what an error says. Task B: keep the counters in a `BTreeMap` keyed by `(method, route, status)` so that the output is sorted; store the histogram as counts per bucket and add them up while rendering. Task C: read the labels *before* `next.run`, because the request is moved into it.

</details>
<details><summary>Hint 2</summary>

Task A: for a TOML syntax error, report only a line number computed from `err.span()`; both the `Display` text and `message()` of the parser can repeat values from the file. Task C: `request.extensions().get::<MatchedPath>()` gives the template, and its absence means `unmatched`. Add the `/metrics` route *after* `.layer(from_fn_with_state(..))` so that it is not counted. Task D: build `tracing::info_span!("request", method = %.., path = %request.uri().path(), request_id = %..)` and write `async move { let response = next.run(request).await; tracing::info!(status = .., "request finished"); response }.instrument(span).await`. Task E: `tokio::time::timeout(check_timeout, sqlx::query("SELECT 1").execute(&pool))`. Task F: give axum a shutdown future that awaits `shutdown`, sets the phase, and signals a `oneshot`; pin the server future; `select!` between it and the `oneshot`; then `timeout(drain, &mut server)`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`/healthz` answers "is this process able to respond at all?" for the supervisor that would otherwise restart it. `/readyz` answers "should requests be routed here now?" for the load balancer; it is false while starting, while draining, and when the database cannot be reached. The `route` label must be the template because every distinct label value becomes its own time series, and paths contain ids: a million items would be a million series. A guard from `span.enter()` marks the span as active on the current *thread* until the guard is dropped. At an `.await` the task is suspended, but the guard is still alive, so the thread carries the span into whatever task it runs next, and those events are logged under the wrong request. `.instrument` enters the span for each poll and leaves it when the poll returns.

</details>

## Lesson 08 lab

<details><summary>Model answers to the lab questions</summary>

1. Process 1 has no default action for signals it does not handle: an unhandled SIGTERM is simply ignored. A server that installs no handler therefore keeps running until `docker stop` sends SIGKILL after its grace period (10 seconds by default). Requests in progress are cut off, and the exit code is 137. This is why the server handles SIGTERM itself and why the image starts it with the exec form of `ENTRYPOINT`, without a shell as process 1.
2. The orchestrator or load balancer does, from outside the container: for example Kubernetes liveness and readiness probes, or a load balancer's health check. A `HEALTHCHECK` in the image needs an HTTP client inside the image and is ignored by Kubernetes.
3. Whether the binary is linked dynamically against glibc (then the runtime image needs a compatible glibc and the loader; `scratch` has neither, and a static build needs the musl target and a C toolchain for it, because SQLite is compiled from C). Whether a non-root user exists without `/etc/passwd` (a numeric `USER` works). Whether the program needs time zone data or CA certificates (this one does not: it makes no outgoing TLS connections). And how you will debug an image without a shell.

</details>

## Lesson 09 · Testing services

<details><summary>Hint 1</summary>

Task A: `app.clone().oneshot(request).await` gives the response; `axum::body::to_bytes(body, usize::MAX)` collects the body. Decide the three cases (empty, JSON, other) after you have the bytes. Task B: `SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:")`, run the migrator, insert the two rows. Task C: bind `127.0.0.1:0`, read `local_addr()`, spawn `axum::serve`, keep the `JoinHandle`; `Drop` aborts it. Task D: each check is a sequence of `call`s whose results you compare with the contract, returning `Err(format!(..))` at the first difference.

</details>
<details><summary>Hint 2</summary>

For `check_create`: assert the status *and* the whole body of a successful create (the fixture has two tools, so the new id is 3), a blank name after trimming, and a duplicate of a seeded name. For `check_failures_are_opaque`: `pool.close().await`, then call a route and compare the whole body with `json!({"error": "internal"})`; comparing the whole body catches any extra field. For `check_borrow_is_atomic`: `break_loan_log(pool).await`, try to borrow tool 1, expect 500, then `GET /tools/1` and require `borrower` to be null. Small helpers such as `fn expect(status, wanted, &body, what) -> Result<(), String>` keep the checks readable.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Only `Running` needs a socket: it shows that the service works through real HTTP, on a port the system chose, and that it stops. Everything about the crib's behaviour can be asked in memory, which is faster and cannot collide with anything. Each test gets its own database because tests run in parallel and in any order; shared rows would make results depend on timing. The database was made to fail in two ways: closing the pool makes every statement fail, and a trigger makes only the second statement of a borrowing fail. The first showed what a client sees when the database is down. The second showed whether a half-finished operation leaves a trace, and no test of the normal case can show that, because in the normal case both statements succeed.

</details>

## C01 · Code review

<details><summary>Hint 1</summary>

There are six defects. Look for: a query whose text depends on input; an error path that hands internal text to the client; a limit that was removed; a call of unknown length on the async thread; a call to another service with no time limit; and a lock that is alive while the handler waits. Then look for the `.clone()` calls and the `unwrap()`.

</details>
<details><summary>Hint 2</summary>

Use one fixed SQL text with `?1`; `WHERE ?1 IS NULL OR author = ?1` with `.bind(Option<String>)` makes the filter optional. Replace `(StatusCode, String)` with an enum that implements `IntoResponse` and log the detail with `tracing::error!`. Set `DefaultBodyLimit::max(MAX_BODY_BYTES)`. Run the exporter in `tokio::task::spawn_blocking`. Wrap the summariser call in `tokio::time::timeout(SUMMARY_TIMEOUT, ..)` and map the timeout to 504. Read the note text before anything else, call the summariser, and only then lock a `std::sync::Mutex` for the single increment.

</details>
<details><summary>Check your review</summary>

1. **SQL injection.** `list_notes` pastes the author into the SQL text. `x' OR '1'='1` returns everybody's notes, and `O'Brien` breaks the statement. sqlx had refused the dynamic string; the author silenced it with `AssertSqlSafe`. Fix: a literal with a bind parameter.
2. **Error details sent to clients.** `failed` puts the database error text into the body. That text can name tables, columns and file paths. Fix: a small error enum with fixed codes; details go to the log.
3. **No body limit.** `DefaultBodyLimit::disable()` lets one request use as much memory as it likes. The 413 in the demo was the limit working; the answer was a higher number, not no number.
4. **Blocking call in a handler.** `export_notes` runs a synchronous library call on an async worker thread, so every other request on that thread waits for it. Fix: `spawn_blocking`.
5. **No timeout on a dependency.** `summarize_note` waits for the summariser forever. One stuck dependency then holds requests, and with them connections and memory, without limit. Fix: `tokio::time::timeout` and a 504.
6. **A lock held across `.await`.** The handler takes the stats lock first and keeps it while it queries the database and waits for the summariser. All summaries run one after another, and `GET /stats` hangs behind them. The comment even records the warning sign: the standard mutex "did not compile" there, because its guard may not cross an `.await` in a task that can move between threads. Fix: do the waiting first, then lock briefly; the standard mutex then compiles and is the right tool.

Smaller points: the `.clone()` calls on `author` and `text` are unnecessary (bind by reference, then move the strings into the response); `text.unwrap()` after `is_none()` should be `ok_or(..)?`; `sql = sql + ..` is `sql += ..`; a summary that timed out must not be counted.

</details>

## Capstone · beacon

<details><summary>Milestone 1 and 2 (server)</summary>

Parse the environment in a pure function that takes pairs and returns `Result<Config, ConfigError>`; `main` maps the error to exit code 78. Hash the administrator's password once at start-up (on `spawn_blocking`), keep only the hash, and also keep a decoy hash for unknown users. Bind both listeners, *then* print the two lines and flush. The ingest side is an accept loop that spawns one task per connection; each task is lesson 07's `receive` with "store in one transaction, commit, then `Ack`" in place of the channel. A primary key over (agent, metric, time) and `INSERT ... ON CONFLICT (agent, metric, ts_ms) DO NOTHING` make redelivery harmless. (`INSERT OR IGNORE` passes the tests too, but it also skips rows that break a `NOT NULL` or `CHECK` constraint, without an error.) Put a time limit on writing the `Ack` as well as on reading, and limit how many password hashes run at once. Any error in a connection task ends that task only. The query is keyset pagination on `ts_ms`: `WHERE ts_ms > ?after ORDER BY ts_ms LIMIT ?limit + 1`.

</details>
<details><summary>Milestone 3 and 4 (agent)</summary>

Two tasks share `Mutex<VecDeque<Sample>>` plus counters, with a `tokio::sync::Notify` to wake the shipper. The sampler pushes a sample at every tick and, if the queue is at capacity, pops the front first and counts a drop. It never touches the network. The shipper copies up to `BEACON_BATCH` samples from the front *without removing them*, sends them, and waits for the `Ack`, with one timeout around the write and the wait. Connecting needs a timeout too. On the `Ack`, it removes from the front every sample whose time is not later than the newest sample of the batch, and counts those as shipped. On any error it drops the connection, sleeps (50 ms, doubling up to 1 s), connects again, sends `Hello`, and continues with whatever is in the queue now. It finishes when sampling has ended and the queue is empty. Keep sample times strictly increasing with `max(now, last + 1)`. Give `BEACON_BATCH` and the two names an upper limit when you read the configuration, so that a batch always fits in one frame: a sample takes at most 20 bytes, and a frame carries 64 KiB.

</details>
<details><summary>If a test hangs or fails in a confusing way</summary>

Every failure message of the acceptance tests includes the standard error of your program. Write your diagnostics there. If `s1` to `s4` time out at start-up, check that both lines are printed after the listeners are bound and that standard output is flushed. If `a4` reports other numbers than `shipped=8 dropped=52`, check whether samples that were sent but not acknowledged still count against the buffer, and which end of the queue you drop from. If `s3` loses data after the kill, look at the order of commit and `Ack`.

</details>

## Mastery check · model answers

<details><summary>Open after answering the questions in the module guide yourself</summary>

1. The router matches path then method (404, 405). Extractors run in parameter order and may each reject (400, 415, 422, 401 from your own). Only then the handler runs. The body extractor must be last because the body can be consumed once.
2. `.layer(a).layer(b)`: `b` is outermost. `ServiceBuilder::new().layer(a).layer(b)`: `a` is outermost. A timeout inside the request-id layers still returns a response through them, so the 503 carries the id; a timeout outside them does not.
3. Dropping a future runs destructors but not the code after the `.await` that was pending. Cleanup that must happen on every exit therefore lives in `Drop`: releasing a slot, decrementing a gauge, rolling back a transaction.
4. The second statement fails; `?` returns; the transaction value is dropped; sqlx rolls back. The first update is undone. Without a transaction it would stay.
5. The caller waits up to the acquire timeout and gets `PoolTimedOut`. Map it to 503. Causes: slow queries, a pool that is too small, or a function that holds one connection (a transaction) and asks the pool for another.
6. Slow and memory-hard so that offline guessing against leaked hashes is expensive; salted so that work is not shared between users; parameters stored in the hash so that the cost can be raised. A MAC key is long and random, so nobody guesses it, and the MAC runs on every request.
7. Same status, headers and body for both cases, and one hash verification in both cases (a decoy for unknown users). It narrows the timing difference; it does not remove every difference and does not replace rate limiting.
8. Offset pagination skips or repeats rows when earlier rows are deleted or inserted between requests, and gets slower with depth. Keyset pagination continues after a known key and is stable.
9. The key is stored with the first response under the same lock (or transaction) that creates the resource. A repeat with an equal request replays the response; a repeat with a different request is an error.
10. The handler blocks an async worker thread. With N workers, N such requests stop all other tasks, including health checks. Move the work to a blocking or CPU pool and bound the number of pending jobs.
11. Magic and version are checked first; the length is checked against a maximum as soon as the header is complete; the CRC is checked before the payload is decoded; counts inside the payload are checked against the bytes that are present. A decoder returns a message, "need more", or a fatal error.
12. A bounded queue with a waiting sender. The receiver stops reading, the socket fills, the sender blocks. With an unbounded queue, memory grows until the process dies.
13. Liveness: can the process answer; no dependencies. Readiness: should it get traffic; false while starting, draining, or without its database. Draining: not ready, stop accepting, finish in-flight work within a limit, exit 0.
14. In-memory calls cover routing, extractors, handlers and middleware. A socket is needed for what the HTTP implementation adds or enforces and for connection and shutdown behaviour. Child processes are needed for `main`, configuration, output and exit codes.
15. The paused clock advances whenever the runtime has nothing to run. Work on other threads (SQLite, real sockets) is invisible to it, so timeouts fire although no real time has passed.

</details>
