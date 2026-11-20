//! Milestone 3: crash recovery. The tests damage the end of the log the way
//! a crash in the middle of a write would, and in ways only a bad disk
//! would, and then start the server on it.
//!
//! The rule of SPEC 6 is short: the valid records from the start of the
//! file up to the first one that is not valid are the store; everything
//! behind them is cut off before the server announces itself.
//!
//! ```text
//! cargo test --test m3_recovery
//! ```

// Miri cannot start other programs or open sockets.
#![cfg(not(miri))]

mod common;

use common::{
    Client, KIND_DEL, KIND_SET, MAGIC, Model, Rng, Server, TestDir, del_record, log_of, model,
    parse_log, raw_record, set_record, show,
};

/// Three records. The tests damage the third.
fn three_records() -> (Vec<u8>, usize) {
    let log = log_of(&[
        set_record("one", "1"),
        set_record("two", "22"),
        set_record("three", "333"),
    ]);
    let third_starts = log.len() - set_record("three", "333").len();
    (log, third_starts)
}

fn first_two() -> Model {
    model(&[("one", "1"), ("two", "22")])
}

/// Starts a server on `log` and checks what SPEC 6 promises: the content is
/// `expected`, and the file has been cut to `valid_len` bytes before the
/// LISTENING line.
#[track_caller]
fn recover(dir: &TestDir, log: &[u8], valid_len: usize, expected: &Model, what: &str) -> Server {
    dir.write_log(log);
    let server = Server::start(dir.path());
    assert_eq!(
        dir.log_len(),
        valid_len as u64,
        "{what}: when the server announces itself, kv.log must already be cut back to its valid \
         prefix of {valid_len} bytes (SPEC 6); the file given to it had {} bytes",
        log.len()
    );
    assert!(
        dir.read_log() == log[..valid_len],
        "{what}: recovery must not change the valid prefix; kv.log now starts with `{}`",
        show(&dir.read_log())
    );
    let mut c = server.client();
    assert_eq!(
        c.dump(),
        *expected,
        "{what}: the store must hold exactly the valid prefix"
    );
    let stats = c.stats();
    assert_eq!(
        (stats.keys, stats.log_bytes),
        (expected.len() as u64, valid_len as u64),
        "{what}: STATS (keys, log_bytes) after recovery"
    );
    server
}

#[test]
fn m3_a_record_cut_off_at_any_byte_is_dropped() {
    let dir = TestDir::new("m3_truncated");
    let (log, third_starts) = three_records();
    for cut in third_starts..log.len() {
        let what = format!(
            "log cut after {} of the {} bytes of its last record",
            cut - third_starts,
            log.len() - third_starts
        );
        recover(&dir, &log[..cut], third_starts, &first_two(), &what).kill();
    }
}

#[test]
fn m3_a_header_cut_off_at_any_byte_is_an_empty_log() {
    let dir = TestDir::new("m3_torn_header");
    for cut in 0..MAGIC.len() {
        dir.write_log(&MAGIC[..cut]);
        let server = Server::start(dir.path());
        assert_eq!(
            show(&dir.read_log()),
            "KVSTORE1",
            "a kv.log that is only the first {cut} magic bytes is a log whose creation was \
             interrupted: it is completed to an empty log (SPEC 6)"
        );
        let mut c = server.client();
        assert_eq!(c.stats().keys, 0);
        c.set("works", "yes");
        server.kill();
        assert_eq!(
            dir.read_log(),
            log_of(&[set_record("works", "yes")]),
            "writes after a completed header"
        );
    }
}

#[test]
fn m3_a_flipped_bit_anywhere_in_the_last_record_drops_it() {
    let dir = TestDir::new("m3_bitflip_tail");
    let (log, third_starts) = three_records();
    for at in third_starts..log.len() {
        // A different bit in each byte.
        let mut damaged = log.clone();
        damaged[at] ^= 1 << (at % 8);
        let field = match at - third_starts {
            0..=3 => "crc",
            4 => "kind",
            5..=6 => "klen",
            7..=10 => "vlen",
            11..=15 => "key",
            _ => "value",
        };
        let what = format!(
            "one bit flipped in the {field} field (byte {}) of the last record",
            at - third_starts
        );
        recover(&dir, &damaged, third_starts, &first_two(), &what).kill();
    }
}

#[test]
fn m3_damage_in_the_middle_keeps_only_what_is_before_it() {
    let dir = TestDir::new("m3_middle");
    let records = [
        set_record("a", "1"),
        set_record("b", "2"),
        set_record("c", "3"),
        del_record("a"),
        set_record("d", "4"),
    ];
    let log = log_of(&records);
    let c_starts = MAGIC.len() + records[0].len() + records[1].len();
    let mut damaged = log.clone();
    damaged[c_starts + 12] ^= 0x20; // the value byte of record c
    recover(
        &dir,
        &damaged,
        c_starts,
        &model(&[("a", "1"), ("b", "2")]),
        "third of five records damaged: the two valid records behind it are lost too, and the \
         `DEL a` among them is not applied",
    );
}

#[test]
fn m3_records_that_were_cut_off_never_come_back() {
    let dir = TestDir::new("m3_no_resurrection");
    let records = [
        set_record("a", "1"),
        set_record("b", "2"),
        set_record("c", "3"),
        set_record("d", "4"),
    ];
    let log = log_of(&records);
    let b_starts = MAGIC.len() + records[0].len();
    let mut damaged = log.clone();
    damaged[b_starts] ^= 0xFF; // first crc byte of record b
    let server = recover(
        &dir,
        &damaged,
        b_starts,
        &model(&[("a", "1")]),
        "second of four records damaged",
    );

    // A new record of the same size as the damaged one. If recovery did not
    // really shorten the file, the old records c and d are now directly
    // behind a valid record again.
    let mut c = server.client();
    c.set("z", "9");
    server.kill();
    let expected = model(&[("a", "1"), ("z", "9")]);
    let server = recover(
        &dir,
        &dir.read_log(),
        b_starts + set_record("z", "9").len(),
        &expected,
        "restart after writing behind a cut",
    );
    assert_eq!(
        server.client().get("c"),
        None,
        "record c was cut off by the first recovery and must not reappear"
    );
}

#[test]
fn m3_records_no_server_would_write_are_not_valid() {
    let dir = TestDir::new("m3_forged");
    let good = [set_record("keep", "me")];
    let prefix = log_of(&good);
    let expected = model(&[("keep", "me")]);
    // Each tail has a correct checksum (where it has a complete record at
    // all). The checksum is necessary, not sufficient (SPEC 5.4).
    let tails: Vec<(&str, Vec<u8>)> = vec![
        ("kind 0", raw_record(0, 1, 1, b"kv")),
        ("kind 3", raw_record(3, 1, 1, b"kv")),
        ("a key length of 0", raw_record(KIND_SET, 0, 1, b"v")),
        (
            "a key length of 129",
            raw_record(KIND_SET, 129, 1, &[b'k'; 130]),
        ),
        (
            "a value length of 4097",
            raw_record(KIND_SET, 1, 4097, &[b'v'; 4098]),
        ),
        (
            "a DEL record with a value",
            raw_record(KIND_DEL, 1, 1, b"kv"),
        ),
        ("a space in the key", raw_record(KIND_SET, 3, 1, b"a bv")),
        (
            "a byte above 0x7E in the key",
            raw_record(KIND_SET, 2, 1, b"k\xe9v"),
        ),
        (
            "a line feed in the value",
            raw_record(KIND_SET, 1, 3, b"kv\nw"),
        ),
        // Length fields that point far past the end of the file. A server
        // that believes them tries to allocate or read 4 GiB.
        (
            "a value length of 4294967295",
            raw_record(KIND_SET, 1, u32::MAX, b"k"),
        ),
        (
            "a key length of 65535",
            raw_record(KIND_SET, u16::MAX, 0, b"k"),
        ),
        (
            "4096 zero bytes, as a file system may leave them",
            vec![0; 4096],
        ),
        ("10 bytes, one less than a record header", vec![0x55; 10]),
    ];
    for (what, tail) in tails {
        let mut log = prefix.clone();
        log.extend_from_slice(&tail);
        // A valid record behind the bad one must not be reached.
        log.extend_from_slice(&set_record("late", "x"));
        recover(
            &dir,
            &log,
            prefix.len(),
            &expected,
            &format!("a record with {what}"),
        )
        .kill();
    }
}

#[test]
fn m3_a_recovered_log_takes_new_writes_and_recovers_again() {
    let dir = TestDir::new("m3_write_after");
    let (log, third_starts) = three_records();
    let server = recover(
        &dir,
        &log[..log.len() - 1],
        third_starts,
        &first_two(),
        "last record one byte short",
    );
    let mut c = server.client();
    c.set("three", "again");
    c.set("four", "4444");
    assert!(c.del("one"));
    server.kill();

    let mut expected_log = log[..third_starts].to_vec();
    expected_log.extend_from_slice(&set_record("three", "again"));
    expected_log.extend_from_slice(&set_record("four", "4444"));
    expected_log.extend_from_slice(&del_record("one"));
    assert_eq!(
        dir.read_log(),
        expected_log,
        "after recovery, new records start exactly where the valid prefix ended"
    );
    recover(
        &dir,
        &expected_log,
        expected_log.len(),
        &model(&[("two", "22"), ("three", "again"), ("four", "4444")]),
        "second start, intact log",
    );
}

#[test]
fn m3_a_large_log_recovers_in_time() {
    const RECORDS: usize = 60_000;
    const KEYS: usize = 20_000;
    let dir = TestDir::new("m3_large");
    let mut log = MAGIC.to_vec();
    let mut expected_last = Model::new();
    for i in 0..RECORDS {
        let key = format!("user:{:05}", i % KEYS);
        let value = format!("visit {i}");
        log.extend_from_slice(&set_record(&key, &value));
        if i >= RECORDS - KEYS {
            expected_last.insert(key.into_bytes(), value.into_bytes());
        }
    }
    let valid = log.len();
    log.extend_from_slice(&set_record("torn", "tail")[..9]);
    dir.write_log(&log);

    // `Server::start` waits for the LISTENING line for the usual time
    // limit. Recovery that reads the file once is far inside it; recovery
    // that does one system call per byte, or re-reads the file per record,
    // is not.
    let server = Server::start(dir.path());
    assert_eq!(
        dir.log_len(),
        valid as u64,
        "the torn tail of a large log is cut off"
    );
    let mut c = server.client();
    assert_eq!(c.stats().keys, KEYS as u64, "60000 records for 20000 keys");
    for i in [0usize, 1, 9_999, 19_999] {
        let key = format!("user:{i:05}");
        assert_eq!(
            c.get(&key).as_deref(),
            expected_last.get(key.as_bytes()).map(Vec::as_slice),
            "the newest value of {key}"
        );
    }
    assert_eq!(
        c.keys(Some("user:1999")).len(),
        10,
        "KEYS by prefix on a large store"
    );
}

/// One step of the random history in `m3_random_crashes_match_a_model`.
#[derive(Debug, Clone)]
enum Step {
    Set(String, String),
    Del(String),
    Incr(String),
}

/// Applies `step` to the model the way SPEC 8 says the server does, and
/// returns how many bytes it appends to the log.
fn apply(content: &mut Model, step: &Step) -> usize {
    match step {
        Step::Set(key, value) => {
            content.insert(key.clone().into_bytes(), value.clone().into_bytes());
            set_record(key, value).len()
        }
        Step::Del(key) => match content.remove(key.as_bytes()) {
            Some(_) => del_record(key).len(),
            None => 0,
        },
        Step::Incr(key) => {
            let current = match content.get(key.as_bytes()) {
                None => Some(0),
                Some(bytes) => std::str::from_utf8(bytes)
                    .ok()
                    .and_then(|t| t.parse::<i64>().ok()),
            };
            match current.and_then(|n| n.checked_add(1)) {
                Some(next) => {
                    let text = next.to_string();
                    content.insert(key.clone().into_bytes(), text.clone().into_bytes());
                    set_record(key, &text).len()
                }
                None => 0,
            }
        }
    }
}

fn send(c: &mut Client, step: &Step) {
    match step {
        Step::Set(key, value) => c.set(key, value),
        Step::Del(key) => {
            c.del(key);
        }
        Step::Incr(key) => {
            c.req(format!("INCR {key}"));
        }
    }
}

#[test]
fn m3_random_crashes_match_a_model() {
    const ROUNDS: usize = 12;
    const STEPS: usize = 40;
    let dir = TestDir::new("m3_random");
    let mut rng = Rng::new(0x6b76_7374_6f72_6531);
    // What is certainly in the log, and how long the log is.
    let mut content = Model::new();
    let mut log_len = MAGIC.len();

    for round in 0..ROUNDS {
        let server = Server::start(dir.path());
        let mut c = server.client();
        assert_eq!(
            dir.log_len(),
            log_len as u64,
            "round {round}: size of kv.log after recovery"
        );
        assert_eq!(c.dump(), content, "round {round}: content after recovery");

        // A history of acknowledged steps, with the log size after each.
        let mut history: Vec<(Step, usize)> = Vec::new();
        let mut after = content.clone();
        let mut end = log_len;
        for _ in 0..STEPS {
            let key = format!("k{}", rng.below(12));
            let step = match rng.below(10) {
                0..=5 => Step::Set(
                    key,
                    "v".repeat(rng.below(30) as usize) + &rng.below(1000).to_string(),
                ),
                6..=7 => Step::Del(key),
                _ => Step::Incr(format!("n{}", rng.below(3))),
            };
            send(&mut c, &step);
            end += apply(&mut after, &step);
            history.push((step, end));
        }
        assert_eq!(
            dir.log_len(),
            end as u64,
            "round {round}: size of kv.log after {STEPS} steps"
        );
        server.kill();

        // Damage somewhere inside what this round wrote: cut the file
        // there, or flip a bit there, or (one round in four) do nothing.
        let mut log = dir.read_log();
        let at = log_len + rng.below((end - log_len).max(1) as u64) as usize;
        match rng.below(4) {
            0 => {}
            1 | 2 => log.truncate(at),
            _ => log[at] ^= 1 << rng.below(8),
        }
        let damaged_at = if log.len() == end && log == dir.read_log() {
            end
        } else {
            at
        };
        dir.write_log(&log);

        // The model: the steps whose record ends at or before the damage.
        let mut survived = content.clone();
        let mut survived_len = log_len;
        for (step, step_end) in &history {
            if *step_end > damaged_at {
                break;
            }
            apply(&mut survived, step);
            survived_len = *step_end;
        }
        // The independent parser must agree with the numbers kept above.
        let parsed = parse_log(&log);
        assert_eq!(
            parsed.valid_len, survived_len as u64,
            "round {round}: the numbers kept by the test"
        );
        assert_eq!(
            parsed.replay(),
            survived,
            "round {round}: the numbers kept by the test"
        );
        content = survived;
        log_len = survived_len;
    }

    let server = Server::start(dir.path());
    assert_eq!(
        dir.log_len(),
        log_len as u64,
        "size of kv.log after the last recovery"
    );
    assert_eq!(
        server.client().dump(),
        content,
        "content after the last recovery"
    );
}
