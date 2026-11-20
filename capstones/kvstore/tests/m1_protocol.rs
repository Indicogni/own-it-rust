//! Milestone 1: the server starts, speaks the protocol and stops.
//!
//! Nothing here needs a log file: a server that keeps everything in memory
//! passes this milestone. Each test uses one connection at a time.
//!
//! ```text
//! cargo test --test m1_protocol
//! ```

// Miri cannot start other programs or open sockets.
#![cfg(not(miri))]

mod common;

use std::ffi::OsString;

use common::{
    Client, MAX_KEY, MAX_LINE, MAX_VALUE, Server, TestDir, expect_refusal, os_args, show,
};

/// Sends `request` and requires exactly `expected` as the answer.
#[track_caller]
fn expect(client: &mut Client, request: impl AsRef<[u8]>, expected: &str, why: &str) {
    let request = request.as_ref();
    let reply = client.req_bytes(request);
    assert_eq!(show(&reply), expected, "request `{}`: {why}", show(request));
}

#[test]
fn m1_announces_its_address_and_accepts_a_connection() {
    let dir = TestDir::new("m1_announce");
    let server = Server::start(dir.path());
    assert!(
        server.addr.ip().is_loopback() && server.addr.port() != 0,
        "with --port 0 the LISTENING line must name the port the system chose"
    );
    let mut client = server.client();
    expect(
        &mut client,
        "PING",
        "PONG",
        "PING is answered with PONG (SPEC 8.3)",
    );
}

#[test]
fn m1_ping_can_be_repeated_on_one_connection() {
    let dir = TestDir::new("m1_ping");
    let server = Server::start(dir.path());
    let mut client = server.client();
    for _ in 0..50 {
        expect(
            &mut client,
            "PING",
            "PONG",
            "a connection stays open and answers request after request (SPEC 8.1)",
        );
    }
}

#[test]
fn m1_set_get_del_round_trip() {
    let dir = TestDir::new("m1_roundtrip");
    let server = Server::start(dir.path());
    let mut c = server.client();
    expect(
        &mut c,
        "GET fruit",
        "NOT_FOUND",
        "a key that was never set is NOT_FOUND (SPEC 8.4)",
    );
    expect(
        &mut c,
        "SET fruit apple",
        "OK",
        "SET is answered with OK (SPEC 8.5)",
    );
    expect(
        &mut c,
        "GET fruit",
        "VALUE apple",
        "GET returns the value that was set (SPEC 8.4)",
    );
    expect(
        &mut c,
        "SET fruit pear",
        "OK",
        "SET on an existing key replaces the value",
    );
    expect(
        &mut c,
        "GET fruit",
        "VALUE pear",
        "GET returns the newest value",
    );
    expect(
        &mut c,
        "DEL fruit",
        "OK",
        "DEL of an existing key is answered with OK (SPEC 8.6)",
    );
    expect(
        &mut c,
        "GET fruit",
        "NOT_FOUND",
        "a deleted key is NOT_FOUND",
    );
    expect(
        &mut c,
        "DEL fruit",
        "NOT_FOUND",
        "DEL of a missing key is NOT_FOUND (SPEC 8.6)",
    );
}

#[test]
fn m1_value_is_everything_after_the_second_space() {
    let dir = TestDir::new("m1_value_spaces");
    let server = Server::start(dir.path());
    let mut c = server.client();
    expect(
        &mut c,
        "SET k ",
        "OK",
        "`SET k ` sets the empty value (SPEC 8.5)",
    );
    expect(
        &mut c,
        "GET k",
        "VALUE ",
        "an empty value is sent as `VALUE ` (SPEC 8.4)",
    );
    expect(
        &mut c,
        "SET k  two  spaces ",
        "OK",
        "a value may contain spaces",
    );
    expect(
        &mut c,
        "GET k",
        "VALUE  two  spaces ",
        "leading, inner and trailing spaces of a value are kept exactly",
    );
    expect(
        &mut c,
        "SET k GET k",
        "OK",
        "a value may look like a command",
    );
    expect(
        &mut c,
        "GET k",
        "VALUE GET k",
        "the value is not interpreted",
    );
}

#[test]
fn m1_values_are_bytes_not_text() {
    let dir = TestDir::new("m1_value_bytes");
    let server = Server::start(dir.path());
    let mut c = server.client();
    // Not valid UTF-8, with a NUL, a carriage return and a tab.
    let value: &[u8] = b"\x00\xff\xfe caf\xe9\r\t\x80";
    c.set("raw", value);
    assert_eq!(
        c.get("raw").as_deref(),
        Some(value),
        "a value is any bytes except a line feed and must come back unchanged (SPEC 7). \
         Reading lines into a `String` breaks this."
    );
    let every_byte: Vec<u8> = (0..=255u8).filter(|&b| b != b'\n').collect();
    c.set("all", &every_byte);
    assert_eq!(
        c.get("all").as_deref(),
        Some(&every_byte[..]),
        "all 255 byte values other than a line feed are allowed in a value (SPEC 7)"
    );
}

#[test]
fn m1_keys_are_case_sensitive_printable_ascii() {
    let dir = TestDir::new("m1_keys_ascii");
    let server = Server::start(dir.path());
    let mut c = server.client();
    c.set("Key", "upper");
    c.set("key", "lower");
    assert_eq!(
        c.get("Key").as_deref(),
        Some(&b"upper"[..]),
        "keys are compared byte by byte"
    );
    assert_eq!(
        c.get("key").as_deref(),
        Some(&b"lower"[..]),
        "keys are compared byte by byte"
    );
    let odd = "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";
    c.set(odd, "punctuation");
    assert_eq!(
        c.get(odd).as_deref(),
        Some(&b"punctuation"[..]),
        "every byte from 0x21 to 0x7E is allowed in a key (SPEC 7)"
    );
    let longest = "k".repeat(MAX_KEY);
    c.set(&longest, "long");
    assert_eq!(
        c.get(&longest).as_deref(),
        Some(&b"long"[..]),
        "a key of exactly 128 bytes is allowed (SPEC 7)"
    );
}

#[test]
fn m1_keys_lists_in_byte_order_and_by_prefix() {
    let dir = TestDir::new("m1_keys");
    let server = Server::start(dir.path());
    let mut c = server.client();
    expect(
        &mut c,
        "KEYS",
        "KEYS 0",
        "an empty store lists no keys (SPEC 8.8)",
    );
    for key in ["b", "a", "B", "ab", "a1", "~", "abc", "aa", "0"] {
        c.set(key, "x");
    }
    c.set("gone", "x");
    assert!(c.del("gone"));
    assert_eq!(
        c.keys(None),
        ["0", "B", "a", "a1", "aa", "ab", "abc", "b", "~"],
        "KEYS lists every live key in ascending byte order: digits, then upper case, then \
         lower case (SPEC 8.8)"
    );
    assert_eq!(
        c.keys(Some("a")),
        ["a", "a1", "aa", "ab", "abc"],
        "`KEYS a` lists the keys that start with `a`, the key `a` itself included (SPEC 8.8)"
    );
    assert_eq!(
        c.keys(Some("ab")),
        ["ab", "abc"],
        "a longer prefix selects fewer keys"
    );
    assert_eq!(
        c.keys(Some("zz")),
        Vec::<String>::new(),
        "a prefix that no key has gives `KEYS 0`"
    );
    expect(
        &mut c,
        "PING",
        "PONG",
        "after a KEYS answer the connection continues normally",
    );
}

#[test]
fn m1_stats_counts_live_keys() {
    let dir = TestDir::new("m1_stats");
    let server = Server::start(dir.path());
    let mut c = server.client();
    assert_eq!(c.stats().keys, 0, "a new store has keys=0 (SPEC 8.9)");
    c.set("a", "1");
    c.set("b", "2");
    c.set("a", "3");
    assert_eq!(
        c.stats().keys,
        2,
        "keys counts each live key once, however often it was set"
    );
    assert!(c.del("a"));
    assert_eq!(c.stats().keys, 1, "a deleted key is no longer counted");
}

#[test]
fn m1_incr_counts_from_zero() {
    let dir = TestDir::new("m1_incr");
    let server = Server::start(dir.path());
    let mut c = server.client();
    expect(
        &mut c,
        "INCR hits",
        "VALUE 1",
        "INCR on a missing key starts from 0 (SPEC 8.7)",
    );
    expect(&mut c, "INCR hits", "VALUE 2", "INCR adds 1");
    expect(
        &mut c,
        "GET hits",
        "VALUE 2",
        "INCR stores the new number as decimal text",
    );
    expect(&mut c, "SET n 41", "OK", "");
    expect(
        &mut c,
        "INCR n",
        "VALUE 42",
        "INCR works on a number that SET stored",
    );
    expect(&mut c, "SET n -1", "OK", "");
    expect(
        &mut c,
        "INCR n",
        "VALUE 0",
        "negative numbers are numbers (SPEC 8.7)",
    );
    expect(&mut c, "SET n 007", "OK", "");
    expect(
        &mut c,
        "INCR n",
        "VALUE 8",
        "leading zeros are accepted and not written back",
    );
    expect(&mut c, "SET n +5", "OK", "");
    expect(
        &mut c,
        "INCR n",
        "VALUE 6",
        "a plus sign is accepted and not written back",
    );
    expect(&mut c, "SET n -9223372036854775808", "OK", "");
    expect(
        &mut c,
        "INCR n",
        "VALUE -9223372036854775807",
        "the smallest 64-bit number can be incremented",
    );
    expect(&mut c, "SET n 0000000000000000000000000000041", "OK", "");
    expect(
        &mut c,
        "INCR n",
        "VALUE 42",
        "the numeric value decides, not the number of digits: 31 digits with leading zeros \
         are still the number 41 (SPEC 8.7)",
    );
    expect(&mut c, "SET n -0", "OK", "");
    expect(
        &mut c,
        "INCR n",
        "VALUE 1",
        "`-0` is a sign and a digit, so it is the integer 0 (SPEC 8.7)",
    );
}

#[test]
fn m1_incr_errors_change_nothing() {
    let dir = TestDir::new("m1_incr_errors");
    let server = Server::start(dir.path());
    let mut c = server.client();
    for not_a_number in [
        "abc",
        "",
        "1.5",
        " 5",
        "5 ",
        "0x10",
        "-",
        "+",
        "9223372036854775808",
        "1e3",
    ] {
        c.set("n", not_a_number);
        expect(
            &mut c,
            "INCR n",
            "ERR not an integer",
            &format!("the value `{not_a_number}` is not a 64-bit decimal integer (SPEC 8.7)"),
        );
        assert_eq!(
            c.get("n").as_deref(),
            Some(not_a_number.as_bytes()),
            "a failed INCR must leave the value as it was"
        );
    }
    c.set("n", "9223372036854775807");
    expect(
        &mut c,
        "INCR n",
        "ERR overflow",
        "the largest 64-bit number cannot be incremented (SPEC 8.7)",
    );
    expect(
        &mut c,
        "GET n",
        "VALUE 9223372036854775807",
        "a failed INCR must leave the value as it was",
    );
}

#[test]
fn m1_unknown_commands() {
    let dir = TestDir::new("m1_unknown");
    let server = Server::start(dir.path());
    let mut c = server.client();
    let why = "a first word that is not a command name gives `ERR unknown command` (SPEC 8.2)";
    for request in [
        "FOO", "ping", "Get a", "", " ", " PING", "PING\r", "GETa", "SETS a b", "FOO a b",
    ] {
        expect(&mut c, request, "ERR unknown command", why);
    }
    expect(
        &mut c,
        "PING",
        "PONG",
        "after an error the connection continues normally (SPEC 8.13)",
    );
}

#[test]
fn m1_syntax_errors() {
    let dir = TestDir::new("m1_syntax");
    let server = Server::start(dir.path());
    let mut c = server.client();
    let why = "a known command with the wrong shape gives `ERR syntax` (SPEC 8.2)";
    for request in [
        "GET",
        "DEL",
        "INCR",
        "SET",
        "SET k",
        "PING ",
        "PING x",
        "STATS ",
        "STATS all",
        "COMPACT now",
        "QUIT now",
        "SHUTDOWN now",
        "SHUTDOWN ",
    ] {
        expect(&mut c, request, "ERR syntax", why);
    }
    expect(
        &mut c,
        "PING",
        "PONG",
        "`QUIT now` and `SHUTDOWN now` are errors: the connection and the server must go on",
    );
    assert_eq!(
        c.stats().keys,
        0,
        "a request that was answered with ERR must not change the store"
    );
}

#[test]
fn m1_bad_keys() {
    let dir = TestDir::new("m1_bad_keys");
    let server = Server::start(dir.path());
    let mut c = server.client();
    let too_long = "k".repeat(MAX_KEY + 1);
    let cases: Vec<(Vec<u8>, &str)> = vec![
        (b"GET ".to_vec(), "an empty key"),
        (b"DEL ".to_vec(), "an empty key"),
        (b"INCR ".to_vec(), "an empty key"),
        (b"KEYS ".to_vec(), "an empty prefix"),
        (
            b"SET  value".to_vec(),
            "an empty key (two spaces after SET)",
        ),
        (
            b"GET a b".to_vec(),
            "a space in the key of GET: everything after `GET ` is the key",
        ),
        (b"DEL a b".to_vec(), "a space in the key of DEL"),
        (b"INCR a ".to_vec(), "a trailing space belongs to the key"),
        (b"KEYS a b".to_vec(), "a space in the prefix"),
        (format!("GET {too_long}").into_bytes(), "a key of 129 bytes"),
        (
            format!("SET {too_long} v").into_bytes(),
            "a key of 129 bytes",
        ),
        (
            format!("KEYS {too_long}").into_bytes(),
            "a prefix of 129 bytes",
        ),
        (b"GET caf\xc3\xa9".to_vec(), "bytes above 0x7E"),
        (b"GET a\x7f".to_vec(), "the byte 0x7F"),
        (b"GET a\tb".to_vec(), "a tab"),
        (b"SET a\x00b v".to_vec(), "a NUL byte"),
        (
            b"GET a\r".to_vec(),
            "a carriage return: the protocol does not treat CR specially",
        ),
    ];
    for (request, what) in &cases {
        expect(
            &mut c,
            request,
            "ERR bad key",
            &format!("{what} gives `ERR bad key` (SPEC 7, SPEC 8.2)"),
        );
    }
    assert_eq!(
        c.stats().keys,
        0,
        "a request that was answered with ERR must not change the store"
    );
}

#[test]
fn m1_value_limit() {
    let dir = TestDir::new("m1_value_limit");
    let server = Server::start(dir.path());
    let mut c = server.client();
    let largest = vec![b'v'; MAX_VALUE];
    c.set("big", &largest);
    assert_eq!(
        c.get("big").as_deref(),
        Some(&largest[..]),
        "a value of exactly 4096 bytes is allowed (SPEC 7)"
    );
    let too_large = "w".repeat(MAX_VALUE + 1);
    expect(
        &mut c,
        format!("SET big {too_large}"),
        "ERR value too large",
        "a value of 4097 bytes is refused (SPEC 8.2)",
    );
    assert_eq!(
        c.get("big").as_deref(),
        Some(&largest[..]),
        "a refused SET must leave the old value in place"
    );
    // The longest line that is not "too long": 4 + 1 + 1 + 8186 = 8192 bytes.
    let fills_the_line = "x".repeat(MAX_LINE - 6);
    expect(
        &mut c,
        format!("SET k {fills_the_line}"),
        "ERR value too large",
        "a line of exactly 8192 bytes is parsed normally; here its value is too large (SPEC 8.1)",
    );
}

#[test]
fn m1_errors_are_checked_in_the_order_of_the_spec() {
    let dir = TestDir::new("m1_error_order");
    let server = Server::start(dir.path());
    let mut c = server.client();
    let too_large = "w".repeat(MAX_VALUE + 1);
    let too_long_key = "k".repeat(MAX_KEY + 1);
    expect(
        &mut c,
        format!("SET {too_long_key} {too_large}"),
        "ERR bad key",
        "the key is checked before the value (SPEC 8.2)",
    );
    expect(
        &mut c,
        format!("FOO {too_long_key} {too_large}"),
        "ERR unknown command",
        "the command name is checked first (SPEC 8.2)",
    );
    expect(
        &mut c,
        format!("GET {}", "k".repeat(MAX_LINE)),
        "ERR line too long",
        "the length of the line is checked before anything else (SPEC 8.1)",
    );
}

#[test]
fn m1_a_line_that_is_too_long_is_skipped() {
    let dir = TestDir::new("m1_line_too_long");
    let server = Server::start(dir.path());
    let mut c = server.client();
    expect(
        &mut c,
        "x".repeat(MAX_LINE + 1),
        "ERR line too long",
        "a line of 8193 bytes is too long (SPEC 8.1)",
    );
    expect(
        &mut c,
        "PING",
        "PONG",
        "after a line that was too long the connection continues",
    );
    // A megabyte without a line feed must not be collected in memory; the
    // server has to throw it away while it reads.
    expect(
        &mut c,
        vec![b'y'; 1 << 20],
        "ERR line too long",
        "a line of one megabyte gets one `ERR line too long` (SPEC 8.1)",
    );
    expect(
        &mut c,
        "SET after ok",
        "OK",
        "the request after the long line is the next line, not a part of the long one",
    );
    assert_eq!(
        c.keys(None),
        ["after"],
        "nothing inside a line that was too long is executed"
    );
}

#[test]
fn m1_pipelined_requests_are_answered_in_order() {
    const N: usize = 2000;
    let dir = TestDir::new("m1_pipeline");
    let server = Server::start(dir.path());
    let mut c = server.client();
    // All requests go out in one piece, before any answer is read.
    let mut requests = Vec::new();
    for i in 0..N {
        requests.extend_from_slice(format!("SET key{i} value{i}\nGET key{i}\n").as_bytes());
    }
    let replies = c.pipeline(&requests, 2 * N);
    for i in 0..N {
        assert_eq!(
            show(&replies[2 * i]),
            "OK",
            "response {} of the pipeline (to `SET key{i} value{i}`)",
            2 * i + 1
        );
        assert_eq!(
            show(&replies[2 * i + 1]),
            format!("VALUE value{i}"),
            "response {} of the pipeline (to `GET key{i}`): responses come in the order of the \
             requests (SPEC 8.1)",
            2 * i + 2
        );
    }
}

#[test]
fn m1_last_line_without_line_feed_is_not_executed() {
    let dir = TestDir::new("m1_unterminated");
    let server = Server::start(dir.path());
    let mut c = server.client();
    c.send_raw(b"SET a 1\nSET b 2");
    c.finish_sending();
    assert_eq!(
        show(&c.read_to_end()),
        "OK\\n",
        "when the client stops sending, the server answers the complete lines, ignores an \
         unfinished last line and closes the connection (SPEC 8.1)"
    );
    let mut c = server.client();
    assert_eq!(
        c.keys(None),
        ["a"],
        "`SET b 2` had no line feed and must not be executed"
    );
}

#[test]
fn m1_quit_says_bye_and_closes_only_this_connection() {
    let dir = TestDir::new("m1_quit");
    let server = Server::start(dir.path());
    let mut c = server.client();
    c.set("stay", "here");
    expect(
        &mut c,
        "QUIT",
        "BYE",
        "QUIT is answered with BYE (SPEC 8.11)",
    );
    c.expect_closed("after BYE");
    let mut c = server.client();
    assert_eq!(
        c.get("stay").as_deref(),
        Some(&b"here"[..]),
        "QUIT ends one connection; the server and its data stay (SPEC 8.11)"
    );
}

#[test]
fn m1_requests_behind_quit_are_not_executed() {
    let dir = TestDir::new("m1_after_quit");
    let server = Server::start(dir.path());
    let mut c = server.client();
    // One piece: the server has the two later lines in hand when it reads
    // `QUIT`.
    c.send_raw(b"SET before 1\nQUIT\nSET after 2\nPING\n");
    assert_eq!(show(&c.recv()), "OK", "the request in front of QUIT");
    assert_eq!(
        show(&c.recv()),
        "BYE",
        "QUIT is answered with BYE (SPEC 8.11)"
    );
    assert_eq!(
        show(&c.read_to_end()),
        "",
        "after BYE the server closes the connection; requests that were sent behind QUIT get \
         no response (SPEC 8.11)"
    );
    let mut c = server.client();
    assert_eq!(
        c.keys(None),
        ["before"],
        "`SET after 2` came after QUIT on the same connection and must not be executed \
         (SPEC 8.11)"
    );
}

#[test]
fn m1_connections_one_after_another_share_the_store() {
    let dir = TestDir::new("m1_sequential");
    let server = Server::start(dir.path());
    for i in 0..20 {
        let mut c = server.client();
        c.set(format!("k{i}"), format!("v{i}"));
        assert_eq!(c.stats().keys, i + 1, "all connections see the same store");
        // Dropping the client closes the connection without QUIT.
    }
    let mut c = server.client();
    assert_eq!(c.get("k7").as_deref(), Some(&b"v7"[..]));
}

#[test]
fn m1_shutdown_answers_ok_and_exits_with_code_0() {
    let dir = TestDir::new("m1_shutdown");
    let server = Server::start(dir.path());
    let mut c = server.client();
    c.set("a", "1");
    expect(
        &mut c,
        "SHUTDOWN",
        "OK",
        "SHUTDOWN is answered with OK (SPEC 8.12)",
    );
    c.expect_closed("after the OK for SHUTDOWN");
    server.expect_clean_exit();
}

#[test]
fn m1_usage_errors_exit_with_code_2() {
    let dir = TestDir::new("m1_usage");
    let data = dir.path().join("never-created");
    let data = data.to_str().expect("the test directory is valid UTF-8");
    let cases: Vec<(Vec<&str>, &str)> = vec![
        (vec![], "no arguments: --dir is required"),
        (vec!["--port", "0"], "--dir is required"),
        (vec!["--dir"], "--dir without a value"),
        (vec!["--dir", data, "--port"], "--port without a value"),
        (
            vec!["--dir", data, "--port", "http"],
            "--port needs a number",
        ),
        (vec!["--dir", data, "--port", "65536"], "--port above 65535"),
        (vec!["--dir", data, "--port", "-1"], "a negative port"),
        (
            vec!["--dir", data, "--port", "+0"],
            "a number is digits only: no plus sign",
        ),
        (vec!["--dir", data, "--port", ""], "an empty value"),
        (
            vec!["--dir", data, "--port", "0", "--auto-compact", "+100"],
            "a number is digits only: no plus sign",
        ),
        (
            vec!["--dir", data, "--port", "0", "--auto-compact", "1e6"],
            "a number is digits only",
        ),
        (
            vec![
                "--dir",
                data,
                "--port",
                "0",
                "--auto-compact",
                "18446744073709551616",
            ],
            "--auto-compact above 2^64 - 1",
        ),
        (
            vec!["--dir", data, "--port", "0", "--max-clients", "+4"],
            "a number is digits only: no plus sign",
        ),
        (
            vec!["--dir", data, "--port", "0", "--fsync", "Always"],
            "--fsync takes always or never, in lower case",
        ),
        (
            vec!["--dir", data, "--port", "0", "--fsync"],
            "--fsync without a value",
        ),
        (vec!["--dir", data, "--help"], "--help is not an option"),
        (
            vec!["--dir", data, "--port", "0", "--fsync", "sometimes"],
            "--fsync takes always or never",
        ),
        (
            vec!["--dir", data, "--port", "0", "--auto-compact", "lots"],
            "--auto-compact needs a number",
        ),
        (
            vec!["--dir", data, "--port", "0", "--max-clients", "0"],
            "--max-clients must be at least 1",
        ),
        (
            vec!["--dir", data, "--port", "0", "--max-clients", "many"],
            "--max-clients needs a number",
        ),
        (
            vec!["--dir", data, "--port", "0", "--verbose"],
            "an option that does not exist",
        ),
        (
            vec!["--dir", data, "--port", "0", "extra"],
            "an argument that is not an option",
        ),
        (
            vec!["--dir", data, "--dir", data, "--port", "0"],
            "the same option twice",
        ),
        (
            vec!["--dir", data, "--port", "0", "--port", "0"],
            "the same option twice",
        ),
    ];
    for (args, why) in cases {
        let args: Vec<OsString> = os_args(&args);
        expect_refusal(&args, 2, &format!("usage error ({why}, SPEC 3.2)"));
        assert!(
            !std::path::Path::new(data).exists(),
            "a usage error ({why}) must be found before the data directory is created (SPEC 3.2)"
        );
    }
}
