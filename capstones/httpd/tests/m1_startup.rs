//! Milestone M1: the process starts, announces its address and answers a
//! request. SPEC sections 3 (command line), 6 (responses) and the simplest
//! part of 8 (one file, one missing file).
//!
//! A server that handles one connection at a time passes this file.

mod common;

use std::net::TcpListener;
use std::time::Duration;

use common::{Conn, SLACK, Server, Site, get, run_to_exit};

const FILES: &[(&str, &str)] = &[("hello.txt", "hello\n")];

#[test]
fn announces_a_real_port_and_accepts_connections() {
    // `Server::start` passes `--addr 127.0.0.1:0` and fails unless the
    // first line on standard output is `listening on 127.0.0.1:PORT`.
    let server = Server::start(FILES);
    assert_ne!(
        server.addr().port(),
        0,
        "port 0 means: let the system choose; print the chosen port"
    );
    // The line must not be printed before the socket really listens.
    let _conn = server.connect();
}

#[test]
fn serves_a_small_file() {
    let server = Server::start(FILES);
    let response = server.get("/hello.txt");
    response.expect_status(200, "GET /hello.txt");
    assert_eq!(
        response.text(),
        "hello\n",
        "the body must be the bytes of the file"
    );
    assert_eq!(
        response.content_length(),
        6,
        "Content-Length is the number of body bytes"
    );
    assert_eq!(
        response.header("content-type"),
        Some("text/plain; charset=utf-8"),
        "Content-Type for a .txt file (SPEC 8.4)"
    );
    response.expect_keep_alive(true, "an HTTP/1.1 request without `Connection: close`");
}

#[test]
fn a_missing_file_is_404_with_the_standard_body() {
    let server = Server::start(FILES);
    server
        .get("/nothing-here.txt")
        .expect_error(404, "GET of a file that does not exist");
}

#[test]
fn serves_one_connection_after_another() {
    let server = Server::start(FILES);
    for round in 1..=5 {
        let mut conn = server.connect();
        conn.send("GET /hello.txt HTTP/1.1\r\nHost: test\r\nConnection: close\r\n\r\n");
        let response = conn.response();
        response.expect_status(200, &format!("connection number {round}"));
        response.expect_keep_alive(false, "a request with `Connection: close`");
        conn.expect_closed(SLACK, "after a response with `Connection: close`");
    }
}

#[test]
fn several_requests_on_one_connection() {
    let server = Server::start(FILES);
    let mut conn = server.connect();
    for round in 1..=3 {
        conn.send(get("/hello.txt"));
        conn.response()
            .expect_status(200, &format!("request number {round} on one connection"));
    }
}

#[test]
fn help_prints_usage_and_exits_with_0() {
    // `--help` wins wherever it appears, also next to arguments that would
    // be errors without it, and nothing else happens (SPEC 3.1).
    let cases: &[&[&str]] = &[
        &["--help"],
        &["--workers", "0", "--help"],
        &["--help", "--frobnicate"],
    ];
    for args in cases {
        let finished = run_to_exit(args, SLACK);
        let shown = args.join(" ");
        assert_eq!(
            finished.status.code(),
            Some(0),
            "`httpd {shown}` exits with status 0 (SPEC 3.1 and 3.4); standard error was {:?}",
            finished.stderr
        );
        assert!(
            finished.stdout.contains("--root"),
            "`httpd {shown}`: the help text goes to standard output and names the options; got {:?}",
            finished.stdout
        );
        assert!(
            !finished.stdout.contains("listening on"),
            "`httpd {shown}` only prints the help text; it must not start a server"
        );
    }
}

#[test]
fn usage_errors_exit_with_2() {
    // A guard against passing by accident: a correct command line works.
    let site = Site::new(FILES);
    let root = site.root();
    let root = root
        .to_str()
        .expect("the temporary directory has a UTF-8 name");
    drop(Server::start_site(Site::new(FILES), &[]));

    let cases: &[&[&str]] = &[
        &[],
        &["--addr", "127.0.0.1:0"],
        &["--root"],
        &["--root", root, "--addr"],
        &["--root", root, "--frobnicate", "1"],
        &["--root", root, "extra"],
        &["--root", root, "--root", root],
        &["--root", root, "--workers", "2", "--workers", "2"],
        &["--root", root, "--addr", "localhost"],
        &["--root", root, "--addr", "127.0.0.1"],
        &["--root", root, "--workers", "0"],
        &["--root", root, "--workers", "257"],
        &["--root", root, "--workers", "many"],
        &["--root", root, "--workers", "+2"],
        &["--root", root, "--queue", "-1"],
        &["--root", root, "--queue", "4097"],
        &["--root", root, "--max-body", "16777217"],
        &["--root", root, "--max-body", "99999999999999999999999999"],
        &["--root", root, "--request-timeout-ms", "0"],
        &["--root", root, "--request-timeout-ms", "600001"],
        &["--root", root, "--idle-timeout-ms", "0"],
        &["--root", root, "--idle-timeout-ms", "1.5"],
    ];
    for args in cases {
        let finished = run_to_exit(args, SLACK);
        let shown = args.join(" ").replace(root, "<root>");
        assert_eq!(
            finished.status.code(),
            Some(2),
            "`httpd {shown}` is a usage error and must exit with status 2 (SPEC 3.4); standard error was {:?}",
            finished.stderr
        );
        assert!(
            !finished.stderr.trim().is_empty(),
            "`httpd {shown}`: a usage error must be explained on standard error"
        );
        assert!(
            finished.stdout.is_empty(),
            "`httpd {shown}`: nothing may be printed on standard output, got {:?}",
            finished.stdout
        );
    }
}

#[test]
fn a_root_that_is_not_a_directory_exits_with_1() {
    // A guard against passing by accident: a correct command line works.
    drop(Server::start(FILES));

    let site = Site::new(FILES);
    let missing = site.base().join("no-such-directory");
    let a_file = site.root().join("hello.txt");
    for root in [missing, a_file] {
        let root = root
            .to_str()
            .expect("the temporary directory has a UTF-8 name");
        let finished = run_to_exit(&["--root", root, "--addr", "127.0.0.1:0"], SLACK);
        assert_eq!(
            finished.status.code(),
            Some(1),
            "a root that is not an existing directory is a start-up failure: exit status 1 (SPEC 3.4); \
             standard error was {:?}",
            finished.stderr
        );
        assert!(
            !finished.stderr.trim().is_empty(),
            "a start-up failure must be explained on standard error"
        );
        assert!(
            finished.stdout.is_empty(),
            "a server that cannot start must not print `listening on`, got {:?}",
            finished.stdout
        );
    }
}

#[test]
fn an_address_in_use_exits_with_1() {
    drop(Server::start(FILES));

    // This test holds a listening socket, so the server cannot have it.
    let taken = TcpListener::bind("127.0.0.1:0").expect("test setup: bind a port");
    let addr = taken
        .local_addr()
        .expect("test setup: local address")
        .to_string();
    let site = Site::new(FILES);
    let root = site.root();
    let root = root
        .to_str()
        .expect("the temporary directory has a UTF-8 name");
    let finished = run_to_exit(&["--root", root, "--addr", &addr], SLACK);
    assert_eq!(
        finished.status.code(),
        Some(1),
        "an address that is already in use is a start-up failure: exit status 1 (SPEC 3.4); standard \
         error was {:?}",
        finished.stderr
    );
    assert!(
        finished.stdout.is_empty(),
        "a server that cannot listen must not print `listening on`, got {:?}",
        finished.stdout
    );
}

#[test]
fn the_connection_is_usable_right_after_the_announcement() {
    // Twenty servers, one after the other. Each must get its own port and
    // answer the moment its line has appeared.
    for _ in 0..20 {
        let server = Server::start(FILES);
        let mut conn = Conn::open(server.addr()).patience(Duration::from_secs(2));
        conn.send(get("/hello.txt"));
        conn.response()
            .expect_status(200, "first request after start");
    }
}
