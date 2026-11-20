//! Milestone M6: the bounded thread pool, back-pressure and graceful
//! shutdown. SPEC sections 10 and 11.
//!
//! How these tests know what the server is doing: a connection that has
//! received a response and stays open is being served by a worker, and
//! keeps that worker until it closes. The tests build every situation out
//! of such observed facts and never out of guesses about speed. Where a
//! test must wait for something it cannot observe (the server reading
//! bytes that were just sent), it waits `SETTLE`, which is hundreds of
//! times longer than the server needs.

mod common;

use std::thread;
use std::time::{Duration, Instant};

use common::{Conn, SLACK, Server, get, post_json};

const FILES: &[(&str, &str)] = &[
    ("hello.txt", "hello\n"),
    ("0.txt", "file zero\n"),
    ("1.txt", "file one\n"),
    ("2.txt", "file two\n"),
    ("3.txt", "file three\n"),
];

/// Time for bytes that were sent to be read by a waiting server thread.
const SETTLE: Duration = Duration::from_millis(300);

/// Opens a connection and proves that a worker is serving it: one request
/// is answered, and the connection stays open.
#[track_caller]
fn occupy_a_worker(server: &Server) -> Conn {
    let mut conn = server.connect();
    conn.send(get("/hello.txt"));
    let response = conn.response();
    response.expect_status(200, "the request that occupies a worker");
    response.expect_keep_alive(true, "the request that occupies a worker");
    conn
}

// ---------------------------------------------------------------------------
// The pool
// ---------------------------------------------------------------------------

#[test]
fn workers_serve_connections_at_the_same_time() {
    let server = Server::start_with(FILES, &["--workers", "4", "--queue", "4"]);
    // Four connections, each stopped in the middle of a request.
    let mut conns: Vec<Conn> = (0..4).map(|_| server.connect()).collect();
    for (n, conn) in conns.iter_mut().enumerate() {
        conn.send(format!("GET /{n}.txt HTTP/1.1\r\nHo"));
    }
    thread::sleep(SETTLE);
    // They are finished in the opposite order. A server that handles one
    // connection at a time is still waiting for the first one.
    let bodies = ["file zero\n", "file one\n", "file two\n", "file three\n"];
    for (n, conn) in conns.iter_mut().enumerate().rev() {
        conn.send("st: test\r\n\r\n");
        let response = conn
            .try_response(true)
            .unwrap_or_else(|problem| panic!(
                "connection {n} of 4 got no answer while the earlier connections were still in the middle \
                 of their requests. With --workers 4, four connections are served at the same time \
                 (SPEC 10.1).\n  {problem}"
            ));
        response.expect_status(200, &format!("connection {n} of 4"));
        assert_eq!(
            response.text(),
            bodies[n],
            "connection {n} must get its own file"
        );
    }
}

#[test]
fn a_slow_client_does_not_delay_the_others() {
    let server = Server::start_with(FILES, &["--workers", "2", "--queue", "2"]);
    let mut slow = server.connect();
    slow.send("POST /echo HTTP/1.1\r\nHost: test\r\nContent-Type: application/json\r\nContent-Length: 8\r\n\r\n[ 1");
    thread::sleep(SETTLE);

    let mut fast = server.connect();
    for round in 0..50 {
        fast.send(get("/hello.txt"));
        fast.response().expect_status(
            200,
            &format!("request {round} next to a stalled connection"),
        );
    }
    slow.send(", 2 ]");
    assert_eq!(
        slow.response().text(),
        "[1,2]",
        "the slow request, once it is complete"
    );
}

#[test]
fn connections_beyond_the_workers_wait_in_the_queue() {
    let server = Server::start_with(
        FILES,
        &[
            "--workers",
            "2",
            "--queue",
            "1",
            "--idle-timeout-ms",
            "30000",
        ],
    );
    let first = occupy_a_worker(&server);
    let _second = occupy_a_worker(&server);

    // Both workers are busy. The third connection is accepted and waits.
    let mut third = server.connect();
    third.send(get("/1.txt"));
    third.expect_silence(
        Duration::from_millis(500),
        "a third connection with --workers 2: no worker is free, so it must wait. A server that answers \
         here starts a thread per connection or has more workers than it was told (SPEC 10.1)",
    );

    // One worker becomes free; the waiting connection is served.
    drop(first);
    let response = third.response();
    response.expect_status(200, "the queued connection, after a worker became free");
    assert_eq!(response.text(), "file one\n");
}

#[test]
fn waiting_connections_are_served_in_the_order_they_arrived() {
    let server = Server::start_with(
        FILES,
        &[
            "--workers",
            "1",
            "--queue",
            "3",
            "--idle-timeout-ms",
            "30000",
        ],
    );
    let busy = occupy_a_worker(&server);
    // Three connections wait for the only worker. Each has already sent its
    // request, for a different file. They connect one after the other from
    // one thread, so the server accepts them in this order.
    let mut waiting: Vec<Conn> = (0..3)
        .map(|n| {
            let mut conn = server.connect().patience(Duration::from_secs(2));
            conn.send(get(&format!("/{n}.txt")));
            conn
        })
        .collect();
    thread::sleep(SETTLE);

    let bodies = ["file zero\n", "file one\n", "file two\n"];
    let mut serving = busy;
    for (n, body) in bodies.into_iter().enumerate() {
        // The connection that has the worker goes away; the worker takes
        // the one that has waited longest.
        drop(serving);
        let mut next = waiting.remove(0);
        let response = match next.try_response(true) {
            Ok(response) => response,
            Err(problem) => panic!(
                "waiting connection {n} of 3 was not served when the worker became free. Connections that \
                 wait are served in the order in which they were accepted (SPEC 10.2); a queue that hands \
                 out the newest connection first serves the wrong one here.\n  {problem}"
            ),
        };
        response.expect_status(200, &format!("waiting connection {n} of 3"));
        assert_eq!(
            response.text(),
            body,
            "waiting connection {n} must get the file it asked for"
        );
        if let Some(later) = waiting.first_mut() {
            later.expect_silence(
                Duration::from_millis(200),
                "a connection further back in the queue, while the only worker serves an earlier one",
            );
        }
        serving = next;
    }
}

#[test]
fn time_spent_waiting_for_a_worker_is_not_limited() {
    let server = Server::start_with(
        FILES,
        &[
            "--workers",
            "1",
            "--queue",
            "1",
            "--idle-timeout-ms",
            "1000",
            "--request-timeout-ms",
            "2000",
        ],
    );
    let mut busy = occupy_a_worker(&server);
    // This connection waits for the only worker. The first half of its
    // request is already on the way, but the server reads nothing from a
    // waiting connection.
    let mut waiting = server.connect();
    waiting.send("GET /1.txt HTTP/1.1\r\nHo");

    // The first connection keeps the worker for 2.5 seconds: longer than
    // the idle timeout and longer than the request timeout. It sends a
    // request every 100 ms, so it is never idle for long itself. (If the
    // test machine stalls for a whole second, the server may close this
    // connection early. Then the wait is shorter, and the test is easier to
    // pass, not harder.)
    let until = Instant::now() + Duration::from_millis(2500);
    while Instant::now() < until {
        thread::sleep(Duration::from_millis(100));
        if busy.try_send(get("/hello.txt")).is_err() || busy.try_response(true).is_err() {
            break;
        }
    }
    drop(busy);

    // The worker is free. It takes the waiting connection and reads the
    // first half of the request: only now does the request timeout start.
    // The second half comes a moment later, in a read of its own.
    thread::sleep(SETTLE);
    waiting.send("st: test\r\n\r\n");
    let response = match waiting.try_response(true) {
        Ok(response) => response,
        Err(problem) => panic!(
            "a connection waited 2.5 s for the only worker, with --idle-timeout-ms 1000 and \
             --request-timeout-ms 2000, and was not served afterwards. Time in the queue counts for neither \
             limit: both clocks start when a worker takes the connection (SPEC 7.3 and 10.2).\n  {problem}"
        ),
    };
    response.expect_status(
        200,
        "a connection that waited in the queue for longer than both time limits: the idle timeout does not \
         run while it waits, and the request timeout starts when a worker reads the first byte (SPEC 7.3)",
    );
    assert_eq!(response.text(), "file one\n");
}

#[test]
fn a_full_server_answers_503_at_once() {
    let server = Server::start_with(
        FILES,
        &[
            "--workers",
            "1",
            "--queue",
            "1",
            "--idle-timeout-ms",
            "30000",
        ],
    );
    let busy = occupy_a_worker(&server);
    // The second connection fills the queue. Nothing shows from outside
    // when it has been accepted, so give the accept loop a moment.
    let mut queued = server.connect();
    thread::sleep(SETTLE);

    // The server now holds workers + queue = 2 connections. Each further
    // one is refused, and the refusal does not wait for a request: these
    // clients send nothing at all.
    for attempt in 1..=3 {
        let mut refused = server.connect().patience(Duration::from_secs(2));
        let response = refused.response();
        response.expect_error(
            503,
            &format!("connection {attempt} beyond workers + queue (SPEC 10.3)"),
        );
        response.expect_keep_alive(false, "503");
        assert_eq!(
            response.header("retry-after"),
            Some("1"),
            "a 503 response carries `Retry-After: 1`"
        );
        assert!(
            response.header("date").is_some(),
            "a 503 response has a Date header like any other"
        );
        refused.expect_closed(SLACK, "after 503");
    }

    // The accepting thread does not wait for a refused client either. Four
    // clients connect, say nothing and stay open, and the last of them is
    // asked first. A server that deals with one refused client at a time,
    // and waits for each to read its answer or to close, makes the fourth
    // one wait for the three before it.
    let mut silent: Vec<Conn> = (0..4)
        .map(|_| server.connect().patience(Duration::from_secs(2)))
        .collect();
    for (n, conn) in silent.iter_mut().enumerate().rev() {
        let response = match conn.try_response(true) {
            Ok(response) => response,
            Err(problem) => panic!(
                "refused client {n} of 4 got no 503 within 2 s while the other refused clients were still \
                 connected. The thread that accepts connections must not wait for a client that it refuses \
                 (SPEC 10.3).\n  {problem}"
            ),
        };
        response.expect_error(503, &format!("refused client {n} of 4"));
    }
    drop(silent);

    // The refusals did not disturb the two connections that were admitted.
    drop(busy);
    queued.send(get("/2.txt"));
    let response = queued.response();
    response.expect_status(200, "the queued connection, after the busy one closed");
    assert_eq!(response.text(), "file two\n");

    // There is room again: one waiting place.
    let mut next = server.connect();
    thread::sleep(SETTLE);
    next.send(get("/3.txt"));
    drop(queued);
    assert_eq!(
        next.response().text(),
        "file three\n",
        "a new connection after the load has gone"
    );
}

#[test]
fn workers_return_to_the_pool_when_clients_vanish() {
    let server = Server::start_with(FILES, &["--workers", "2", "--queue", "2"]);
    // Clients that leave at every stage of a request, twenty of each.
    for round in 0..20 {
        let unfinished: [&[u8]; 4] = [
            b"",
            b"GET /hello.txt HT",
            b"GET /hello.txt HTTP/1.1\r\nHost: test\r\n",
            b"POST /echo HTTP/1.1\r\nHost: test\r\nContent-Type: application/json\r\nContent-Length: 50\r\n\r\n[1,",
        ];
        for bytes in unfinished {
            // The server may be full for a moment and refuse: that is fine.
            if let Ok(mut conn) = Conn::try_open(server.addr()) {
                let _ = conn.try_send(bytes);
            }
        }
        // After each round the server must recover and serve normally.
        let deadline = Instant::now() + SLACK;
        loop {
            // While the server still holds the vanished clients it may
            // answer 503, and a 503 may get lost if it crosses this request
            // on the wire. Both are fine for a moment, not for long.
            let outcome = Conn::try_open(server.addr()).map(|mut conn| {
                let _ = conn.try_send(get("/hello.txt"));
                conn.try_response(true)
            });
            match outcome {
                Ok(Ok(response)) if response.status == 200 => break,
                _ if Instant::now() < deadline => thread::sleep(Duration::from_millis(20)),
                other => panic!(
                    "round {round}: after four clients disconnected in the middle of their requests, the \
                     server must serve again within {SLACK:?}. A worker that never notices a closed \
                     connection, or dies on it, is lost for good (SPEC 10.2). Last attempt: {other:?}"
                ),
            }
        }
    }
}

#[test]
fn a_long_series_of_short_connections_is_served() {
    let server = Server::start_with(FILES, &["--workers", "2", "--queue", "2"]);
    for n in 0..300 {
        let mut conn = server.connect();
        conn.send(format!(
            "GET /{}.txt HTTP/1.1\r\nHost: test\r\nConnection: close\r\n\r\n",
            n % 4
        ));
        let response = conn.response();
        response.expect_status(
            200,
            &format!(
                "connection {n} of 300, each opened after the one before was closed by the server. A 503 \
                 here means that closed connections still count as held (SPEC 10.2)"
            ),
        );
        conn.expect_closed(SLACK, "after `Connection: close`");
    }
}

#[test]
fn parallel_clients_all_get_their_own_answers() {
    let server = Server::start_with(FILES, &["--workers", "4", "--queue", "8"]);
    let addr = server.addr();
    // 12 clients = workers + queue: nobody is refused, 8 of them wait.
    thread::scope(|scope| {
        for client in 0..12 {
            scope.spawn(move || {
                let mut conn = Conn::open(addr).patience(Duration::from_secs(20));
                for round in 0..40 {
                    let context = format!("client {client}, round {round}");
                    let body = format!("{{ \"client\" : {client} , \"round\" : {round} }}");
                    conn.send(post_json("/echo", &body));
                    let response = conn.response();
                    response.expect_status(200, &context);
                    assert_eq!(
                        response.text(),
                        format!("{{\"client\":{client},\"round\":{round}}}"),
                        "{context}: a response must belong to the request of the same connection"
                    );
                    conn.send(get(&format!("/{}.txt", client % 4)));
                    let expected =
                        ["file zero\n", "file one\n", "file two\n", "file three\n"][client % 4];
                    assert_eq!(conn.response().text(), expected, "{context}");
                }
            });
        }
    });
}

// ---------------------------------------------------------------------------
// Shutdown
// ---------------------------------------------------------------------------

/// Checks the end of a shutdown: the process exits by itself, with status 0.
///
/// There is no check that the port is free afterwards. A process that has
/// exited holds no socket, and the system may give the port to another
/// program at once (for example to the server of a test that runs at the
/// same time), so "somebody answers on that port" would prove nothing.
#[track_caller]
fn expect_clean_exit(server: &mut Server) {
    let status = server.wait_exit(SLACK + SLACK);
    assert_eq!(
        status.code(),
        Some(0),
        "after a graceful shutdown the exit status is 0 (SPEC 11.3), got {status}"
    );
}

/// Expects the line `shutting down` as the next line of standard output.
#[track_caller]
fn expect_shutting_down(server: &mut Server) {
    assert_eq!(
        server.stdout_line(SLACK).as_deref(),
        Some("shutting down"),
        "after the `listening on` line, the next and last line on standard output is `shutting down` \
         (SPEC 11.2). Log lines belong on standard error."
    );
}

#[test]
fn the_line_shutdown_stops_the_server() {
    let mut server = Server::start(FILES);
    server
        .get("/hello.txt")
        .expect_status(200, "before shutdown");
    server.write_stdin("shutdown\n");
    expect_shutting_down(&mut server);
    expect_clean_exit(&mut server);
    assert_eq!(
        server.stdout_line(SLACK),
        None,
        "nothing is printed after `shutting down`"
    );
}

#[test]
fn end_of_input_stops_the_server() {
    let mut server = Server::start(FILES);
    server
        .get("/hello.txt")
        .expect_status(200, "before shutdown");
    server.close_stdin();
    expect_shutting_down(&mut server);
    expect_clean_exit(&mut server);
}

#[test]
fn other_input_lines_are_ignored() {
    let mut server = Server::start(FILES);
    for line in [
        "\n",
        "hello\n",
        "shutdown now\n",
        " shutdown\n",
        "SHUTDOWN\n",
        "shut\n",
        "down\n",
        "xshutdown\n",
    ] {
        server.write_stdin(line);
    }
    thread::sleep(SETTLE);
    assert!(
        server.is_running(),
        "only the exact line `shutdown` (or end of input) stops the server (SPEC 11.1)"
    );
    server
        .get("/hello.txt")
        .expect_status(200, "after eight lines that are not `shutdown`");
    // Windows line endings are accepted.
    server.write_stdin("shutdown\r\n");
    expect_shutting_down(&mut server);
    expect_clean_exit(&mut server);
}

#[test]
fn shutdown_closes_idle_connections_at_once() {
    let mut server = Server::start_with(FILES, &["--idle-timeout-ms", "60000", "--workers", "3"]);
    let mut used = occupy_a_worker(&server);
    let mut unused = server.connect();
    thread::sleep(SETTLE);

    let started = Instant::now();
    server.write_stdin("shutdown\n");
    used.expect_closed(
        SLACK,
        "an idle connection that was used before: closed without a response (SPEC 11.2)",
    );
    unused.expect_closed(
        SLACK,
        "an idle connection that never sent a request: closed without a response",
    );
    expect_shutting_down(&mut server);
    expect_clean_exit(&mut server);
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "shutdown took {:?}; it must not wait for the idle timeout (60 s here)",
        started.elapsed()
    );
}

#[test]
fn shutdown_lets_a_request_in_progress_finish() {
    let mut server =
        Server::start_with(FILES, &["--workers", "2", "--request-timeout-ms", "20000"]);
    // A request whose first half has reached the server.
    let mut active = server.connect();
    active.send("POST /echo HTTP/1.1\r\nHost: test\r\nContent-Type: application/json\r\nContent-Length: 12\r\n\r\n{ \"ok\"");
    thread::sleep(SETTLE);

    server.write_stdin("shutdown\n");
    expect_shutting_down(&mut server);

    // The server no longer accepts: a new client gets no response.
    match Conn::try_open(server.addr()) {
        Err(_) => {}
        Ok(mut late) => {
            let _ = late.try_send(get("/hello.txt"));
            let bytes = late.read_until_closed(SLACK);
            assert!(
                bytes.is_empty(),
                "a connection made after `shutting down` was printed must not be served (SPEC 11.2), but \
                 it received {:?}",
                common::abbreviate(&bytes)
            );
        }
    }

    // It is still running, because the request in progress is not done.
    thread::sleep(SETTLE);
    assert!(
        server.is_running(),
        "the server exited while a request was in progress; that request must be allowed to finish \
         (SPEC 11.2)"
    );

    active.send(" : 1 }");
    let response = active.response();
    response.expect_status(200, "the request that was in progress when shutdown began");
    assert_eq!(response.text(), "{\"ok\":1}");
    response.expect_keep_alive(
        false,
        "a response sent during shutdown says `Connection: close` (SPEC 11.2)",
    );
    active.expect_closed(SLACK, "after the last response");
    expect_clean_exit(&mut server);
}

#[test]
fn shutdown_drops_waiting_connections_without_a_response() {
    let mut server = Server::start_with(
        FILES,
        &[
            "--workers",
            "1",
            "--queue",
            "2",
            "--request-timeout-ms",
            "20000",
        ],
    );
    let mut active = server.connect();
    active.send("GET /hello.txt HTTP/1.1\r\nHo");
    thread::sleep(SETTLE);
    // The only worker is busy, so this connection waits in the queue.
    let mut waiting = server.connect();
    thread::sleep(SETTLE);

    server.write_stdin("shutdown\n");
    expect_shutting_down(&mut server);
    waiting.expect_closed(
        SLACK,
        "a connection that no worker had started to serve is closed without a response (SPEC 11.2)",
    );

    active.send("st: test\r\n\r\n");
    let response = active.response();
    response.expect_status(200, "the request in progress");
    response.expect_keep_alive(false, "a response sent during shutdown");
    expect_clean_exit(&mut server);
}

#[test]
fn a_stalled_request_cannot_delay_shutdown_beyond_its_timeout() {
    let mut server = Server::start_with(
        FILES,
        &["--request-timeout-ms", "700", "--idle-timeout-ms", "60000"],
    );
    let mut stalled = server.connect();
    stalled.send("GET /hello.txt HTTP/1.1\r\nHo");
    thread::sleep(Duration::from_millis(200));

    let started = Instant::now();
    server.write_stdin("shutdown\n");
    expect_shutting_down(&mut server);
    // The request never completes: it gets its 408 as usual, and then
    // nothing keeps the server alive.
    let response = stalled.response();
    response.expect_error(408, "a request that stalls during shutdown");
    stalled.expect_closed(SLACK, "after 408");
    expect_clean_exit(&mut server);
    assert!(
        started.elapsed() < Duration::from_millis(700) + SLACK + SLACK,
        "shutdown took {:?} with one stalled request and --request-timeout-ms 700",
        started.elapsed()
    );
}

#[test]
fn shutdown_under_load_loses_no_accepted_request() {
    let mut server = Server::start_with(FILES, &["--workers", "4", "--queue", "4"]);
    let addr = server.addr();
    // Four clients send requests as fast as they can until the server
    // closes their connection. Every response they do get must be correct
    // and complete: shutdown may refuse work, it may not damage it.
    //
    // The line `shutdown` is written after 300 ms. A client that is still
    // being served long after that fails; it does not go on for ever.
    let give_up = Instant::now() + Duration::from_millis(300) + SLACK;
    let completed = thread::scope(|scope| {
        let clients: Vec<_> = (0..4)
            .map(|client| {
                scope.spawn(move || {
                    let mut conn = Conn::open(addr);
                    let mut completed = 0u32;
                    loop {
                        assert!(
                            Instant::now() < give_up,
                            "client {client} was still being served {SLACK:?} after the line `shutdown` was \
                             written to the server's standard input (SPEC 11.1 and 11.2)"
                        );
                        let body = format!("[ {client} , {completed} ]");
                        if conn.try_send(post_json("/echo", &body)).is_err() {
                            return completed;
                        }
                        match conn.try_response(true) {
                            Ok(response) => {
                                assert_eq!(response.status, 200, "client {client}");
                                assert_eq!(response.text(), format!("[{client},{completed}]"), "client {client}");
                                completed += 1;
                                if !response.keeps_alive() {
                                    return completed;
                                }
                            }
                            // Closed between two requests: allowed.
                            Err(problem) if problem.closed_without_a_byte => return completed,
                            Err(problem) => panic!(
                                "client {client}: a response was cut off during shutdown (SPEC 11.2).\n  {problem}"
                            ),
                        }
                    }
                })
            })
            .collect();
        thread::sleep(Duration::from_millis(300));
        server.write_stdin("shutdown\n");
        clients
            .into_iter()
            .map(|client| {
                client
                    .join()
                    .expect("a client thread failed; see its message above")
            })
            .sum::<u32>()
    });
    assert!(
        completed > 0,
        "the clients completed no request at all before shutdown"
    );
    expect_shutting_down(&mut server);
    expect_clean_exit(&mut server);
}
