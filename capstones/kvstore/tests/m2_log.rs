//! Milestone 2: the log file. Every write is appended to `kv.log` in the
//! format of SPEC 5 before it is acknowledged, and a restarted server reads
//! the file back.
//!
//! The logs in this milestone are always intact; damaged logs are
//! milestone 3. A server is stopped with a kill, never with `SHUTDOWN`:
//! what has been acknowledged must already be in the file.
//!
//! ```text
//! cargo test --test m2_log
//! ```

// Miri cannot start other programs or open sockets.
#![cfg(not(miri))]

mod common;

use std::net::TcpListener;

use common::{
    COMPACT_FILE, MAGIC, MAX_KEY, MAX_VALUE, Model, Record, Server, Stats, TestDir, del_record,
    expect_refusal, live_bytes, log_of, model, os_args, parse_log, record_len, server_args,
    set_record, show,
};

#[test]
fn m2_creates_the_directory_and_a_log_with_only_the_header() {
    let dir = TestDir::new("m2_create");
    let nested = dir.path().join("a").join("b");
    let server = Server::start(&nested);
    let log = std::fs::read(nested.join("kv.log")).unwrap_or_else(|e| {
        panic!(
            "a new server must create <dir>/kv.log, and <dir> itself if it is missing (SPEC 4): {e}"
        )
    });
    assert_eq!(
        show(&log),
        "KVSTORE1",
        "a new log is exactly the 8 magic bytes (SPEC 5.1), written before the LISTENING line"
    );
    let mut c = server.client();
    assert_eq!(
        c.stats(),
        Stats {
            keys: 0,
            log_bytes: 8,
            live_bytes: 8,
            compactions: 0
        },
        "the STATS of an empty store (SPEC 8.9)"
    );
    assert!(
        !nested.join(COMPACT_FILE).exists(),
        "kv.compact exists only while a compaction runs (SPEC 4)"
    );
}

#[test]
fn m2_the_example_of_spec_5_5() {
    let dir = TestDir::new("m2_example");
    let server = Server::start(dir.path());
    let mut c = server.client();
    c.set("fruit", "apple");
    c.set("n", "");
    assert!(c.del("fruit"));
    #[rustfmt::skip]
    let expected: &[u8] = &[
        // header "KVSTORE1"
        0x4B, 0x56, 0x53, 0x54, 0x4F, 0x52, 0x45, 0x31,
        // SET fruit apple: crc, kind, klen, vlen, key, value
        0xB9, 0xE8, 0x3E, 0xB7, 0x01, 0x05, 0x00, 0x05, 0x00, 0x00, 0x00,
        0x66, 0x72, 0x75, 0x69, 0x74, 0x61, 0x70, 0x70, 0x6C, 0x65,
        // SET n (empty value)
        0x1C, 0x98, 0xF5, 0xA5, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x6E,
        // DEL fruit
        0x38, 0xD1, 0x90, 0x80, 0x02, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x66, 0x72, 0x75, 0x69, 0x74,
    ];
    let log = dir.read_log();
    let first_difference = log.iter().zip(expected).position(|(a, b)| a != b);
    assert!(
        log == expected,
        "after `SET fruit apple`, `SET n ` and `DEL fruit` on a new store, kv.log must be the 57 \
         bytes of SPEC 5.5. Your file has {} bytes; the first difference is at offset {:?} \
         (None: one file is a beginning of the other).\nyour file: {}",
        log.len(),
        first_difference,
        log.iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
}

#[test]
fn m2_a_write_is_in_the_file_before_it_is_acknowledged() {
    let dir = TestDir::new("m2_append");
    let server = Server::start(dir.path());
    let mut c = server.client();
    let mut expected = MAGIC.to_vec();

    c.set("fruit", "apple");
    expected.extend_from_slice(&set_record("fruit", "apple"));
    assert_eq!(
        dir.read_log(),
        expected,
        "after the OK for `SET fruit apple` the file must hold the header and one SET record \
         of 11 + 5 + 5 bytes (SPEC 5.2, SPEC 9.1). If the file is shorter, the record is still \
         in a buffer of your process."
    );

    c.set("n", "");
    expected.extend_from_slice(&set_record("n", ""));
    assert_eq!(
        dir.read_log(),
        expected,
        "an empty value gives a SET record with vlen 0"
    );

    c.set("fruit", "apple");
    expected.extend_from_slice(&set_record("fruit", "apple"));
    assert_eq!(
        dir.read_log(),
        expected,
        "every successful SET appends a record, also when the value does not change (SPEC 8.5)"
    );

    assert!(c.del("fruit"));
    expected.extend_from_slice(&del_record("fruit"));
    assert_eq!(
        dir.read_log(),
        expected,
        "DEL of a live key appends a DEL record of 11 + 5 bytes (SPEC 8.6)"
    );

    assert_eq!(c.req("INCR hits"), "VALUE 1");
    expected.extend_from_slice(&set_record("hits", "1"));
    assert_eq!(c.req("INCR hits"), "VALUE 2");
    expected.extend_from_slice(&set_record("hits", "2"));
    assert_eq!(
        dir.read_log(),
        expected,
        "a successful INCR appends a SET record with the new number (SPEC 8.7)"
    );
}

#[test]
fn m2_requests_that_change_nothing_append_nothing() {
    let dir = TestDir::new("m2_no_append");
    let server = Server::start(dir.path());
    let mut c = server.client();
    c.set("text", "abc");
    let expected = log_of(&[set_record("text", "abc")]);
    let requests: Vec<Vec<u8>> = vec![
        b"DEL missing".to_vec(),
        b"INCR text".to_vec(),
        b"GET text".to_vec(),
        b"GET missing".to_vec(),
        b"KEYS".to_vec(),
        b"STATS".to_vec(),
        b"PING".to_vec(),
        b"SET".to_vec(),
        b"SET bad\x7fkey v".to_vec(),
        format!("SET big {}", "v".repeat(MAX_VALUE + 1)).into_bytes(),
        b"NOPE".to_vec(),
    ];
    for request in requests {
        let reply = c.req_bytes(&request);
        if reply.starts_with(b"KEYS ") {
            c.recv();
        }
        assert_eq!(
            dir.read_log(),
            expected,
            "`{}` (answered `{}`) must not append anything: only a successful SET, INCR or DEL \
             of a live key writes a record (SPEC 8)",
            show(&request),
            show(&reply)
        );
    }
}

#[test]
fn m2_stats_follow_the_log_and_the_live_set() {
    let dir = TestDir::new("m2_stats");
    let server = Server::start(dir.path());
    let mut c = server.client();
    let mut content = Model::new();
    let mut log_bytes = 8;
    let steps: &[(&str, Option<&str>)] = &[
        ("a", Some("1")),
        ("bb", Some("22")),
        ("a", Some("a longer value")),
        ("ccc", Some("")),
        ("bb", None),
        ("a", Some("x")),
        ("ccc", None),
    ];
    for (key, value) in steps {
        match value {
            Some(value) => {
                c.set(key, value);
                content.insert(key.as_bytes().to_vec(), value.as_bytes().to_vec());
                log_bytes += record_len(key.as_bytes(), value.as_bytes());
            }
            None => {
                assert!(c.del(key));
                content.remove(key.as_bytes());
                log_bytes += record_len(key.as_bytes(), b"");
            }
        }
        let expected = Stats {
            keys: content.len() as u64,
            log_bytes,
            live_bytes: live_bytes(&content),
            compactions: 0,
        };
        assert_eq!(
            c.stats(),
            expected,
            "after {key} = {value:?}: log_bytes is the size of kv.log; live_bytes is 8 plus \
             11 + klen + vlen for every live key (SPEC 8.9)"
        );
        assert_eq!(
            dir.log_len(),
            log_bytes,
            "log_bytes must be the real size of kv.log"
        );
    }
}

/// Some content with overwrites and deletions, applied through `c`.
fn busy_history(c: &mut common::Client) -> Model {
    let mut content = Model::new();
    for i in 0..300 {
        let key = format!("key{:03}", i % 120);
        let value = format!("value-{i}-{}", "x".repeat(i % 40));
        c.set(&key, &value);
        content.insert(key.into_bytes(), value.into_bytes());
    }
    for i in (0..120).step_by(7) {
        let key = format!("key{i:03}");
        assert!(c.del(&key));
        content.remove(key.as_bytes());
    }
    c.set("empty", "");
    content.insert(b"empty".to_vec(), Vec::new());
    content
}

#[test]
fn m2_content_survives_a_kill() {
    let dir = TestDir::new("m2_kill");
    let server = Server::start(dir.path());
    let mut c = server.client();
    let content = busy_history(&mut c);
    let before = c.stats();
    server.kill();

    let server = Server::start(dir.path());
    let mut c = server.client();
    assert_eq!(
        c.dump(),
        content,
        "after a kill and a restart the store holds every acknowledged write, and deleted keys \
         stay deleted (SPEC 6)"
    );
    assert_eq!(
        c.stats(),
        before,
        "replaying an intact log gives the same STATS as before the kill (compactions was 0)"
    );
}

#[test]
fn m2_content_survives_several_restarts() {
    let dir = TestDir::new("m2_restarts");
    let mut content = Model::new();
    for round in 0..5 {
        let server = Server::start(dir.path());
        let mut c = server.client();
        assert_eq!(c.dump(), content, "content at the start of round {round}");
        for i in 0..20 {
            let key = format!("k{}", (round * 7 + i) % 30);
            let value = format!("r{round}i{i}");
            c.set(&key, &value);
            content.insert(key.into_bytes(), value.into_bytes());
        }
        let victim = format!("k{}", round * 3);
        if c.del(&victim) {
            content.remove(victim.as_bytes());
        }
        server.kill();
    }
    let parsed = parse_log(&dir.read_log());
    parsed.expect_clean("after five rounds of writes and kills");
    assert_eq!(
        parsed.replay(),
        content,
        "replaying kv.log by the rules of SPEC 6 gives the content"
    );
}

#[test]
fn m2_reads_a_log_that_another_program_wrote() {
    let dir = TestDir::new("m2_handwritten");
    let long_key = "K".repeat(MAX_KEY);
    let long_value = vec![0xA5u8; MAX_VALUE];
    let log = log_of(&[
        set_record("b", "first"),
        set_record("a", "alpha"),
        set_record("b", "second"),
        del_record("never-set"),
        set_record("gone", "soon"),
        set_record("empty", ""),
        del_record("gone"),
        set_record(&long_key, &long_value),
        set_record("bytes", b"\x00\xff\r\t "),
        set_record("n", "41"),
    ]);
    dir.write_log(&log);

    let server = Server::start(dir.path());
    assert_eq!(
        dir.read_log(),
        log,
        "starting on an intact log must not change the file (SPEC 6)"
    );
    let mut c = server.client();
    let mut expected = model(&[("a", "alpha"), ("b", "second"), ("empty", ""), ("n", "41")]);
    expected.insert(long_key.clone().into_bytes(), long_value);
    expected.insert(b"bytes".to_vec(), b"\x00\xff\r\t ".to_vec());
    assert_eq!(
        c.dump(),
        expected,
        "the store is the result of replaying the records in file order; a DEL of a key that \
         is not live is valid and does nothing (SPEC 6)"
    );
    assert_eq!(
        c.stats(),
        Stats {
            keys: 6,
            log_bytes: log.len() as u64,
            live_bytes: live_bytes(&expected),
            compactions: 0,
        },
        "STATS after replaying a log"
    );
    assert_eq!(
        c.req("INCR n"),
        "VALUE 42",
        "a recovered value behaves like one that was just set"
    );
    let mut expected_log = log.clone();
    expected_log.extend_from_slice(&set_record("n", "42"));
    assert_eq!(
        dir.read_log(),
        expected_log,
        "new records go to the end of the recovered log"
    );
}

#[test]
fn m2_the_log_can_be_read_back_by_the_rules_of_the_spec() {
    let dir = TestDir::new("m2_parse");
    let server = Server::start(dir.path());
    let mut c = server.client();
    c.set("x", "1");
    c.set("y", "two words");
    assert!(c.del("x"));
    c.set(b"z", b"\xff\x00");
    server.kill();
    let parsed = parse_log(&dir.read_log());
    parsed.expect_clean("after four acknowledged writes and a kill");
    assert_eq!(
        parsed.records,
        [
            Record::set("x", "1"),
            Record::set("y", "two words"),
            Record::del("x"),
            Record::Set(b"z".to_vec(), b"\xff\x00".to_vec()),
        ],
        "one record per write, in the order of the acknowledgements, each with a correct \
         CRC-32 (SPEC 5)"
    );
}

#[test]
fn m2_a_second_server_on_the_same_directory_is_refused() {
    let dir = TestDir::new("m2_lock");
    let first = Server::start(dir.path());
    let mut c = first.client();
    c.set("owner", "first");

    expect_refusal(
        &server_args(dir.path(), common::FAST),
        3,
        "a second server on a data directory that is in use (SPEC 4)",
    );
    assert_eq!(
        c.get("owner").as_deref(),
        Some(&b"first"[..]),
        "the first server must be unharmed by the refused second one"
    );
    c.set("still", "writable");
    assert_eq!(
        parse_log(&dir.read_log()).records,
        [
            Record::set("owner", "first"),
            Record::set("still", "writable")
        ],
        "the refused server must not have touched the log"
    );

    // A killed server cannot clean up. Its lock must go away with it.
    first.kill();
    let second = Server::start(dir.path());
    assert_eq!(
        second.client().get("owner").as_deref(),
        Some(&b"first"[..]),
        "after the first server was killed, a new one must start on the same directory: the \
         lock has to disappear with the process, however it ends (SPEC 4)"
    );
}

#[test]
fn m2_a_file_that_is_not_a_log_is_refused_and_left_alone() {
    let dir = TestDir::new("m2_foreign");
    let foreign: &[&[u8]] = &[
        b"This is somebody's notes file, not a log.\n",
        b"KVSTORE2",
        b"kvstore1 and then some bytes",
        b"KVSTORE!",
        b"KVS\n",
        b"\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
    ];
    for content in foreign {
        dir.write_log(content);
        expect_refusal(
            &server_args(dir.path(), common::FAST),
            3,
            &format!(
                "kv.log starts with `{}`, which is not the magic bytes (SPEC 6)",
                show(content)
            ),
        );
        assert_eq!(
            show(&dir.read_log()),
            show(content),
            "a file that is not a log must not be changed: it may be somebody's data (SPEC 6)"
        );
    }
}

#[test]
fn m2_a_port_that_is_in_use_is_a_start_up_failure() {
    let dir = TestDir::new("m2_port_in_use");
    // Keep a listener on a port of the system's choice, then ask the server
    // for exactly that port.
    let occupied = TcpListener::bind("127.0.0.1:0").expect("cannot bind a test listener");
    let port = occupied
        .local_addr()
        .expect("the listener has an address")
        .port()
        .to_string();
    let dir_arg = dir
        .path()
        .to_str()
        .expect("the test directory is valid UTF-8");
    expect_refusal(
        &os_args(&["--dir", dir_arg, "--port", &port, "--fsync", "never"]),
        3,
        "the port is already in use (SPEC 3.4)",
    );
}

#[test]
fn m2_a_data_directory_that_cannot_be_created_is_a_start_up_failure() {
    let dir = TestDir::new("m2_no_dir");
    // A file stands where the data directory, or one of its parents, would
    // have to be. That fails in the same way on every system.
    let blocker = dir.path().join("a-file");
    std::fs::write(&blocker, b"not a directory").expect("cannot write a test file");
    for data in [blocker.clone(), blocker.join("below")] {
        let data_arg = data.to_str().expect("the test directory is valid UTF-8");
        expect_refusal(
            &os_args(&["--dir", data_arg, "--port", "0", "--fsync", "never"]),
            3,
            "the data directory cannot be created or opened (SPEC 3.4). An `unwrap` on the \
             error gives exit code 101 instead",
        );
    }
    assert_eq!(
        std::fs::read(&blocker).expect("cannot read the test file back"),
        b"not a directory",
        "a file in the way of the data directory must not be changed"
    );
}

#[test]
fn m2_options_may_come_in_any_order_and_fsync_always_is_the_default() {
    let dir = TestDir::new("m2_defaults");
    let dir_arg = dir
        .path()
        .to_str()
        .expect("the test directory is valid UTF-8");
    // Only --dir and --port: fsync is `always`, automatic compaction is on
    // with its large default threshold. A few writes keep this quick even
    // on a disk with slow syncs.
    let server =
        Server::launch(&os_args(&["--port", "0", "--dir", dir_arg])).unwrap_or_else(|exited| {
            panic!("`kvstore --port 0 --dir <dir>` must start (SPEC 3.1): {exited}")
        });
    let mut c = server.client();
    for i in 0..10 {
        c.set(format!("k{i}"), format!("v{i}"));
    }
    assert!(c.del("k3"));
    server.kill();

    let server = Server::launch(&os_args(&[
        "--max-clients",
        "8",
        "--auto-compact",
        "0",
        "--fsync",
        "always",
        "--port",
        "0",
        "--dir",
        dir_arg,
    ]))
    .unwrap_or_else(|exited| panic!("the options may come in any order (SPEC 3.1): {exited}"));
    let mut c = server.client();
    assert_eq!(
        c.stats().keys,
        9,
        "content written in the default fsync mode survives a kill"
    );
    assert_eq!(c.get("k9").as_deref(), Some(&b"v9"[..]));
}
