//! Milestone M4: request bodies, `POST /echo`, and persistent connections.
//! SPEC sections 4 (steps 3 to 6), 7.1, 7.2 and 9.
//!
//! A server that handles one connection at a time passes this file.

mod common;

use std::thread;
use std::time::{Duration, Instant};

use common::{Conn, SLACK, Server, get, head, post_json};

const FILES: &[(&str, &str)] = &[("hello.txt", "hello\n"), ("other.txt", "another file\n")];

#[track_caller]
fn server() -> Server {
    Server::start(FILES)
}

/// Posts `body` to `/echo` as JSON and returns the response.
#[track_caller]
fn echo(conn: &mut Conn, body: impl AsRef<[u8]>) -> common::Response {
    conn.send(post_json("/echo", body));
    conn.response()
}

// ---------------------------------------------------------------------------
// POST /echo
// ---------------------------------------------------------------------------

#[test]
fn echo_returns_the_value_without_insignificant_whitespace() {
    let server = server();
    let mut conn = server.connect();
    let cases: &[(&str, &str)] = &[
        (r#"{"a":1}"#, r#"{"a":1}"#),
        (
            "  { \"a\" : [ 1 , 2.50 , \"x y\" ] ,\r\n\t\"b\" : null }\n",
            r#"{"a":[1,2.50,"x y"],"b":null}"#,
        ),
        ("[ ]", "[]"),
        ("{ }", "{}"),
        ("[\n  [ ],\n  { }\n]", "[[],{}]"),
        (" 42 ", "42"),
        ("\t\r\n true \n", "true"),
        ("false", "false"),
        ("null", "null"),
        ("\"text\"", "\"text\""),
        // Whitespace inside a string is content.
        (
            "[ \"  two  spaces  \" , \"tab\\there\" ]",
            "[\"  two  spaces  \",\"tab\\there\"]",
        ),
        // Numbers are copied, not converted: no rounding, no reformatting.
        (
            "[ -0 , 0.5 , 1e10 , 1E+2 , -1.5e-3 , 2.50 , 123456789012345678901234567890.123456789e+400 ]",
            "[-0,0.5,1e10,1E+2,-1.5e-3,2.50,123456789012345678901234567890.123456789e+400]",
        ),
        // Escapes are copied, not decoded.
        (
            r#"[ "a\"b\\c\/d\b\f\n\r\t" , "\u00e9\uD83D\uDE00\ud83d" ]"#,
            r#"["a\"b\\c\/d\b\f\n\r\t","\u00e9\uD83D\uDE00\ud83d"]"#,
        ),
        (
            "[ \"h\u{e9}llo \u{2603} \u{1F600}\" ]",
            "[\"h\u{e9}llo \u{2603} \u{1F600}\"]",
        ),
        // Members keep their order; repeated names are not an error.
        (
            r#"{ "b" : 1 , "a" : 2 , "b" : 3 }"#,
            r#"{"b":1,"a":2,"b":3}"#,
        ),
        (
            r#"{ "" : { "" : [ { } , [ [ ] ] ] } }"#,
            r#"{"":{"":[{},[[]]]}}"#,
        ),
    ];
    for (sent, expected) in cases {
        let response = echo(&mut conn, sent);
        response.expect_status(200, &format!("POST /echo with body {sent:?}"));
        assert_eq!(
            response.text(),
            *expected,
            "POST /echo with body {sent:?}: every token byte for byte, no whitespace between tokens (SPEC 9.3)"
        );
        assert_eq!(
            response.header("content-type"),
            Some("application/json"),
            "Content-Type of an echo response"
        );
        response.expect_keep_alive(true, "a successful echo");
    }
}

#[test]
fn invalid_json_is_400_and_the_connection_stays_open() {
    let server = server();
    let mut conn = server.connect();
    let cases: &[&str] = &[
        "",
        "   ",
        "{",
        "[",
        "]",
        "}",
        "[1,]",
        "[,1]",
        "[1 2]",
        "[1,,2]",
        r#"{"a"}"#,
        r#"{"a":}"#,
        r#"{"a":1,}"#,
        r#"{"a":1 "b":2}"#,
        r#"{"a":1,,"b":2}"#,
        r#"{"a" 1}"#,
        "{a:1}",
        "{'a':1}",
        "{1:2}",
        "{,}",
        r#"{"a":1]"#,
        "[1}",
        "[01]",
        "[1.]",
        "[.5]",
        "[+1]",
        "[1e]",
        "[1e+]",
        "[-]",
        "-",
        "[0x10]",
        "[1_000]",
        "tru",
        "nul",
        "nulll",
        "True",
        "NaN",
        "Infinity",
        "undefined",
        "[true false]",
        "\"unterminated",
        r#""bad \x escape""#,
        r#""\u12""#,
        r#""\u12G4""#,
        r#""\U0041""#,
        "\"a line\nbreak\"",
        "\"a real\ttab\"",
        "[1]]",
        "{}{}",
        "1 2",
        "[1],",
        "[1] x",
        "// comment\n[1]",
        "[1] /* comment */",
        "\u{feff}{}",
        "[1]\u{c}",
        "[\u{a0}1]",
    ];
    for body in cases {
        let response = echo(&mut conn, body);
        response.expect_error(400, &format!("POST /echo with the invalid body {body:?}"));
        response.expect_keep_alive(
            true,
            "400 for invalid JSON: the request itself was complete and well framed (SPEC 7.2)",
        );
    }
    let response = echo(&mut conn, "[ 1 ]");
    response.expect_status(
        200,
        "a valid echo after many invalid ones on the same connection",
    );
    assert_eq!(response.text(), "[1]");
}

#[test]
fn a_body_that_is_not_utf8_is_400() {
    let server = server();
    let mut conn = server.connect();
    let cases: &[&[u8]] = &[
        b"\"\xFF\"",
        b"\"caf\xE9\"",
        b"\"\xC3\"",
        b"\"\xC0\xAF\"",
        b"\"\xED\xA0\x80\"",
        b"[1]\xFF",
    ];
    for body in cases {
        let response = echo(&mut conn, body);
        response.expect_error(
            400,
            &format!("POST /echo with bytes that are not UTF-8: {body:?}"),
        );
    }
}

#[test]
fn nesting_is_limited_to_64_levels() {
    let server = server();
    let mut conn = server.connect();
    let arrays = |depth: usize| format!("{}{}", "[".repeat(depth), "]".repeat(depth));

    let response = echo(&mut conn, arrays(64));
    response.expect_status(200, "64 nested arrays");
    assert_eq!(response.text(), arrays(64));
    echo(&mut conn, arrays(65)).expect_error(400, "65 nested arrays (SPEC 9.2)");

    // Arrays and objects count alike: 32 objects that each hold an array.
    let mixed = format!("{}1{}", "{\"k\":[".repeat(32), "]}".repeat(32));
    echo(&mut conn, &mixed).expect_status(200, "32 objects and 32 arrays, nested alternately");
    echo(&mut conn, format!("[{mixed}]"))
        .expect_error(400, "the same inside one more array: 65 levels");

    // Values next to each other do not add depth.
    let wide = format!("[{}[]]", "[[]],".repeat(5000));
    echo(&mut conn, &wide).expect_status(200, "5001 shallow arrays next to each other");

    // 100 000 open brackets: the answer must be 400, not a crash.
    echo(&mut conn, "[".repeat(100_000)).expect_error(400, "100 000 open brackets");
    echo(&mut conn, "{\"a\":".repeat(100_000)).expect_error(400, "100 000 nested objects");
    drop(conn);
    server
        .get("/hello.txt")
        .expect_status(200, "the server is still alive after deeply nested input");
}

#[test]
fn echo_needs_the_json_content_type() {
    let server = server();
    let mut conn = server.connect();
    let post = |content_type: Option<&str>| {
        let header =
            content_type.map_or(String::new(), |value| format!("Content-Type: {value}\r\n"));
        format!("POST /echo HTTP/1.1\r\nHost: test\r\n{header}Content-Length: 3\r\n\r\n[1]")
    };
    for accepted in [
        "application/json",
        "application/json; charset=utf-8",
        "application/json;charset=utf-8",
        "application/json ; x=y",
        "Application/JSON",
    ] {
        conn.send(post(Some(accepted)));
        conn.response()
            .expect_status(200, &format!("Content-Type: {accepted}"));
    }
    for refused in [
        Some("text/plain"),
        Some("application/jsonx"),
        Some("application/x-www-form-urlencoded"),
        Some("json"),
        Some("text/json"),
        Some(""),
        None,
    ] {
        conn.send(post(refused));
        let response = conn.response();
        response.expect_error(415, &format!("Content-Type: {refused:?}"));
        response.expect_keep_alive(true, "415: the request was complete");
    }
    // If the header occurs more than once, the first line counts (SPEC 5.3).
    let twice = |first: &str, second: &str| {
        format!(
            "POST /echo HTTP/1.1\r\nHost: test\r\nContent-Type: {first}\r\nContent-Type: {second}\r\n\
             Content-Length: 3\r\n\r\n[1]"
        )
    };
    conn.send(twice("application/json", "text/plain"));
    conn.response().expect_status(
        200,
        "two Content-Type lines; the first one says application/json",
    );
    conn.send(twice("text/plain", "application/json"));
    conn.response()
        .expect_error(415, "two Content-Type lines; the first one says text/plain");
    // The type is checked before the body: 415, not 400, for a bad body.
    conn.send("POST /echo HTTP/1.1\r\nHost: test\r\nContent-Type: text/plain\r\nContent-Length: 1\r\n\r\n{");
    conn.response()
        .expect_error(415, "a wrong type and invalid JSON together");
}

#[test]
fn the_query_string_does_not_change_the_echo_endpoint() {
    let server = server();
    let mut conn = server.connect();
    conn.send(post_json("/echo?pretty=no", "[ 1 ]"));
    let response = conn.response();
    response.expect_status(200, "POST /echo?pretty=no");
    assert_eq!(response.text(), "[1]");
    // Only the exact path `/echo` is the endpoint.
    for other in ["/echo/", "/echo2", "/Echo", "/%65cho"] {
        conn.send(post_json(other, "[1]"));
        conn.response()
            .expect_error(405, &format!("POST {other} is a POST to a file path"));
    }
}

#[test]
fn a_body_may_arrive_in_pieces() {
    let server = server();
    let mut conn = server.connect();
    let body = r#"{ "piece" : [ 1 , 2 , 3 ] }"#;
    let request = post_json("/echo", body);
    let head_len = request.len() - body.len();
    for chunk in [
        &request[..head_len],
        &request[head_len..head_len + 5],
        &request[head_len + 5..],
    ] {
        conn.send(chunk);
        thread::sleep(Duration::from_millis(40));
    }
    let response = conn.response();
    response.expect_status(200, "a body sent in two pieces after the head");
    assert_eq!(response.text(), r#"{"piece":[1,2,3]}"#);

    // And byte by byte, starting in the middle of the head.
    let request = post_json("/echo", "[ true ]");
    conn.send(&request[..20]);
    for byte in &request[20..] {
        conn.send([*byte]);
        thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(conn.response().text(), "[true]");
}

#[test]
fn a_body_of_exactly_the_default_limit_is_echoed() {
    let server = server();
    let mut conn = server.connect().patience(Duration::from_secs(20));
    // One JSON string of exactly 1 048 576 bytes, the default --max-body.
    let body = format!("\"{}\"", "abcdefgh".repeat((1_048_576 - 2) / 8) + "abcdef");
    assert_eq!(body.len(), 1_048_576, "test setup");
    // The request is larger than any socket buffer, so it is sent from a
    // second thread while this one reads: a server may answer early with
    // an error, and then nobody would be reading.
    let request = post_json("/echo", &body);
    let response = thread::scope(|scope| {
        let mut writer = conn.writer();
        scope.spawn(move || {
            let _ = std::io::Write::write_all(&mut writer, &request);
        });
        conn.response()
    });
    response.expect_status(200, "a JSON body of exactly 1 MiB");
    assert!(
        response.body == body.as_bytes(),
        "the 1 MiB string must come back unchanged"
    );
}

// ---------------------------------------------------------------------------
// Framing: Content-Length and Transfer-Encoding
// ---------------------------------------------------------------------------

/// Sends `request` on a new connection and expects the standard error
/// response `status` with `Connection: close`, followed by the close.
#[track_caller]
fn expect_refused(server: &Server, request: impl AsRef<[u8]>, status: u16, context: &str) {
    let mut conn = server.connect();
    conn.send(request);
    let response = conn.response();
    response.expect_error(status, context);
    response.expect_keep_alive(false, context);
    conn.expect_closed(SLACK, context);
}

#[test]
fn post_without_content_length_is_411() {
    let server = server();
    expect_refused(
        &server,
        "POST /echo HTTP/1.1\r\nHost: test\r\nContent-Type: application/json\r\n\r\n",
        411,
        "POST /echo without Content-Length",
    );
    expect_refused(
        &server,
        "POST /hello.txt HTTP/1.1\r\nHost: test\r\n\r\n",
        411,
        "POST to a file without Content-Length: the length is checked before the method is routed",
    );
}

#[test]
fn content_length_must_be_one_plain_number() {
    let server = server();
    for value in [
        "", "+3", "-3", "3 3", "3,3", "0x3", "3e0", "3.0", "three", "\"3\"",
    ] {
        let request = format!(
            "POST /echo HTTP/1.1\r\nHost: test\r\nContent-Type: application/json\r\nContent-Length: {value}\r\n\r\n[1]"
        );
        expect_refused(&server, request, 400, &format!("Content-Length: {value:?}"));
    }
    expect_refused(
        &server,
        "POST /echo HTTP/1.1\r\nHost: test\r\nContent-Type: application/json\r\nContent-Length: 3\r\n\
         Content-Length: 3\r\n\r\n[1]",
        400,
        "two Content-Length headers, even with the same value",
    );
    expect_refused(
        &server,
        "GET /hello.txt HTTP/1.1\r\nHost: test\r\nContent-Length: 0\r\ncontent-length: 0\r\n\r\n",
        400,
        "two Content-Length headers on a GET",
    );

    // Leading zeros are digits like any other.
    let mut conn = server.connect();
    conn.send(
        "POST /echo HTTP/1.1\r\nHost: test\r\nContent-Type: application/json\r\nContent-Length: 0003\r\n\r\n[1]",
    );
    conn.response().expect_status(200, "Content-Length: 0003");
}

#[test]
fn transfer_encoding_is_501() {
    let server = server();
    expect_refused(
        &server,
        "POST /echo HTTP/1.1\r\nHost: test\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n\r\n",
        501,
        "a chunked POST",
    );
    expect_refused(
        &server,
        "GET /hello.txt HTTP/1.1\r\nHost: test\r\nTransfer-Encoding: gzip\r\n\r\n",
        501,
        "Transfer-Encoding on a GET",
    );
    expect_refused(
        &server,
        "POST /echo HTTP/1.1\r\nHost: test\r\nContent-Type: application/json\r\nContent-Length: 3\r\n\
         Transfer-Encoding: chunked\r\n\r\n",
        501,
        "Transfer-Encoding together with Content-Length",
    );
    // Step 3 checks in a fixed order, and Transfer-Encoding comes first:
    // the broken Content-Length lines behind it are never looked at.
    expect_refused(
        &server,
        "POST /echo HTTP/1.1\r\nHost: test\r\nContent-Type: application/json\r\nContent-Length: 3\r\n\
         Content-Length: three\r\nTransfer-Encoding: chunked\r\n\r\n",
        501,
        "Transfer-Encoding together with two Content-Length lines, one of them not a number (SPEC 4, step 3)",
    );
}

// ---------------------------------------------------------------------------
// Persistent connections
// ---------------------------------------------------------------------------

#[test]
fn many_requests_on_one_connection() {
    let server = server();
    let mut conn = server.connect();
    for round in 0..25 {
        let context = format!("round {round} on one connection");
        conn.send(get("/hello.txt"));
        let response = conn.response();
        response.expect_status(200, &context);
        assert_eq!(response.text(), "hello\n", "{context}");

        let response = echo(&mut conn, format!("[ {round} , \"x\" ]"));
        assert_eq!(response.text(), format!("[{round},\"x\"]"), "{context}");

        conn.send(head("/other.txt"));
        assert_eq!(conn.head_response().content_length(), 13, "{context}");

        conn.send(get("/missing"));
        conn.response().expect_error(404, &context);
    }
}

#[test]
fn connection_close_is_honoured() {
    let server = server();
    for value in [
        "close",
        "Close",
        "CLOSE",
        "keep-alive, close",
        "close, TE",
        "foo,close ,bar",
    ] {
        let mut conn = server.connect();
        conn.send(get("/hello.txt"));
        conn.response()
            .expect_keep_alive(true, "the first request did not ask to close");
        conn.send(format!(
            "GET /hello.txt HTTP/1.1\r\nHost: test\r\nConnection: {value}\r\n\r\n"
        ));
        let response = conn.response();
        response.expect_status(200, &format!("Connection: {value}"));
        response.expect_keep_alive(false, &format!("the request said `Connection: {value}`"));
        conn.expect_closed(SLACK, &format!("after `Connection: {value}`"));
    }
    // A token that only contains the letters of `close` is not `close`.
    let mut conn = server.connect();
    conn.send("GET /hello.txt HTTP/1.1\r\nHost: test\r\nConnection: closed, enclose\r\n\r\n");
    conn.response().expect_keep_alive(
        true,
        "`Connection: closed, enclose` does not contain the token `close`",
    );
    // The token may be in any of several Connection lines.
    conn.send("GET /hello.txt HTTP/1.1\r\nHost: test\r\nConnection: keep-alive\r\nConnection: close\r\n\r\n");
    conn.response()
        .expect_keep_alive(false, "two Connection lines, the second says close");
}

#[test]
fn http_1_0_closes_unless_asked_to_keep_alive() {
    let server = server();

    let mut conn = server.connect();
    conn.send("GET /hello.txt HTTP/1.0\r\n\r\n");
    let response = conn.response();
    response.expect_status(200, "an HTTP/1.0 request");
    response.expect_keep_alive(false, "HTTP/1.0 without `Connection: keep-alive`");
    conn.expect_closed(SLACK, "after an HTTP/1.0 response");
    // Closed on this side too before the next connection is opened, so
    // that a server that handles one connection at a time can pass this
    // file.
    drop(conn);

    let mut conn = server.connect();
    for round in 1..=3 {
        conn.send("GET /hello.txt HTTP/1.0\r\nConnection: Keep-Alive\r\n\r\n");
        let response = conn.response();
        response.expect_status(200, &format!("HTTP/1.0 with keep-alive, request {round}"));
        response.expect_keep_alive(true, "HTTP/1.0 with `Connection: Keep-Alive`");
    }
    conn.send("GET /hello.txt HTTP/1.0\r\nConnection: keep-alive, close\r\n\r\n");
    conn.response()
        .expect_keep_alive(false, "`close` wins over `keep-alive`");
    conn.expect_closed(SLACK, "after `Connection: keep-alive, close`");
}

#[test]
fn pipelined_requests_are_answered_in_order() {
    let server = server();
    let mut conn = server.connect();
    // Five requests in a single write, before any response is read.
    let mut batch = Vec::new();
    batch.extend_from_slice(get("/hello.txt").as_bytes());
    batch.extend_from_slice(&post_json("/echo", "{ \"n\" : 2 }"));
    batch.extend_from_slice(head("/other.txt").as_bytes());
    batch.extend_from_slice(get("/missing").as_bytes());
    batch.extend_from_slice(b"GET /other.txt HTTP/1.1\r\nHost: test\r\nConnection: close\r\n\r\n");
    conn.send(batch);

    let first = conn.response();
    first.expect_status(200, "first of five pipelined requests");
    assert_eq!(first.text(), "hello\n");
    let second = conn.response();
    assert_eq!(
        second.text(),
        "{\"n\":2}",
        "second of five pipelined requests"
    );
    let third = conn.head_response();
    assert_eq!(
        third.content_length(),
        13,
        "third of five pipelined requests (HEAD)"
    );
    conn.response()
        .expect_error(404, "fourth of five pipelined requests");
    let fifth = conn.response();
    assert_eq!(
        fifth.text(),
        "another file\n",
        "fifth of five pipelined requests"
    );
    fifth.expect_keep_alive(false, "the fifth request said `Connection: close`");
    conn.expect_closed(SLACK, "after the last pipelined response");
}

#[test]
fn a_body_on_get_is_read_and_thrown_away() {
    let server = server();
    let mut conn = server.connect();
    // The body looks like a request. If the server does not skip it, it
    // answers three times, and the second answer is a 404.
    let body = "GET /missing HTTP/1.1\r\nHost: test\r\n\r\n";
    let mut batch = format!(
        "GET /hello.txt HTTP/1.1\r\nHost: test\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    batch.push_str(&get("/other.txt"));
    conn.send(batch);
    let first = conn.response();
    first.expect_status(200, "a GET with a body");
    assert_eq!(first.text(), "hello\n");
    let second = conn.response();
    second.expect_status(
        200,
        "the request after a GET with a body: the body must be skipped (SPEC 4, step 4)",
    );
    assert_eq!(second.text(), "another file\n");
    conn.expect_silence(
        Duration::from_millis(300),
        "two requests were sent, so two responses are due",
    );
}

#[test]
fn errors_after_a_complete_request_keep_the_connection() {
    let server = server();
    let mut conn = server.connect();
    let cases: &[(&str, u16)] = &[
        ("GET /missing HTTP/1.1\r\nHost: test\r\n\r\n", 404),
        ("GET /../hello.txt HTTP/1.1\r\nHost: test\r\n\r\n", 403),
        ("GET /%zz HTTP/1.1\r\nHost: test\r\n\r\n", 400),
        (
            "POST /hello.txt HTTP/1.1\r\nHost: test\r\nContent-Length: 2\r\n\r\n[]",
            405,
        ),
        (
            "POST /echo HTTP/1.1\r\nHost: test\r\nContent-Length: 2\r\n\r\n[]",
            415,
        ),
        (
            "POST /echo HTTP/1.1\r\nHost: test\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n[}",
            400,
        ),
        (
            "PUT /hello.txt HTTP/1.1\r\nHost: test\r\nContent-Length: 5\r\n\r\nhello",
            501,
        ),
        ("DELETE /hello.txt HTTP/1.1\r\nHost: test\r\n\r\n", 501),
    ];
    for (request, status) in cases {
        conn.send(request);
        let response = conn.response();
        let first_line = request.lines().next().unwrap_or_default();
        response.expect_error(*status, first_line);
        response.expect_keep_alive(true, first_line);
        conn.send(get("/hello.txt"));
        conn.response().expect_status(
            200,
            &format!("the request after `{first_line}` on the same connection"),
        );
    }
}

#[test]
fn two_hundred_requests_take_far_less_than_four_seconds() {
    let server = server();
    let mut conn = server.connect();
    conn.send(get("/hello.txt"));
    conn.response().expect_status(200, "warm-up");
    let started = Instant::now();
    for _ in 0..100 {
        conn.send(get("/hello.txt"));
        assert_eq!(conn.response().text(), "hello\n");
        let response = echo(&mut conn, "[ 1 ]");
        assert_eq!(response.text(), "[1]");
    }
    let took = started.elapsed();
    assert!(
        took < Duration::from_secs(4),
        "200 small requests on one connection took {took:?}. A healthy server needs a few hundredths of a \
         second. About 40 ms per request usually means that a response is sent with two small writes and \
         the second waits for the client's delayed acknowledgement (SPEC 6.5)."
    );
}
