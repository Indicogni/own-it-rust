//! Milestone 4: compaction. `COMPACT` rewrites the log to one record per
//! live key, a crash at any moment of that loses nothing, and the server
//! compacts by itself when the log holds too much garbage.
//!
//! Like the tests of milestones 1 to 3, these use one connection at a time.
//! Compaction with other clients at work is part of milestone 5.
//!
//! ```text
//! cargo test --test m4_compaction
//! ```

// Miri cannot start other programs or open sockets.
#![cfg(not(miri))]

mod common;

use common::{
    COMPACT_FILE, Client, Model, Server, Stats, TestDir, canonical_log, live_bytes, log_of, model,
    parse_log, record_len, set_record, show, wait_for_stats,
};

/// Writes a history with overwrites and deletions and returns what is live.
fn churn(c: &mut Client, keys: usize, rounds: usize) -> Model {
    let mut content = Model::new();
    for round in 0..rounds {
        for i in 0..keys {
            // Insert in an order that is not byte order.
            let key = format!("key-{:03}", (i * 37) % keys);
            let value = format!("round {round}, {}", "y".repeat((i + round) % 23));
            c.set(&key, &value);
            content.insert(key.into_bytes(), value.into_bytes());
        }
    }
    for i in (0..keys).step_by(5) {
        let key = format!("key-{i:03}");
        assert!(c.del(&key));
        content.remove(key.as_bytes());
    }
    content
}

#[track_caller]
fn compact(c: &mut Client) {
    assert_eq!(
        c.req("COMPACT"),
        "OK",
        "COMPACT is answered with OK when the new log is in place (SPEC 8.10)"
    );
}

#[test]
fn m4_compact_writes_one_record_per_live_key_in_key_order() {
    let dir = TestDir::new("m4_canonical");
    let server = Server::start(dir.path());
    let mut c = server.client();
    let content = churn(&mut c, 60, 4);
    let before = c.stats();
    assert!(
        before.garbage() > 0,
        "the history has overwrites and deletions, so the log has garbage"
    );

    compact(&mut c);
    let expected = canonical_log(&content);
    let log = dir.read_log();
    assert!(
        log == expected,
        "after COMPACT, kv.log must be the 8 magic bytes followed by one SET record per live \
         key, in ascending byte order of the keys, and nothing else (SPEC 10.1).\n\
         expected {} bytes, found {}; the file starts with `{}`",
        expected.len(),
        log.len(),
        show(&log)
    );
    assert_eq!(
        c.stats(),
        Stats {
            keys: content.len() as u64,
            log_bytes: expected.len() as u64,
            live_bytes: expected.len() as u64,
            compactions: 1,
        },
        "after a compaction log_bytes equals live_bytes, and compactions has counted it (SPEC 8.9)"
    );
    assert_eq!(
        c.dump(),
        content,
        "a compaction must not change what the store holds"
    );
    assert!(
        !dir.path().join(COMPACT_FILE).exists(),
        "kv.compact must be gone when COMPACT is answered (SPEC 10.2)"
    );
}

#[test]
fn m4_the_example_session_of_spec_8_14() {
    let dir = TestDir::new("m4_example");
    let server = Server::start(dir.path());
    let mut c = server.client();
    // Each request with all the lines of its response.
    let session: &[(&str, &[&str])] = &[
        ("PING", &["PONG"]),
        ("SET fruit apple", &["OK"]),
        ("SET motto to be, or not", &["OK"]),
        ("GET motto", &["VALUE to be, or not"]),
        ("GET nothing", &["NOT_FOUND"]),
        ("INCR visits", &["VALUE 1"]),
        ("INCR visits", &["VALUE 2"]),
        ("INCR fruit", &["ERR not an integer"]),
        ("KEYS", &["KEYS 3", "fruit", "motto", "visits"]),
        ("KEYS m", &["KEYS 1", "motto"]),
        ("DEL fruit", &["OK"]),
        ("DEL fruit", &["NOT_FOUND"]),
        ("get motto", &["ERR unknown command"]),
        (
            "STATS",
            &["STATS keys=2 log_bytes=110 live_bytes=55 compactions=0"],
        ),
        ("COMPACT", &["OK"]),
        (
            "STATS",
            &["STATS keys=2 log_bytes=55 live_bytes=55 compactions=1"],
        ),
        ("QUIT", &["BYE"]),
    ];
    for (request, response) in session {
        c.send(request);
        for expected in *response {
            let line = c.recv();
            // `STATS` may carry extra fields after the four required ones.
            let line = show(&line);
            assert!(
                line == *expected
                    || (request == &"STATS" && line.starts_with(&format!("{expected} "))),
                "the session of SPEC 8.14: after `{request}` the server must send `{expected}`, \
                 not `{line}`"
            );
        }
    }
    c.expect_closed("after BYE");
}

#[test]
fn m4_compacting_an_empty_store_leaves_the_header() {
    let dir = TestDir::new("m4_empty");
    let server = Server::start(dir.path());
    let mut c = server.client();
    compact(&mut c);
    assert_eq!(
        show(&dir.read_log()),
        "KVSTORE1",
        "compacting an empty store"
    );
    c.set("a", "1");
    assert!(c.del("a"));
    compact(&mut c);
    assert_eq!(
        show(&dir.read_log()),
        "KVSTORE1",
        "compacting a store whose only key was deleted"
    );
    assert_eq!(
        c.stats(),
        Stats {
            keys: 0,
            log_bytes: 8,
            live_bytes: 8,
            compactions: 2
        },
        "every COMPACT counts, also when there was nothing to remove (SPEC 8.10)"
    );
}

#[test]
fn m4_writes_after_a_compaction_go_to_the_new_log() {
    let dir = TestDir::new("m4_after");
    let server = Server::start(dir.path());
    let mut c = server.client();
    let mut content = churn(&mut c, 40, 3);
    compact(&mut c);
    let mut expected_log = canonical_log(&content);

    c.set("zzz-new", "after");
    content.insert(b"zzz-new".to_vec(), b"after".to_vec());
    c.set("key-001", "changed");
    content.insert(b"key-001".to_vec(), b"changed".to_vec());
    assert_eq!(c.req("INCR counter"), "VALUE 1");
    content.insert(b"counter".to_vec(), b"1".to_vec());

    expected_log.extend_from_slice(&set_record("zzz-new", "after"));
    expected_log.extend_from_slice(&set_record("key-001", "changed"));
    expected_log.extend_from_slice(&set_record("counter", "1"));
    assert!(
        dir.read_log() == expected_log,
        "records written after a compaction are appended to the compacted log (SPEC 10.1)"
    );
    assert_eq!(
        c.dump(),
        content,
        "content after compaction and three more writes"
    );

    server.kill();
    let server = Server::start(dir.path());
    let mut c = server.client();
    assert_eq!(
        c.dump(),
        content,
        "a compacted log with later records recovers like any other log"
    );
    assert_eq!(
        c.stats().compactions,
        0,
        "compactions counts since the start of the process (SPEC 8.9)"
    );
}

#[test]
fn m4_reads_are_right_after_every_compaction() {
    // For a server that keeps positions in the log instead of values: every
    // position changes in a compaction.
    let dir = TestDir::new("m4_reads");
    let server = Server::start(dir.path());
    let mut c = server.client();
    let mut content = Model::new();
    for round in 0..6 {
        for i in 0..150 {
            if (i + round) % 3 == 0 {
                let key = format!("item{i:03}");
                let value = format!("{round}:{}", "z".repeat(i % 50));
                c.set(&key, &value);
                content.insert(key.into_bytes(), value.into_bytes());
            }
        }
        compact(&mut c);
        assert_eq!(
            c.dump(),
            content,
            "content after compaction number {}",
            round + 1
        );
        assert_eq!(c.stats().compactions, round as u64 + 1);
    }
}

#[test]
fn m4_a_leftover_compaction_file_is_removed_at_start_up() {
    let dir = TestDir::new("m4_leftover");
    let log = log_of(&[set_record("real", "data"), set_record("more", "data")]);
    dir.write_log(&log);
    // What a crash in the middle of a compaction leaves: the old log,
    // complete, and an unfinished new one beside it. Here the unfinished
    // file even looks like a good log with other content.
    let leftover = dir.path().join(COMPACT_FILE);
    std::fs::write(&leftover, log_of(&[set_record("wrong", "file")]))
        .expect("cannot write kv.compact");

    let server = Server::start(dir.path());
    assert!(
        !leftover.exists(),
        "a kv.compact found at start-up belongs to a compaction that never finished; it must \
         be deleted before the LISTENING line (SPEC 10.2)"
    );
    assert_eq!(
        dir.read_log(),
        log,
        "kv.log is the truth and must not be changed"
    );
    let mut c = server.client();
    assert_eq!(
        c.dump(),
        model(&[("real", "data"), ("more", "data")]),
        "the content comes from kv.log only"
    );
    compact(&mut c);
    assert!(!leftover.exists());
}

#[test]
fn m4_a_kill_between_compactions_loses_nothing() {
    let dir = TestDir::new("m4_kill_after");
    let server = Server::start(dir.path());
    let mut c = server.client();
    let content = churn(&mut c, 50, 3);
    compact(&mut c);
    server.kill();
    let parsed = parse_log(&dir.read_log());
    parsed.expect_clean("a compacted log after a kill");
    assert_eq!(parsed.replay(), content);
    let server = Server::start(dir.path());
    assert_eq!(
        server.client().dump(),
        content,
        "restart on a compacted log"
    );
}

/// What differs between the store and the model, in a few words. The
/// contents themselves are megabytes.
fn difference(served: &Model, expected: &Model) -> String {
    let wrong: Vec<&Vec<u8>> = expected
        .iter()
        .filter(|(key, value)| served.get(*key) != Some(*value))
        .map(|(key, _)| key)
        .collect();
    let extra = served
        .keys()
        .filter(|key| !expected.contains_key(*key))
        .count();
    format!(
        "{} of {} keys are missing or have another value (the first is `{}`), and {extra} keys \
         are there that should not be",
        wrong.len(),
        expected.len(),
        wrong.first().map_or_else(String::new, |key| show(key)),
    )
}

#[test]
fn m4_a_kill_in_the_middle_of_a_compaction_loses_nothing() {
    // The store holds about 2 MB, so that writing the new log takes a
    // moment. The test sends COMPACT, watches the data directory, and kills
    // the server at one of two instants:
    //
    // - when kv.compact appears: the new log is being written;
    // - when kv.log has become shorter: the old log has just been replaced.
    //
    // Whatever instant the kill hits, the content must be what it was
    // before: a compaction changes no content, and every write was
    // acknowledged before the COMPACT was sent.
    const KEYS: usize = 500;
    const ROUNDS: usize = 6;
    let dir = TestDir::new("m4_kill_during");
    let compact_file = dir.path().join(COMPACT_FILE);
    let value = |round: usize, i: usize| vec![b'a' + ((round + i) % 26) as u8; 4000];

    let mut content = Model::new();
    let mut server = Server::start(dir.path());
    {
        let mut c = server.client();
        let mut requests = Vec::new();
        for i in 0..KEYS {
            let key = format!("key{i:04}").into_bytes();
            requests.extend_from_slice(b"SET ");
            requests.extend_from_slice(&key);
            requests.push(b' ');
            requests.extend_from_slice(&value(0, i));
            requests.push(b'\n');
            content.insert(key, value(0, i));
        }
        for reply in c.pipeline(&requests, KEYS) {
            assert_eq!(show(&reply), "OK");
        }
    }

    for round in 1..=ROUNDS {
        let mut c = server.client();
        // Overwrite some keys, so that the compaction has garbage to remove
        // and the log gets shorter.
        for i in 0..20 {
            let key = format!("key{:04}", (round * 20 + i) % KEYS);
            c.set(&key, value(round, i));
            content.insert(key.into_bytes(), value(round, i));
        }
        let before = dir.log_len();
        let live = live_bytes(&content);
        assert!(
            before > live,
            "round {round}: after 20 overwrites the log holds garbage"
        );

        let wait_for_the_switch = round % 2 == 0;
        c.send("COMPACT");
        let deadline = std::time::Instant::now() + common::patience();
        loop {
            // The file can be missing for an instant while it is replaced
            // on some systems; that counts as "shorter".
            let shorter = std::fs::metadata(dir.log()).map_or(true, |m| m.len() < before);
            if shorter || (!wait_for_the_switch && compact_file.exists()) {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "round {round}: {} s after COMPACT was sent, kv.log still has its old size of \
                 {before} bytes, of which only {live} are live (SPEC 10.1)",
                common::patience().as_secs()
            );
            std::hint::spin_loop();
        }
        server.kill();
        drop(c);
        let when = if wait_for_the_switch {
            "at the moment kv.log was replaced"
        } else {
            "while the new log was being written"
        };

        server = Server::start(dir.path());
        assert!(
            !compact_file.exists(),
            "round {round}: kv.compact must be gone after start-up (SPEC 10.2)"
        );
        let served = server.client().dump();
        assert!(
            served == content,
            "round {round}: the server was killed {when}. A crash during a compaction must \
             leave either the complete old log or the complete new log, so the store must \
             hold exactly what it held before (SPEC 10.2). But {}. Was the new log written \
             into kv.log itself, or put in place before it was complete?",
            difference(&served, &content)
        );
    }
    drop(server);
    parse_log(&dir.read_log()).expect_clean("after kills in the middle of compactions");
}

#[test]
fn m4_automatic_compaction_keeps_the_garbage_below_the_threshold() {
    const THRESHOLD: u64 = 2000;
    const ROUNDS: u64 = 5;
    let dir = TestDir::new("m4_auto");
    let server = Server::start_with(dir.path(), &["--fsync", "never", "--auto-compact", "2000"]);
    let mut c = server.client();
    let value = "v".repeat(100);
    let mut content = Model::new();
    let mut compactions = 0;
    for round in 0..ROUNDS {
        // Three keys, overwritten again and again: 112 or 113 bytes each
        // time. Sixty overwrites are about 6700 bytes of garbage, more than
        // three times the threshold.
        for i in 0..60 {
            let key = format!("k{}", i % 3);
            c.set(&key, &value);
            content.insert(key.into_bytes(), value.clone().into_bytes());
        }
        if round == 2 {
            assert!(c.del("k1"));
            content.remove(&b"k1"[..]);
        }

        // The server may compact before it answers the write that crossed
        // the threshold, or in the background within 5 seconds, but in
        // both cases without another request. So do not send one: watch
        // the size of the file.
        let live = live_bytes(&content);
        let deadline = std::time::Instant::now() + common::patience();
        loop {
            // During the rename the file can be missing for an instant on
            // some systems; then look again.
            let log_bytes = std::fs::metadata(dir.log()).map(|m| m.len()).ok();
            if log_bytes.is_some_and(|n| n.saturating_sub(live) < THRESHOLD) {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "round {round}: with --auto-compact 2000 the server must compact by itself, \
                 without a further request, until less than 2000 bytes of the log are garbage \
                 (SPEC 10.3). After {} s kv.log has {log_bytes:?} bytes, and {live} of them \
                 are live",
                common::patience().as_secs()
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        // The file is replaced a moment before the server's numbers are;
        // ask until they agree.
        let stats = wait_for_stats(
            &mut c,
            "after an automatic compaction STATS must show it (SPEC 8.9, SPEC 10.3)",
            |stats| stats.compactions > compactions && stats.garbage() < THRESHOLD,
        );
        assert_eq!(
            stats.live_bytes, live,
            "round {round}: live_bytes after automatic compactions"
        );
        compactions = stats.compactions;
    }
    assert!(
        compactions >= ROUNDS,
        "the garbage went over the threshold in each of {ROUNDS} rounds, so there must have \
         been at least {ROUNDS} automatic compactions; STATS says {compactions}"
    );
    assert_eq!(
        c.dump(),
        content,
        "automatic compaction must not change the content"
    );
    server.kill();
    let parsed = parse_log(&dir.read_log());
    parsed.expect_clean("a log that was compacted automatically");
    assert_eq!(parsed.replay(), content);
}

#[test]
fn m4_no_automatic_compaction_below_the_threshold_or_when_switched_off() {
    let value = "v".repeat(100);
    let record = record_len(b"k", value.as_bytes());

    // Switched off: 300 overwrites pile up.
    let dir = TestDir::new("m4_auto_off");
    let server = Server::start_with(dir.path(), &["--fsync", "never", "--auto-compact", "0"]);
    let mut c = server.client();
    for _ in 0..300 {
        c.set("k", &value);
    }
    assert_eq!(
        c.stats(),
        Stats {
            keys: 1,
            log_bytes: 8 + 300 * record,
            live_bytes: 8 + record,
            compactions: 0
        },
        "`--auto-compact 0` switches automatic compaction off (SPEC 10.3)"
    );
    drop(c);
    drop(server);

    // Threshold 5000: nine overwrites make 9 * 112 = 1008 bytes of garbage.
    let dir = TestDir::new("m4_auto_below");
    let server = Server::start_with(dir.path(), &["--fsync", "never", "--auto-compact", "5000"]);
    let mut c = server.client();
    for _ in 0..10 {
        c.set("k", &value);
    }
    assert_eq!(
        c.stats(),
        Stats {
            keys: 1,
            log_bytes: 8 + 10 * record,
            live_bytes: 8 + record,
            compactions: 0
        },
        "while the garbage is below the threshold the server must not compact (SPEC 10.3)"
    );
}
