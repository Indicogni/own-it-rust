//! Milestone M5: limits and timeouts. SPEC sections 5.4 (sizes), 7.3
//! (timeouts) and 7.4 (closing so that the last response survives).
//!
//! A server that handles one connection at a time passes this file.
//!
//! About time in these tests: a lower bound ("not before") is always well
//! below the configured timeout, and an upper bound is the timeout plus
//! `SLACK` (3 s). A correct server passes on a slow machine; a server that
//! has no timeout at all fails with a message, it does not hang the test.

mod common;

use std::io::Read;
use std::thread;
use std::time::{Duration, Instant};

use common::{Conn, SLACK, Server, Site, find, get, pattern_bytes, post_json};

const FILES: &[(&str, &str)] = &[("hello.txt", "hello\n")];

/// A GET for `/hello.txt` whose head is exactly `total` bytes long: one
/// header line is padded.
fn padded_get(total: usize) -> String {
    let fixed = "GET /hello.txt HTTP/1.1\r\nHost: test\r\nX-Pad: \r\n\r\n".len();
    let request = format!(
        "GET /hello.txt HTTP/1.1\r\nHost: test\r\nX-Pad: {}\r\n\r\n",
        "a".repeat(total - fixed)
    );
    assert_eq!(request.len(), total, "test setup");
    request
}

/// Expects the standard error response `status` with `Connection: close`,
/// then the close.
#[track_caller]
fn expect_refusal(conn: &mut Conn, status: u16, context: &str) {
    let response = conn.response();
    response.expect_error(status, context);
    response.expect_keep_alive(false, context);
    conn.expect_closed(SLACK, context);
}

// ---------------------------------------------------------------------------
// Sizes
// ---------------------------------------------------------------------------

#[test]
fn a_head_may_be_8192_bytes_and_not_one_more() {
    let server = Server::start(FILES);

    let mut conn = server.connect();
    conn.send(padded_get(8192));
    conn.response()
        .expect_status(200, "a head of exactly 8192 bytes");
    // The limit is per head, not per connection or per read.
    conn.send(padded_get(8192) + &get("/hello.txt") + &padded_get(8192));
    for nth in ["first", "second", "third"] {
        conn.response().expect_status(
            200,
            &format!("{nth} of three requests in one write, two with a full-size head"),
        );
    }
    // Body bytes do not count as head bytes.
    let body = "[ 1 ]";
    let fixed = format!(
        "POST /echo HTTP/1.1\r\nHost: test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nX-Pad: \r\n\r\n",
        body.len()
    );
    let pad = "a".repeat(8192 - fixed.len());
    conn.send(fixed.replace("X-Pad: ", &format!("X-Pad: {pad}")) + body);
    assert_eq!(
        conn.response().text(),
        "[1]",
        "a POST with a head of exactly 8192 bytes"
    );

    // Close the first connection before opening the next one, so that a
    // server with a single worker can pass this file.
    drop(conn);
    let mut conn = server.connect();
    conn.send(padded_get(8193));
    expect_refusal(&mut conn, 431, "a head of 8193 bytes (SPEC 5.4)");
}

#[test]
fn a_head_without_end_is_431_as_soon_as_it_is_too_long() {
    let server = Server::start(FILES);
    // The server's request timeout is 5 s. Nothing here ends the head, so a
    // server that keeps reading until an empty line arrives answers 408
    // after 5 s, or grows its buffer without limit. 431 must come at once.
    let endless_header = format!(
        "GET /hello.txt HTTP/1.1\r\nHost: test\r\nX-Pad: {}",
        "a".repeat(9000)
    );
    let endless_request_line = format!("GET /{}", "a".repeat(9000));
    let many_lines = format!(
        "GET /hello.txt HTTP/1.1\r\nHost: test\r\n{}",
        "X-A: b\r\n".repeat(1200)
    );
    for (request, what) in [
        (endless_header, "a header line that never ends"),
        (endless_request_line, "a request line that never ends"),
        (many_lines, "header lines without an empty line"),
    ] {
        let mut conn = server.connect().patience(Duration::from_secs(2));
        conn.send(request);
        expect_refusal(&mut conn, 431, what);
    }
}

#[test]
fn a_request_may_have_64_header_lines() {
    let server = Server::start(FILES);
    let with_lines = |count: usize| {
        let mut request = String::from("GET /hello.txt HTTP/1.1\r\nHost: test\r\n");
        for n in 1..count {
            request.push_str(&format!("X-Line-{n}: {n}\r\n"));
        }
        request + "\r\n"
    };
    let mut conn = server.connect();
    conn.send(with_lines(64));
    conn.response()
        .expect_status(200, "64 header lines, counting Host");
    conn.send(with_lines(65));
    expect_refusal(&mut conn, 431, "65 header lines (SPEC 5.4)");
}

#[test]
fn a_target_may_be_2048_bytes() {
    let server = Server::start(FILES);
    let with_target = |len: usize| {
        let target = format!("/hello.txt?{}", "q".repeat(len - "/hello.txt?".len()));
        assert_eq!(target.len(), len, "test setup");
        get(&target)
    };
    let mut conn = server.connect();
    conn.send(with_target(2048));
    conn.response()
        .expect_status(200, "a target of exactly 2048 bytes");
    conn.send(with_target(2049));
    expect_refusal(&mut conn, 414, "a target of 2049 bytes (SPEC 5.4)");
}

#[test]
fn a_body_may_be_max_body_bytes_and_not_one_more() {
    let server = Server::start_with(FILES, &["--max-body", "1024"]);

    let mut conn = server.connect();
    let body = format!("\"{}\"", "x".repeat(1022));
    conn.send(post_json("/echo", &body));
    let response = conn.response();
    response.expect_status(200, "a body of exactly --max-body bytes");
    assert_eq!(response.text(), body);
    drop(conn);

    // Only the head is sent. The server must refuse because of the number
    // in Content-Length, without waiting for a body that it will not read.
    let too_large = [
        ("POST /echo", "1025"),
        ("GET /hello.txt", "1025"),
        ("PUT /hello.txt", "1025"),
        ("POST /echo", "4294967296"),
        ("POST /echo", "18446744073709551615"),
        // One more than a u64 can hold. Still digits, so still "too large".
        ("POST /echo", "18446744073709551616"),
        ("POST /echo", "99999999999999999999999999999999999999999999"),
    ];
    for (line, length) in too_large {
        let mut conn = server.connect().patience(Duration::from_secs(2));
        conn.send(format!(
            "{line} HTTP/1.1\r\nHost: test\r\nContent-Type: application/json\r\nContent-Length: {length}\r\n\r\n"
        ));
        expect_refusal(
            &mut conn,
            413,
            &format!("{line} with Content-Length: {length} and --max-body 1024"),
        );
    }
}

#[test]
fn max_body_zero_allows_only_empty_bodies() {
    let server = Server::start_with(FILES, &["--max-body", "0"]);
    let mut conn = server.connect();
    conn.send(post_json("/echo", ""));
    let response = conn.response();
    response.expect_error(
        400,
        "an empty body is allowed by --max-body 0, but it is not JSON",
    );
    response.expect_keep_alive(true, "400 for invalid JSON");
    conn.send(get("/hello.txt"));
    conn.response()
        .expect_status(200, "GET without a body and --max-body 0");
    conn.send("POST /echo HTTP/1.1\r\nHost: test\r\nContent-Type: application/json\r\nContent-Length: 1\r\n\r\n");
    expect_refusal(&mut conn, 413, "one body byte with --max-body 0");
}

#[test]
fn an_error_response_can_be_read_although_more_bytes_were_sent() {
    let server = Server::start_with(FILES, &["--max-body", "1024"]);
    let surplus = [
        // The client announces 20 000 bytes and sends them right behind
        // the head. The server answers 413 after the head and never reads
        // the body.
        (
            post_json("/echo", "x".repeat(20_000)),
            413,
            "a body that is too large, sent together with the head",
        ),
        // A head that is too long: 30 000 bytes in one write.
        (
            format!(
                "GET /hello.txt HTTP/1.1\r\nHost: test\r\nX-Pad: {}\r\n\r\n",
                "a".repeat(30_000)
            )
            .into_bytes(),
            431,
            "a head that is too long, sent in one write",
        ),
        // A malformed head with more data behind it.
        (
            format!("BAD\r\n\r\n{}", "z".repeat(20_000)).into_bytes(),
            400,
            "a malformed head followed by 20 000 more bytes",
        ),
    ];
    // In each case the server answers while bytes of the client are still
    // unread. If it now simply closes the socket, the operating system
    // resets the connection, and a reset can throw the response away
    // before the client has read it. Whether that happens depends on the
    // system and on timing, so this test demands more than "the response
    // arrived": the connection must end with a normal end of stream.
    for (request, status, what) in surplus {
        let mut conn = server.connect();
        conn.send(&request);
        let response = conn.response();
        response.expect_error(status, what);
        response.expect_keep_alive(false, what);
        conn.expect_orderly_close(SLACK, what);
    }
}

#[test]
fn a_client_that_never_closes_cannot_keep_a_worker_after_an_error() {
    let server = Server::start_with(
        FILES,
        &["--workers", "1", "--queue", "1", "--max-body", "1024"],
    );
    // This client is refused with 413. Then it neither sends the rest of
    // its body nor closes the connection.
    let mut lingering = server.connect();
    lingering.send(
        "POST /echo HTTP/1.1\r\nHost: test\r\nContent-Type: application/json\r\nContent-Length: 5000\r\n\r\n[1,",
    );
    let response = lingering.response();
    response.expect_error(413, "a body that is too large");
    response.expect_keep_alive(false, "413");

    // The only worker now reads and throws away what this client may still
    // send (SPEC 7.4, step 2). That wait ends after 1 second at the latest,
    // and then the worker is free for the next connection.
    let mut next = server.connect().patience(Duration::from_secs(1) + SLACK);
    next.send(get("/hello.txt"));
    let response = match next.try_response(true) {
        Ok(response) => response,
        Err(problem) => panic!(
            "a client that got an error response and then stayed connected without closing still held the \
             only worker after {:?}. The read that follows a `Connection: close` response is limited to 1 \
             second (SPEC 7.4).\n  {problem}",
            Duration::from_secs(1) + SLACK
        ),
    };
    response.expect_status(200, "the connection behind a client that never closes");
    drop(lingering);
}

// ---------------------------------------------------------------------------
// Timeouts
// ---------------------------------------------------------------------------

#[test]
fn a_request_that_stalls_in_the_head_gets_408() {
    let server = Server::start_with(
        FILES,
        &["--request-timeout-ms", "500", "--idle-timeout-ms", "20000"],
    );
    for partial in [
        "G",
        "GET /hello.txt HTT",
        "GET /hello.txt HTTP/1.1\r\nHost: te",
        "GET /hello.txt HTTP/1.1\r\nHost: test\r\n",
    ] {
        let mut conn = server.connect();
        // The clock starts before the bytes leave, so the server cannot
        // have started its own clock earlier.
        let started = Instant::now();
        conn.send(partial);
        expect_refusal(
            &mut conn,
            408,
            &format!("a request that stops after {partial:?}"),
        );
        let waited = started.elapsed();
        assert!(
            waited >= Duration::from_millis(450),
            "the 408 came after {waited:?}; with --request-timeout-ms 500 it must not come that early"
        );
        assert!(
            waited <= Duration::from_millis(500) + SLACK,
            "the 408 came after {waited:?}; it is governed by --request-timeout-ms 500, not by the idle \
             timeout of 20 s"
        );
    }
}

#[test]
fn a_request_that_stalls_in_the_body_gets_408() {
    let server = Server::start_with(
        FILES,
        &["--request-timeout-ms", "500", "--idle-timeout-ms", "20000"],
    );
    let mut conn = server.connect();
    let started = Instant::now();
    conn.send("POST /echo HTTP/1.1\r\nHost: test\r\nContent-Type: application/json\r\nContent-Length: 10\r\n\r\n[1,2");
    expect_refusal(&mut conn, 408, "a body that stops after 4 of 10 bytes");
    let waited = started.elapsed();
    assert!(
        (Duration::from_millis(450)..=Duration::from_millis(500) + SLACK).contains(&waited),
        "the 408 came after {waited:?}; expected about 500 ms (--request-timeout-ms 500)"
    );
}

#[test]
fn the_request_timeout_covers_the_whole_request() {
    let server = Server::start_with(
        FILES,
        &["--request-timeout-ms", "800", "--idle-timeout-ms", "20000"],
    );
    let mut conn = server.connect();
    let request = format!(
        "GET /hello.txt HTTP/1.1\r\nHost: test\r\nX-Slow: {}",
        "s".repeat(200)
    );
    let started = Instant::now();
    let mut answered = false;
    // One byte every 100 ms: each read of the server succeeds quickly, but
    // the request as a whole is never finished.
    for byte in request.bytes() {
        if started.elapsed() > Duration::from_millis(800) + SLACK {
            break;
        }
        if conn.try_send([byte]).is_err() {
            answered = true;
            break;
        }
        if conn.wait_for_activity(Duration::from_millis(100)) {
            answered = true;
            break;
        }
    }
    assert!(
        answered,
        "a client that sends one byte every 100 ms was still being listened to after {:?}. The request \
         timeout (800 ms here) is a limit for the whole request, from its first byte to its last, not a \
         limit for each read (SPEC 7.3).",
        started.elapsed()
    );
    expect_refusal(&mut conn, 408, "a request that arrives too slowly");
    assert!(
        started.elapsed() >= Duration::from_millis(750),
        "the 408 came after {:?}, earlier than --request-timeout-ms 800 allows",
        started.elapsed()
    );
}

#[test]
fn an_idle_connection_is_closed_without_a_response() {
    let server = Server::start_with(
        FILES,
        &["--idle-timeout-ms", "1000", "--request-timeout-ms", "20000"],
    );

    // A connection that never sends anything. It must still be open well
    // before the idle timeout ends, and closed soon after.
    let mut conn = server.connect();
    conn.expect_silence(
        Duration::from_millis(400),
        "a new connection, 400 ms into an idle timeout of 1000 ms",
    );
    conn.expect_closed(
        Duration::from_millis(1000) + SLACK,
        "an idle connection gets no response, only the close (SPEC 7.3)",
    );

    // A connection that has been used: the idle time starts again after
    // each response.
    let mut conn = server.connect();
    for _ in 0..3 {
        conn.send(get("/hello.txt"));
        conn.response()
            .expect_status(200, "a request on a connection that is reused");
        conn.expect_silence(
            Duration::from_millis(400),
            "400 ms after a response, with an idle timeout of 1000 ms",
        );
    }
    conn.expect_closed(
        Duration::from_millis(1000) + SLACK,
        "an idle connection after three requests",
    );
}

#[test]
fn the_idle_timeout_does_not_limit_a_request_in_progress() {
    let server = Server::start_with(
        FILES,
        &["--idle-timeout-ms", "500", "--request-timeout-ms", "20000"],
    );
    let mut conn = server.connect();
    conn.send("GET /hello.txt HTTP/1.1\r\nHo");
    // Three times the idle timeout. The connection is not idle: a request
    // has started, so only the request timeout (20 s) applies.
    thread::sleep(Duration::from_millis(1500));
    conn.send("st: test\r\n\r\n");
    conn.response().expect_status(
        200,
        "a request that pauses for longer than the idle timeout (SPEC 7.3)",
    );

    // And the other way round: a short request timeout does not shorten
    // the idle time.
    let server = Server::start_with(
        FILES,
        &["--idle-timeout-ms", "20000", "--request-timeout-ms", "300"],
    );
    let mut conn = server.connect();
    conn.expect_silence(
        Duration::from_millis(900),
        "an idle connection with a 300 ms request timeout",
    );
    conn.send(get("/hello.txt"));
    conn.response()
        .expect_status(200, "a request after an idle second");
    conn.expect_silence(Duration::from_millis(900), "idle again after a response");
}

#[test]
fn a_client_that_does_not_read_cannot_keep_a_worker() {
    let site = Site::new(FILES);
    // Far more than the socket buffers between the two processes can hold.
    site.write("big.bin", pattern_bytes(32 * 1024 * 1024));
    let server = Server::start_site(
        site,
        &[
            "--workers",
            "1",
            "--queue",
            "1",
            "--request-timeout-ms",
            "500",
        ],
    );

    // This client asks for the big file and then never reads.
    let mut stuck = server.connect();
    stuck.send(get("/big.bin"));
    thread::sleep(Duration::from_millis(100));

    // The only worker is now blocked in a write. After 500 ms without
    // progress it must give up, and then this second client is served.
    let mut waiting = server
        .connect()
        .patience(Duration::from_millis(500) + SLACK + SLACK);
    waiting.send(get("/hello.txt"));
    let response = waiting.response();
    response.expect_status(
        200,
        "a request behind a client that does not read its response (SPEC 7.3: the request timeout also limits \
         every write)",
    );
    drop(stuck);
}

#[test]
fn a_slow_but_steady_reader_gets_the_whole_file() {
    const SIZE: usize = 32 * 1024 * 1024;
    let site = Site::new(FILES);
    let big = pattern_bytes(SIZE);
    site.write("big.bin", &big);
    let server = Server::start_site(site, &["--request-timeout-ms", "1000"]);

    let mut conn = server.connect();
    conn.send("GET /big.bin HTTP/1.1\r\nHost: test\r\nConnection: close\r\n\r\n");
    // This client reads about 10 MiB per second, in steps of at most 1 MiB,
    // so the file takes more than three seconds: three times the request
    // timeout. The server makes progress all the time, so it must not give
    // up (SPEC 7.3: the limit is for a write that makes no progress, not
    // for the response as a whole).
    let mut stream = conn.writer();
    let mut received = Vec::with_capacity(SIZE + 1024);
    let mut chunk = vec![0u8; 1024 * 1024];
    let started = Instant::now();
    let mut expected = None;
    loop {
        // The socket has a read timeout, so this cannot wait for ever.
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => received.extend_from_slice(&chunk[..n]),
            Err(e) => panic!(
                "reading the file failed after {} of {SIZE} bytes: {e}",
                received.len()
            ),
        }
        if expected.is_none() {
            expected = find(&received, b"\r\n\r\n").map(|head_end| head_end + 4 + SIZE);
        }
        if expected.is_some_and(|total| received.len() >= total) {
            break;
        }
        let due = started + Duration::from_millis((received.len() as u64 * 100) >> 20);
        thread::sleep(due.saturating_duration_since(Instant::now()));
    }

    let head_end = find(&received, b"\r\n\r\n")
        .unwrap_or_else(|| panic!("no response head in {} bytes", received.len()));
    let head = String::from_utf8_lossy(&received[..head_end]);
    assert!(
        head.starts_with("HTTP/1.1 200 OK\r\n"),
        "GET /big.bin: expected 200, got {head:?}"
    );
    let body = &received[head_end + 4..];
    assert_eq!(
        body.len(),
        SIZE,
        "a client that reads slowly but steadily received {} of {SIZE} bytes in {:?}; then the server \
         closed the connection. With --request-timeout-ms 1000, a write may be given up when it makes no \
         progress for one second. A response that takes longer than that in total is fine (SPEC 7.3).",
        body.len(),
        started.elapsed()
    );
    assert!(body == big, "the 32 MiB file must arrive unchanged");
}
