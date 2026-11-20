//! Milestone 5: many clients at the same time. Every command is atomic,
//! the log has the order in which the clients saw things happen, a slow
//! client holds nobody up, and there is a limit on connections.
//!
//! Tests of concurrent code cannot prove that a server is right. These
//! tests always pass for a correct server; a server with a race fails them
//! most of the time, not every time. If one fails once in ten runs, the
//! bug is real.
//!
//! ```text
//! cargo test --test m5_concurrency
//! ```

// Miri cannot start other programs or open sockets.
#![cfg(not(miri))]

mod common;

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use common::{Client, CountWhenDone, Model, Record, Server, TestDir, parse_log, patience, show};

#[test]
fn m5_a_client_that_stops_in_the_middle_of_a_line_blocks_nobody() {
    let dir = TestDir::new("m5_slow_client");
    let server = Server::start(dir.path());
    // Three clients that never finish their line, one that sends nothing.
    let mut stuck: Vec<Client> = (0..3).map(|_| server.client()).collect();
    for c in &mut stuck {
        c.send_raw(b"SET half-typed val");
    }
    let _silent = server.client();

    let mut busy = server.client();
    assert_eq!(
        busy.req("PING"),
        "PONG",
        "a client must be served while other connections are open and idle (SPEC 11.2). A \
         server that handles one connection after the other fails here."
    );
    busy.set("k", "v");

    // The stuck clients finish their lines now and are served too.
    for (i, c) in stuck.iter_mut().enumerate() {
        c.send_raw(format!("ue{i}\n").as_bytes());
        assert_eq!(
            show(&c.recv()),
            "OK",
            "a line may arrive in several pieces (SPEC 8.1)"
        );
    }
    assert_eq!(
        busy.get("half-typed").map(|v| v.len()),
        Some(6),
        "one of `value0`..`value2` was stored"
    );
}

#[test]
fn m5_sixty_four_connections_at_once() {
    const CLIENTS: usize = 64;
    let dir = TestDir::new("m5_64");
    let server = Server::start(dir.path());
    let mut clients: Vec<Client> = (0..CLIENTS).map(|_| server.client()).collect();
    // All 64 are open now. Use them in turn, twice.
    for round in 0..2 {
        for (i, c) in clients.iter_mut().enumerate() {
            c.set(format!("client{i}"), format!("round{round}"));
        }
    }
    for (i, c) in clients.iter_mut().enumerate().rev() {
        assert_eq!(
            c.get(format!("client{i}")).as_deref(),
            Some(&b"round1"[..]),
            "connection {i} of {CLIENTS} open at once (SPEC 11.2)"
        );
    }
    assert_eq!(clients[0].stats().keys, CLIENTS as u64);
}

#[test]
fn m5_writers_on_different_keys_all_land() {
    const CLIENTS: usize = 16;
    const WRITES: usize = 150;
    let dir = TestDir::new("m5_disjoint");
    let server = Server::start(dir.path());
    let mut expected = Model::new();
    std::thread::scope(|scope| {
        for t in 0..CLIENTS {
            let mut c = server.client();
            scope.spawn(move || {
                for i in 0..WRITES {
                    c.set(format!("t{t:02}-{i:03}"), format!("{t}:{i}"));
                    if i % 10 == 9 {
                        // Read your own write on the same connection.
                        assert_eq!(
                            c.get(format!("t{t:02}-{i:03}")).as_deref(),
                            Some(format!("{t}:{i}").as_bytes()),
                            "a client must see its own acknowledged write (SPEC 11.1)"
                        );
                    }
                }
            });
        }
    });
    for t in 0..CLIENTS {
        for i in 0..WRITES {
            expected.insert(
                format!("t{t:02}-{i:03}").into_bytes(),
                format!("{t}:{i}").into_bytes(),
            );
        }
    }
    let mut c = server.client();
    assert_eq!(
        c.stats().keys,
        (CLIENTS * WRITES) as u64,
        "16 clients wrote 150 keys each"
    );
    assert!(
        c.dump() == expected,
        "every acknowledged write of every client must be in the store"
    );
    drop(c);
    server.kill();

    let parsed = parse_log(&dir.read_log());
    parsed.expect_clean("after 16 clients wrote at the same time: records must not be interleaved");
    assert_eq!(
        parsed.records.len(),
        CLIENTS * WRITES,
        "one record per acknowledged write"
    );
    assert!(
        parsed.replay() == expected,
        "the log replays to what the clients wrote"
    );
}

#[test]
fn m5_incr_loses_no_update() {
    const CLIENTS: usize = 8;
    const INCRS: usize = 250;
    let dir = TestDir::new("m5_incr");
    let server = Server::start(dir.path());
    let seen: Vec<Vec<i64>> = std::thread::scope(|scope| {
        let threads: Vec<_> = (0..CLIENTS)
            .map(|_| {
                let mut c = server.client();
                scope.spawn(move || {
                    (0..INCRS)
                        .map(|_| {
                            let reply = c.req("INCR counter");
                            reply
                                .strip_prefix("VALUE ")
                                .and_then(|n| n.parse::<i64>().ok())
                                .unwrap_or_else(|| {
                                    panic!("INCR must be answered with `VALUE <n>`, not `{reply}`")
                                })
                        })
                        .collect::<Vec<i64>>()
                })
            })
            .collect();
        threads
            .into_iter()
            .map(|t| t.join().expect("an INCR thread panicked"))
            .collect()
    });

    let total = (CLIENTS * INCRS) as i64;
    let mut c = server.client();
    assert_eq!(
        c.get("counter").as_deref(),
        Some(total.to_string().as_bytes()),
        "{CLIENTS} clients sent {INCRS} INCRs each. INCR reads and writes in one atomic step, \
         so none may be lost (SPEC 11.1). A smaller number means two clients read the same \
         old value."
    );
    for (t, values) in seen.iter().enumerate() {
        assert!(
            values.windows(2).all(|w| w[0] < w[1]),
            "client {t} must see the counter grow with each of its own INCRs"
        );
    }
    let distinct: BTreeSet<i64> = seen.iter().flatten().copied().collect();
    assert_eq!(
        distinct,
        (1..=total).collect::<BTreeSet<i64>>(),
        "each of the numbers 1..={total} must have been returned to exactly one INCR"
    );
    drop(c);
    server.kill();
    let server = Server::start(dir.path());
    assert_eq!(
        server.client().get("counter").as_deref(),
        Some(total.to_string().as_bytes()),
        "the counter after a kill and a restart"
    );
}

#[test]
fn m5_the_log_has_the_order_the_clients_saw() {
    const CLIENTS: usize = 8;
    const KEYS: usize = 300;
    let dir = TestDir::new("m5_order");
    let server = Server::start(dir.path());
    std::thread::scope(|scope| {
        for t in 0..CLIENTS {
            let mut c = server.client();
            scope.spawn(move || {
                // Every client writes every key once, in the same order, so
                // each key is written by eight clients at about the same
                // time. That makes 300 separate races.
                for i in 0..KEYS {
                    c.set(format!("shared{i:03}"), format!("client{t}"));
                }
            });
        }
    });
    let mut c = server.client();
    let served = c.dump();
    assert_eq!(served.len(), KEYS);
    drop(c);
    server.kill();

    let parsed = parse_log(&dir.read_log());
    parsed.expect_clean("after 8 clients wrote the same keys");
    assert_eq!(parsed.records.len(), CLIENTS * KEYS);
    let in_log = parsed.replay();
    let differing: Model = served
        .iter()
        .filter(|(key, value)| in_log.get(*key) != Some(*value))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    assert!(
        differing.is_empty(),
        "for {} of {KEYS} keys, the last record in the log is not the value the server was \
         serving. Then the record was appended and the memory was updated in two separate \
         steps, and two writers went through them in different orders (SPEC 11.1).\n\
         served, for example: {}",
        differing.len(),
        describe(&differing.into_iter().take(3).collect())
    );
    let server = Server::start(dir.path());
    assert!(
        server.client().dump() == served,
        "after a restart the store must hold what it served before the kill"
    );
}

fn describe(content: &Model) -> String {
    content
        .iter()
        .map(|(k, v)| format!("{}={}", show(k), show(v)))
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn m5_readers_never_see_half_a_write() {
    const WRITERS: usize = 4;
    const READERS: usize = 4;
    const WRITES: usize = 250;
    let dir = TestDir::new("m5_torn_reads");
    let server = Server::start(dir.path());
    server.client().set("page", "a".repeat(100));

    // How many writers have finished; the readers go on until all have.
    let finished = AtomicUsize::new(0);
    let finished = &finished;
    std::thread::scope(|scope| {
        for t in 0..WRITERS {
            let mut c = server.client();
            scope.spawn(move || {
                let _finished = CountWhenDone(finished);
                for i in 0..WRITES {
                    // One letter, repeated; the length changes too.
                    let letter = char::from(b'a' + ((t * 7 + i) % 26) as u8);
                    let len = 1 + (i * 131 + t * 17) % 4000;
                    c.set("page", letter.to_string().repeat(len));
                }
            });
        }
        // Compactions move every record while the readers read.
        let mut compactor = server.client();
        scope.spawn(move || {
            for _ in 0..15 {
                assert_eq!(compactor.req("COMPACT"), "OK");
            }
        });
        for r in 0..READERS {
            let mut c = server.client();
            scope.spawn(move || {
                let mut reads = 0;
                while reads < 200 || finished.load(Ordering::SeqCst) < WRITERS {
                    let value = c
                        .get("page")
                        .unwrap_or_else(|| panic!("reader {r}: `page` is never deleted"));
                    reads += 1;
                    let uniform = !value.is_empty()
                        && value[0].is_ascii_lowercase()
                        && value.iter().all(|&b| b == value[0]);
                    assert!(
                        uniform,
                        "reader {r} got a value that no writer ever wrote ({} bytes: `{}`). Every \
                         value was one lower-case letter repeated. A GET must return a complete \
                         value from one SET, also while other clients write and compact \
                         (SPEC 11.1).",
                        value.len(),
                        show(&value)
                    );
                }
            });
        }
    });
}

#[test]
fn m5_keys_is_a_snapshot() {
    // Each writer owns one "token" and moves it from key to key, downwards:
    // it sets the next key and then deletes the old one. So at every
    // instant its token is on one key, or on two neighbours. A KEYS that
    // looks at the store at one instant shows exactly that. A KEYS that
    // collects its answer piece by piece, while writers go on, can miss the
    // token completely, or announce a count that does not fit its list.
    const WRITERS: usize = 4;
    const MOVES: usize = 400;
    let dir = TestDir::new("m5_snapshot");
    let server = Server::start(dir.path());
    let mut filler = server.client();
    for i in 0..200 {
        filler.set(format!("other{i:03}"), "x");
    }
    for w in 0..WRITERS {
        filler.set(format!("token{w}-{MOVES:04}"), "x");
    }
    drop(filler);

    let finished = AtomicUsize::new(0);
    let finished = &finished;
    std::thread::scope(|scope| {
        for w in 0..WRITERS {
            let mut c = server.client();
            scope.spawn(move || {
                let _finished = CountWhenDone(finished);
                for at in (1..=MOVES).rev() {
                    c.set(format!("token{w}-{:04}", at - 1), "x");
                    assert!(c.del(format!("token{w}-{at:04}")));
                }
            });
        }
        for r in 0..WRITERS {
            let mut c = server.client();
            scope.spawn(move || {
                let prefix = format!("token{r}-");
                let mut lists = 0;
                while lists < 50 || finished.load(Ordering::SeqCst) < WRITERS {
                    let keys = c.keys(Some(&prefix));
                    lists += 1;
                    let places: Vec<usize> = keys
                        .iter()
                        .map(|key| {
                            key.strip_prefix(&prefix)
                                .and_then(|n| n.parse().ok())
                                .unwrap_or_else(|| {
                                    panic!(
                                        "`KEYS {prefix}` listed `{key}`, which is not a key \
                                         with that prefix. Did an earlier answer announce a \
                                         count that did not fit the lines after it (SPEC 8.8)?"
                                    )
                                })
                        })
                        .collect();
                    let one_instant = match places[..] {
                        [_] => true,
                        [low, high] => low + 1 == high,
                        _ => false,
                    };
                    assert!(
                        one_instant,
                        "`KEYS {prefix}` answered {keys:?}. At every instant the store holds \
                         one key with this prefix, or two neighbours while the writer moves \
                         from one to the next. KEYS must list the store as it was at one \
                         instant (SPEC 8.8); this answer mixes several."
                    );
                }
            });
        }
    });
    let mut c = server.client();
    assert_eq!(
        c.keys(Some("token")),
        ["token0-0000", "token1-0000", "token2-0000", "token3-0000"],
        "where the four tokens end"
    );
    assert_eq!(c.stats().keys, 204);
}

#[test]
fn m5_a_kill_while_clients_write_and_compact_loses_no_acknowledged_write() {
    // Milestone 4 killed the server in the middle of a compaction with
    // nobody else around. Here one client writes without pause while a
    // second one compacts without pause. A write that arrives during a
    // compaction must end up in the new log, not in the old one that is
    // about to be thrown away.
    const ENOUGH: usize = 25;
    const WRITES: usize = 100_000;
    let dir = TestDir::new("m5_kill_compacting");
    let server = Server::start(dir.path());
    let acknowledged = AtomicUsize::new(0);
    let compactions = AtomicUsize::new(0);

    let (written, reached) = std::thread::scope(|scope| {
        // One client writes k0, k1, ... and counts the acknowledgements.
        let mut writer = server.client();
        let acknowledged = &acknowledged;
        let writing = scope.spawn(move || {
            let mut done = 0;
            for i in 0..WRITES {
                let line = format!("SET k{i} v{i}\n");
                if writer.try_send_raw(line.as_bytes()).is_err() {
                    break;
                }
                match writer.try_recv() {
                    Ok(Some(reply)) if reply == b"OK" => {
                        done = i + 1;
                        acknowledged.store(done, Ordering::SeqCst);
                    }
                    _ => break,
                }
            }
            done
        });
        let mut compactor = server.client();
        let compactions = &compactions;
        let compacting = scope.spawn(move || {
            loop {
                if compactor.try_send_raw(b"COMPACT\n").is_err() {
                    break;
                }
                match compactor.try_recv() {
                    Ok(Some(reply)) if reply == b"OK" => compactions.fetch_add(1, Ordering::SeqCst),
                    _ => break,
                };
            }
        });

        // Both threads stop when the server is killed, so kill it in every
        // case and judge afterwards.
        let deadline = Instant::now() + patience();
        let reached = loop {
            if acknowledged.load(Ordering::SeqCst) >= ENOUGH
                && compactions.load(Ordering::SeqCst) >= ENOUGH
            {
                break true;
            }
            if Instant::now() >= deadline {
                break false;
            }
            std::thread::sleep(Duration::from_millis(1));
        };
        server.kill();
        compacting.join().expect("the compactor thread panicked");
        (writing.join().expect("the writer thread panicked"), reached)
    });
    assert!(
        reached,
        "after {} s only {written} writes and {} compactions were acknowledged; the test needs \
         {ENOUGH} of each. Does a COMPACT loop on one connection starve the writers?",
        patience().as_secs(),
        compactions.load(Ordering::SeqCst),
    );

    let server = Server::start(dir.path());
    let mut c = server.client();
    let content = c.dump();
    for i in 0..written {
        assert_eq!(
            content.get(format!("k{i}").as_bytes()).map(Vec::as_slice),
            Some(format!("v{i}").as_bytes()),
            "`SET k{i} v{i}` was acknowledged while another client compacted, and the server \
             was killed afterwards; {written} writes were acknowledged in all, and none may \
             be lost (SPEC 10.2). Did a compaction copy the store, let the write go to the \
             old log, and then replace that log?"
        );
    }
    // At most the one write that was in flight may be there in addition.
    assert!(
        content.len() <= written + 1,
        "{} keys after recovery, but only {written} writes were acknowledged and one more was \
         in flight",
        content.len()
    );
}

#[test]
fn m5_the_connection_limit() {
    let dir = TestDir::new("m5_limit");
    let server = Server::start_with(dir.path(), &["--fsync", "never", "--max-clients", "3"]);
    let mut admitted: Vec<Client> = (0..3).map(|_| server.client()).collect();
    for c in &mut admitted {
        assert_eq!(
            c.req("PING"),
            "PONG",
            "up to --max-clients connections are served (SPEC 11.3)"
        );
    }

    // The fourth connection is told why and closed, without being asked.
    for _ in 0..2 {
        let mut refused = server.client();
        assert_eq!(
            show(&refused.recv()),
            "ERR too many connections",
            "a connection over the limit gets this line at once, before it sends anything \
             (SPEC 11.3)"
        );
        refused.expect_closed("after `ERR too many connections`");
    }
    for c in &mut admitted {
        assert_eq!(
            c.req("PING"),
            "PONG",
            "refusing a connection must not disturb the admitted ones"
        );
    }

    // One client leaves; its place becomes free "soon" (SPEC 11.3).
    admitted.pop();
    let deadline = Instant::now() + patience();
    loop {
        let mut c = server.client();
        // A refused connection may already be closed; then this fails,
        // which the read below reports.
        let _ = c.try_send_raw(b"PING\n");
        match c.try_recv() {
            Ok(Some(reply)) if reply == b"PONG" => break,
            Ok(Some(reply)) if reply == b"ERR too many connections" => {}
            // The server closed a refused connection while the `PING` was
            // on its way. Some systems then reset the connection and throw
            // the refusal line away. This is still a refusal.
            Ok(None) => {}
            Err(e) if !common::is_timeout(&e) => {}
            other => panic!("a new connection got neither PONG nor the limit error: {other:?}"),
        }
        assert!(
            Instant::now() < deadline,
            "{} s after a client closed its connection, its place was still taken (SPEC 11.3)",
            patience().as_secs()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn m5_connections_that_come_and_go() {
    // 300 short connections against a limit of 8: places must be given
    // back, and nothing may pile up per connection.
    let dir = TestDir::new("m5_churn");
    let server = Server::start_with(dir.path(), &["--fsync", "never", "--max-clients", "8"]);
    std::thread::scope(|scope| {
        for t in 0..4 {
            let server = &server;
            scope.spawn(move || {
                for i in 0..75 {
                    let mut c = server.client();
                    c.set(format!("c{t}-{i}"), "x");
                    if i % 2 == 0 {
                        assert_eq!(c.req("QUIT"), "BYE");
                    }
                    // Otherwise the connection is simply dropped.
                }
            });
        }
    });
    let mut c = server.client();
    assert_eq!(
        c.stats().keys,
        300,
        "4 threads made 75 connections each, one SET per connection"
    );
    assert_eq!(
        parse_log(&dir.read_log())
            .records
            .iter()
            .filter(|r| matches!(r, Record::Set(..)))
            .count(),
        300
    );
}
