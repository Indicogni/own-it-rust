//! Milestone M2: the request head. SPEC section 5 (grammar), the status
//! codes of section 4 that come from the head, and the `Date` header.
//!
//! Every test uses its own server and usually one connection, so a server
//! that handles one connection at a time passes this file.

mod common;

use std::thread;
use std::time::Duration;

use common::{Conn, SLACK, Server, get, head, parse_http_date, unix_now};

const FILES: &[(&str, &str)] = &[("hello.txt", "hello\n"), ("index.html", "<h1>home</h1>\n")];

/// Sends `request` on a new connection and returns the connection and the
/// response.
#[track_caller]
fn exchange(server: &Server, request: impl AsRef<[u8]>) -> (Conn, common::Response) {
    let mut conn = server.connect();
    conn.send(request);
    let response = conn.response();
    (conn, response)
}

/// Sends `request` and expects the standard error response `status`, with
/// `Connection: close`, followed by the close.
#[track_caller]
fn expect_refused(server: &Server, request: impl AsRef<[u8]>, status: u16, context: &str) {
    let (mut conn, response) = exchange(server, request);
    response.expect_error(status, context);
    response.expect_keep_alive(false, context);
    conn.expect_closed(SLACK, context);
}

#[test]
fn malformed_request_lines_are_400() {
    let server = Server::start(FILES);
    let cases: &[(&[u8], &str)] = &[
        (b"GET /hello.txt", "two parts instead of three"),
        (b"GET /hello.txt HTTP/1.1 extra", "four parts"),
        (b"GET  /hello.txt HTTP/1.1", "two spaces after the method"),
        (b"GET /hello.txt  HTTP/1.1", "two spaces after the target"),
        (b" GET /hello.txt HTTP/1.1", "a space before the method"),
        (b"GET /hello.txt HTTP/1.1 ", "a space after the version"),
        (b"GET\t/hello.txt\tHTTP/1.1", "tabs instead of spaces"),
        (b"G@T /hello.txt HTTP/1.1", "`@` is not allowed in a method"),
        (
            b"GET hello.txt HTTP/1.1",
            "the target does not start with `/`",
        ),
        (
            b"GET http://test/hello.txt HTTP/1.1",
            "a target in absolute form",
        ),
        (b"GET * HTTP/1.1", "the target `*`"),
        (
            b"GET /hel\x01lo.txt HTTP/1.1",
            "a control character in the target",
        ),
        (b"GET /hel\x7Flo.txt HTTP/1.1", "DEL in the target"),
        (
            b"GET /caf\xC3\xA9.txt HTTP/1.1",
            "bytes above 0x7E in the target",
        ),
        (b"GET /hello.txt http/1.1", "the version in lower case"),
        (b"GET /hello.txt HTTP/1", "a version without a minor number"),
        (
            b"GET /hello.txt HTTP/1.10",
            "a version with a two-digit minor number",
        ),
        (b"GET /hello.txt HTTP/11", "a version without a dot"),
        (b"GET /hello.txt FTP/1.1", "a version that is not HTTP"),
        (b"", "an empty line before the request line"),
    ];
    for (line, what) in cases {
        let mut request = line.to_vec();
        request.extend_from_slice(b"\r\nHost: test\r\n\r\n");
        expect_refused(&server, request, 400, what);
    }
}

#[test]
fn a_well_formed_but_unsupported_version_is_505() {
    let server = Server::start(FILES);
    for version in ["HTTP/2.0", "HTTP/0.9", "HTTP/1.2", "HTTP/3.0", "HTTP/9.9"] {
        let request = format!("GET /hello.txt {version}\r\nHost: test\r\n\r\n");
        expect_refused(&server, request, 505, version);
    }
}

#[test]
fn unknown_methods_are_501_and_the_connection_stays_open() {
    let server = Server::start(FILES);
    let mut conn = server.connect();
    // `get` is a token like any other: method names are case-sensitive.
    for method in [
        "PUT",
        "DELETE",
        "OPTIONS",
        "PATCH",
        "BREW",
        "get",
        "M-SEARCH",
        "X!#$%&'*+-.^_`|~9",
    ] {
        conn.send(format!(
            "{method} /hello.txt HTTP/1.1\r\nHost: test\r\n\r\n"
        ));
        let response = conn.response();
        response.expect_error(501, method);
        response.expect_keep_alive(true, "501 for an unknown method: the request was complete");
    }
    conn.send(get("/hello.txt"));
    conn.response().expect_status(
        200,
        "a GET after eight 501 responses on the same connection",
    );
}

#[test]
fn the_wrong_method_for_a_resource_is_405_with_allow() {
    let server = Server::start(FILES);
    let mut conn = server.connect();
    let allowed = |response: &common::Response| -> Vec<String> {
        let allow = response
            .header("allow")
            .expect("a 405 response must have an Allow header (SPEC 4, step 5)");
        let mut methods: Vec<String> = allow
            .split(',')
            .map(|method| method.trim().to_string())
            .collect();
        methods.sort();
        methods
    };

    conn.send("POST /hello.txt HTTP/1.1\r\nHost: test\r\nContent-Length: 0\r\n\r\n");
    let response = conn.response();
    response.expect_error(405, "POST to a file");
    assert_eq!(allowed(&response), ["GET", "HEAD"], "Allow for a file path");
    response.expect_keep_alive(true, "405");

    conn.send("POST /no-such-file HTTP/1.1\r\nHost: test\r\nContent-Length: 0\r\n\r\n");
    let response = conn.response();
    response.expect_error(
        405,
        "POST to a path that is not /echo: the method is checked before the path",
    );
    assert_eq!(allowed(&response), ["GET", "HEAD"], "Allow for a file path");

    conn.send(get("/echo"));
    let response = conn.response();
    response.expect_error(405, "GET /echo");
    assert_eq!(allowed(&response), ["POST"], "Allow for /echo");

    conn.send(head("/echo"));
    let response = conn.head_response();
    response.expect_status(405, "HEAD /echo");
    assert_eq!(allowed(&response), ["POST"], "Allow for /echo");
    assert_eq!(
        response.content_length(),
        23,
        "a HEAD response has the Content-Length of the same GET: `405 Method Not Allowed` and a line feed"
    );

    conn.send(get("/hello.txt"));
    conn.response()
        .expect_status(200, "a GET after four 405 responses on the same connection");
}

#[test]
fn http_1_1_needs_exactly_one_host_header() {
    let server = Server::start(FILES);
    expect_refused(
        &server,
        "GET /hello.txt HTTP/1.1\r\n\r\n",
        400,
        "HTTP/1.1 without Host",
    );
    expect_refused(
        &server,
        "GET /hello.txt HTTP/1.1\r\nHost: test\r\nHost: test\r\n\r\n",
        400,
        "two Host headers",
    );
    expect_refused(
        &server,
        "GET /hello.txt HTTP/1.0\r\nHost: a\r\nhost: b\r\n\r\n",
        400,
        "two Host headers in HTTP/1.0",
    );
    let (conn, response) = exchange(&server, "GET /hello.txt HTTP/1.0\r\n\r\n");
    response.expect_status(200, "HTTP/1.0 does not need a Host header");
    // This connection is closed before the next one is opened, so that a
    // server that handles one connection at a time can pass this file.
    drop(conn);
    let (_conn, response) = exchange(&server, "GET /hello.txt HTTP/1.1\r\nHost:\r\n\r\n");
    response.expect_status(200, "an empty Host value is still a Host header");
}

#[test]
fn malformed_header_lines_are_400() {
    let server = Server::start(FILES);
    let cases: &[(&[u8], &str)] = &[
        (b"X-Test : 1", "a space before the colon"),
        (b"X-Test", "a line without a colon"),
        (b": 1", "an empty name"),
        (
            b" X-Test: 1",
            "a line that starts with a space (obsolete line folding)",
        ),
        (
            b"\tfolded",
            "a line that starts with a tab (obsolete line folding)",
        ),
        (b"X Test: 1", "a space inside the name"),
        (b"X(Test): 1", "parentheses in the name"),
        (b"X-T\xC3\xA9st: 1", "bytes above 0x7E in the name"),
        (b"X-Test: a\x00b", "NUL in the value"),
        (b"X-Test: a\x01b", "a control character in the value"),
        (b"X-Test: a\x7Fb", "DEL in the value"),
        (
            b"X-Test: a\rb",
            "a carriage return that is not followed by a line feed",
        ),
    ];
    for (line, what) in cases {
        let mut request = b"GET /hello.txt HTTP/1.1\r\nHost: test\r\n".to_vec();
        request.extend_from_slice(line);
        request.extend_from_slice(b"\r\n\r\n");
        expect_refused(&server, request, 400, what);
    }
}

#[test]
fn a_bare_line_feed_is_refused_at_once() {
    let server = Server::start(FILES);
    // The server's request timeout is 5 s. A server that waits for
    // CR LF CR LF here answers 408 much later, or not at all in time.
    for request in [
        "GET /hello.txt HTTP/1.1\nHost: test\n\n",
        "GET /hello.txt HTTP/1.1\r\nHost: test\n",
        "\nGET /hello.txt HTTP/1.1\r\nHost: test\r\n\r\n",
    ] {
        let mut conn = server.connect().patience(Duration::from_secs(2));
        conn.send(request);
        let response = conn.response();
        response.expect_error(
            400,
            "a line feed without a carriage return before it (SPEC 5.1)",
        );
        response.expect_keep_alive(false, "400 for a malformed head");
        conn.expect_closed(SLACK, "after the 400");
    }
}

#[test]
fn header_names_ignore_case_and_values_are_trimmed() {
    let server = Server::start(FILES);
    let (mut conn, response) = exchange(
        &server,
        "GET /hello.txt HTTP/1.1\r\nhOsT:   test  \r\ncOnNeCtIoN:\t  ClOsE \t \r\n\r\n",
    );
    response.expect_status(200, "mixed-case header names and padded values");
    response.expect_keep_alive(
        false,
        "`cOnNeCtIoN: ClOsE` with spaces and tabs around the value",
    );
    conn.expect_closed(SLACK, "after `Connection: close`");
}

#[test]
fn unusual_but_valid_header_lines_are_accepted() {
    let server = Server::start(FILES);
    let mut request = b"GET /hello.txt HTTP/1.1\r\nHost: test\r\n".to_vec();
    request.extend_from_slice(b"X-Empty:\r\n");
    request.extend_from_slice(b"X-Only-Spaces:   \r\n");
    request.extend_from_slice(b"X-Latin1: caf\xE9\r\n");
    request.extend_from_slice(b"X-Tab: a\tb\r\n");
    request.extend_from_slice(b"X-Colons: a:b::c\r\n");
    request.extend_from_slice(b"x-!#$%&'*+-.^_`|~: every other character a name may have\r\n");
    request.extend_from_slice(b"X-Repeated: 1\r\nX-Repeated: 2\r\n");
    request.extend_from_slice(b"\r\n");
    let (_conn, response) = exchange(&server, request);
    response.expect_status(
        200,
        "a request with unusual but valid header lines (SPEC 5.3)",
    );
}

#[test]
fn a_request_may_arrive_one_byte_at_a_time() {
    let server = Server::start(FILES);
    let mut conn = server.connect();
    for byte in get("/hello.txt").bytes() {
        conn.send([byte]);
        // Long enough for each byte to travel in its own segment.
        thread::sleep(Duration::from_millis(2));
    }
    let response = conn.response();
    response.expect_status(200, "a request sent one byte at a time");
    assert_eq!(response.text(), "hello\n");
}

#[test]
fn a_request_may_be_split_anywhere() {
    let server = Server::start(FILES);
    let request = get("/hello.txt");
    let mut conn = server.connect();
    // One connection, one request per split point: the head is cut in two
    // at every possible place, including inside the final CR LF CR LF.
    for split in 1..request.len() {
        conn.send(&request[..split]);
        thread::sleep(Duration::from_millis(3));
        conn.send(&request[split..]);
        let response = conn.response();
        response.expect_status(200, &format!("the request split after byte {split}"));
        assert_eq!(response.text(), "hello\n");
    }
}

#[test]
fn head_has_the_headers_of_get_and_no_body() {
    let server = Server::start(FILES);
    let mut conn = server.connect();

    conn.send(head("/hello.txt"));
    let for_head = conn.head_response();
    for_head.expect_status(200, "HEAD /hello.txt");
    assert_eq!(
        for_head.content_length(),
        6,
        "HEAD reports the length GET would send"
    );
    assert_eq!(
        for_head.header("content-type"),
        Some("text/plain; charset=utf-8")
    );

    // If the server had sent a body after the HEAD response, those bytes
    // would now be read as the start of this response and fail to parse.
    conn.send(get("/hello.txt"));
    let for_get = conn.response();
    for_get.expect_status(200, "GET right after HEAD on the same connection");
    assert_eq!(for_get.text(), "hello\n");

    conn.send(head("/missing"));
    let missing = conn.head_response();
    missing.expect_status(404, "HEAD of a missing file");
    assert_eq!(
        missing.content_length(),
        14,
        "the length of `404 Not Found` and a line feed"
    );

    conn.send(get("/hello.txt"));
    conn.response()
        .expect_status(200, "GET after a HEAD that was answered with 404");
}

#[test]
fn every_response_has_a_current_date() {
    let server = Server::start(FILES);
    let requests: &[(&str, u16)] = &[
        ("GET /hello.txt HTTP/1.1\r\nHost: test\r\n\r\n", 200),
        ("GET /missing HTTP/1.1\r\nHost: test\r\n\r\n", 404),
        ("BREW /hello.txt HTTP/1.1\r\nHost: test\r\n\r\n", 501),
        ("nonsense\r\n\r\n", 400),
    ];
    for (request, status) in requests {
        let before = unix_now();
        let (_conn, response) = exchange(&server, request);
        let after = unix_now();
        response.expect_status(*status, "a request of the Date test");
        let date = response
            .header("date")
            .unwrap_or_else(|| panic!("the {status} response has no Date header (SPEC 6.3)"));
        let seconds =
            parse_http_date(date).unwrap_or_else(|problem| panic!("Date header: {problem}"));
        assert!(
            (before.saturating_sub(2)..=after + 2).contains(&seconds),
            "the Date header {date:?} is {seconds} s after 1970, but the request was made between {before} \
             and {after}. The date must be the current time in UTC."
        );
    }

    // The clock is read for every response. A date that was formatted once,
    // when the server started, is right for the checks above and wrong now.
    let date_of = |response: &common::Response| {
        let date = response.header("date").expect("checked above");
        parse_http_date(date).unwrap_or_else(|problem| panic!("Date header: {problem}"))
    };
    // The first connection is closed before the pause, so that a server
    // that handles one connection at a time can pass this file.
    let (conn, earlier) = exchange(&server, get("/hello.txt"));
    drop(conn);
    thread::sleep(Duration::from_millis(2100));
    let (_conn, later) = exchange(&server, get("/hello.txt"));
    assert!(
        date_of(&later) >= date_of(&earlier) + 2,
        "two responses that were sent 2.1 s apart have the dates {:?} and {:?}. Every response carries the \
         time at which it is sent (SPEC 6.3).",
        earlier.header("date").unwrap_or_default(),
        later.header("date").unwrap_or_default()
    );
}
