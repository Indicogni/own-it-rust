//! Milestone 6: graceful shutdown. After `SHUTDOWN` the server stops
//! accepting, finishes what it is doing, closes every connection, makes the
//! log durable and exits with code 0 within five seconds, whatever its
//! clients are doing.
//!
//! ```text
//! cargo test --test m6_shutdown
//! ```

// Miri cannot start other programs or open sockets.
#![cfg(not(miri))]

mod common;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use common::{Client, Model, Server, Stats, TestDir, parse_log, patience, show};

#[test]
fn m6_idle_and_half_typed_clients_do_not_delay_the_exit() {
    let dir = TestDir::new("m6_idle");
    let server = Server::start(dir.path());
    let addr = server.addr;
    let mut idle: Vec<Client> = (0..5).map(|_| server.client()).collect();
    for c in &mut idle {
        assert_eq!(c.req("PING"), "PONG");
    }
    let mut silent = server.client(); // connected, never sent a byte
    let mut half = server.client();
    half.send_raw(b"SET half-typed valu"); // no line feed

    let mut boss = server.client();
    boss.set("kept", "yes");
    assert_eq!(boss.req("SHUTDOWN"), "OK");
    let exited = server.expect_clean_exit();

    boss.expect_closed("after the OK for SHUTDOWN");
    for c in &mut idle {
        c.expect_closed("at shutdown, for an idle client (SPEC 12)");
    }
    silent.expect_closed("at shutdown, for a client that never sent anything (SPEC 12)");
    half.expect_closed("at shutdown, for a client in the middle of a line (SPEC 12)");
    assert_eq!(
        exited.stdout,
        format!("LISTENING {addr}\n"),
        "standard output must hold the LISTENING line and nothing else, from start to exit \
         (SPEC 3.3)"
    );

    let server = Server::start(dir.path());
    let mut c = server.client();
    assert_eq!(
        c.keys(None),
        ["kept"],
        "the unfinished line `SET half-typed valu` must not have been executed (SPEC 8.1)"
    );
}

#[test]
fn m6_what_was_acknowledged_is_there_after_a_restart() {
    const N: usize = 1500;
    let dir = TestDir::new("m6_durable");
    let server = Server::start(dir.path());
    let mut c = server.client();
    let mut requests = Vec::new();
    let mut expected = Model::new();
    for i in 0..N {
        requests.extend_from_slice(format!("SET k{i} v{i}\n").as_bytes());
        expected.insert(format!("k{i}").into_bytes(), format!("v{i}").into_bytes());
    }
    for reply in c.pipeline(&requests, N) {
        assert_eq!(show(&reply), "OK");
    }
    let before: Stats = c.stats();
    server.shutdown();

    let log = dir.read_log();
    assert_eq!(
        log.len() as u64,
        before.log_bytes,
        "a shutdown must not add to or cut the log"
    );
    let parsed = parse_log(&log);
    parsed.expect_clean("after a graceful shutdown");
    assert!(
        parsed.replay() == expected,
        "the log after shutdown holds every acknowledged write"
    );

    // The lock is free at once, and the content is back.
    let server = Server::start(dir.path());
    let mut c = server.client();
    assert_eq!(
        c.stats(),
        before,
        "STATS after shutdown and restart (compactions was 0)"
    );
    assert!(c.dump() == expected);
}

#[test]
fn m6_shutdown_under_load_keeps_every_acknowledged_write() {
    const WRITERS: usize = 6;
    const ENOUGH: usize = 600;
    let dir = TestDir::new("m6_load");
    let server = Server::start(dir.path());
    let acknowledged = AtomicUsize::new(0);

    let (exited, per_writer) = std::thread::scope(|scope| {
        let writers: Vec<_> = (0..WRITERS)
            .map(|t| {
                let mut c = server.client();
                let acknowledged = &acknowledged;
                scope.spawn(move || {
                    // Write until the server goes away; remember how many
                    // writes were acknowledged.
                    let mut done = 0;
                    loop {
                        let line = format!("SET w{t}-{done} value-{t}-{done}\n");
                        if c.try_send_raw(line.as_bytes()).is_err() {
                            break;
                        }
                        match c.try_recv() {
                            Ok(Some(reply)) if reply == b"OK" => {
                                done += 1;
                                acknowledged.fetch_add(1, Ordering::SeqCst);
                            }
                            Ok(Some(reply)) => {
                                panic!("writer {t}: SET was answered with `{}`", show(&reply))
                            }
                            // Closed or reset: the server is shutting down.
                            Ok(None) => break,
                            Err(e) if common::is_timeout(&e) => {
                                panic!("writer {t}: no answer and no close within the time limit")
                            }
                            Err(_) => break,
                        }
                    }
                    done
                })
            })
            .collect();

        let deadline = Instant::now() + patience();
        while acknowledged.load(Ordering::SeqCst) < ENOUGH {
            assert!(
                Instant::now() < deadline,
                "the writers were too slow to reach {ENOUGH} writes"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
        let mut boss = server.client();
        assert_eq!(boss.req("SHUTDOWN"), "OK");
        let exited = server.expect_clean_exit();
        let per_writer: Vec<usize> = writers
            .into_iter()
            .map(|w| w.join().expect("a writer thread panicked"))
            .collect();
        (exited, per_writer)
    });
    drop(exited);

    let parsed = parse_log(&dir.read_log());
    parsed.expect_clean("after a shutdown under load");
    let content = parsed.replay();
    for (t, &done) in per_writer.iter().enumerate() {
        for i in 0..done {
            assert_eq!(
                content
                    .get(format!("w{t}-{i}").as_bytes())
                    .map(Vec::as_slice),
                Some(format!("value-{t}-{i}").as_bytes()),
                "writer {t} got OK for its write number {i} before the server shut down, so the \
                 write must be in the log (SPEC 12)"
            );
        }
    }
    // Each writer can have one more write in the log than it saw answered.
    let acknowledged: usize = per_writer.iter().sum();
    assert!(
        (acknowledged..=acknowledged + WRITERS).contains(&content.len()),
        "{} keys in the log, {acknowledged} acknowledged writes, {WRITERS} writers",
        content.len()
    );
    let server = Server::start(dir.path());
    assert!(
        server.client().dump() == content,
        "restart after a shutdown under load"
    );
}

#[test]
fn m6_shutdown_works_with_fsync_always() {
    let dir = TestDir::new("m6_fsync");
    let server = Server::start_with(dir.path(), &["--auto-compact", "0"]);
    let mut c = server.client();
    for i in 0..20 {
        c.set(format!("k{i}"), "v");
    }
    assert_eq!(c.req("SHUTDOWN"), "OK");
    server.expect_clean_exit();
    let parsed = parse_log(&dir.read_log());
    parsed.expect_clean("after a shutdown in the default fsync mode");
    assert_eq!(parsed.records.len(), 20);
}

#[test]
fn m6_start_write_shut_down_ten_times() {
    let dir = TestDir::new("m6_cycles");
    let mut expected = Model::new();
    for round in 0..10 {
        let server = Server::start(dir.path());
        let mut c = server.client();
        assert!(
            c.dump() == expected,
            "content at the start of round {round}"
        );
        for i in 0..10 {
            let (key, value) = (format!("k{}", (round * 3 + i) % 17), format!("r{round}"));
            c.set(&key, &value);
            expected.insert(key.into_bytes(), value.into_bytes());
        }
        if round % 3 == 2 {
            assert_eq!(c.req("COMPACT"), "OK");
        }
        drop(c);
        server.shutdown();
    }
    assert!(parse_log(&dir.read_log()).replay() == expected);
}

#[test]
fn m6_a_client_that_does_not_read_cannot_block_the_shutdown() {
    let dir = TestDir::new("m6_not_reading");
    let server = Server::start(dir.path());
    let mut filler = server.client();
    filler.set("big", "v".repeat(4000));

    // This client asks for megabytes of answers and reads none of them.
    // The server's writes to it will block when the buffers are full.
    let mut deaf = server.client();
    let requests = "GET big\n".repeat(4000);
    let sending = std::thread::spawn(move || {
        // The write may fail or time out once the server is gone; either
        // is fine. Keep the connection open until the test is over.
        let _ = deaf.try_send_raw(requests.as_bytes());
        deaf
    });
    // Give the server a moment to run into the full buffer.
    std::thread::sleep(Duration::from_millis(300));

    assert_eq!(
        filler.req("PING"),
        "PONG",
        "a client that does not read must not block others (SPEC 11.2)"
    );
    assert_eq!(filler.req("SHUTDOWN"), "OK");
    server.expect_clean_exit();
    drop(sending.join());
}
