//! Acceptance tests for the capstone. They are the contract in executable form.
//! Run: cargo test --manifest-path mastery/m11_services/Cargo.toml -p beacon
//!
//! Each test starts `beacon-server`, `beacon-agent`, or both, as child processes
//! on loopback ports chosen by the operating system, and talks to them only from
//! the outside. Every child is killed when its test ends, also when the test
//! fails. Until the binaries are implemented, they stop at `todo!()` and each
//! test reports what the program printed.
//!
//! Suggested order: the tests are named s1.. (server), a1.. (agent), e1 (both).

mod support;

use beacon_wire::{Message, Sample};
use serde_json::{Value, json};
use std::process::Command;
use std::time::Duration;
use support::{ChildGuard, Event, FakeIngest, Peer, TempDb, http};

const ADMIN: &str = "admin";
const PASSWORD: &str = "correct horse battery staple";
const TOKEN_KEY: &str = "acceptance-test-key-0123456789abcdef-0123456789abcdef";

/// A running `beacon-server` and the two addresses it printed.
struct Server {
    child: ChildGuard,
    ingest: String,
    http: String,
}

fn server_command(db_url: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_beacon-server"));
    command
        .env("BEACON_INGEST_ADDR", "127.0.0.1:0")
        .env("BEACON_HTTP_ADDR", "127.0.0.1:0")
        .env("BEACON_DB", db_url)
        .env("BEACON_ADMIN_USER", ADMIN)
        .env("BEACON_ADMIN_PASSWORD", PASSWORD)
        .env("BEACON_TOKEN_KEY", TOKEN_KEY);
    command
}

#[track_caller]
fn start_server(db: &TempDb) -> Server {
    let mut child = ChildGuard::spawn("beacon-server", &mut server_command(&db.url()));
    let first = child.line(60);
    let second = child.line(60);
    let ingest = first
        .strip_prefix("ingest=")
        .unwrap_or_else(|| panic!("the first line must be `ingest=ADDR`, got {first:?}"))
        .to_owned();
    let http = second
        .strip_prefix("http=")
        .unwrap_or_else(|| panic!("the second line must be `http=ADDR`, got {second:?}"))
        .to_owned();
    Server {
        child,
        ingest,
        http,
    }
}

#[track_caller]
fn login(server: &Server) -> String {
    let body = json!({ "user": ADMIN, "password": PASSWORD }).to_string();
    let reply = http(&server.http, "POST", "/v1/login", &[], Some(&body));
    assert_eq!(reply.status, 200, "login failed: {}", reply.body);
    reply.json()["token"]
        .as_str()
        .expect("the login response has a string `token`")
        .to_owned()
}

/// `GET` with a bearer token.
#[track_caller]
fn get_as(server: &Server, token: &str, path: &str) -> support::Reply {
    let bearer = format!("Bearer {token}");
    http(
        &server.http,
        "GET",
        path,
        &[("authorization", &bearer)],
        None,
    )
}

/// All samples of a series, read page by page.
#[track_caller]
fn all_samples(
    server: &Server,
    token: &str,
    agent: &str,
    metric: &str,
    page: u32,
) -> Vec<(u64, i64)> {
    let mut collected = Vec::new();
    let mut after: Option<u64> = None;
    for _ in 0..1000 {
        let mut path = format!("/v1/agents/{agent}/metrics/{metric}/samples?limit={page}");
        if let Some(after) = after {
            path.push_str(&format!("&after={after}"));
        }
        let reply = get_as(server, token, &path);
        assert_eq!(reply.status, 200, "{path}: {}", reply.body);
        let body = reply.json();
        let samples = body["samples"].as_array().expect("`samples` is an array");
        assert!(
            samples.len() <= page as usize,
            "a page has at most `limit` samples: {path}"
        );
        for sample in samples {
            collected.push((
                sample["ts_ms"].as_u64().expect("`ts_ms` is a number"),
                sample["value"].as_i64().expect("`value` is a number"),
            ));
        }
        match body["next_after"].as_u64() {
            Some(next) => after = Some(next),
            None => {
                assert!(
                    body["next_after"].is_null(),
                    "`next_after` is a number or null"
                );
                return collected;
            }
        }
    }
    panic!("paging did not end after 1000 pages");
}

fn agent_command(
    server: &str,
    id: &str,
    count: u32,
    interval_ms: u32,
    buffer: u32,
    batch: u32,
) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_beacon-agent"));
    command
        .env("BEACON_SERVER", server)
        .env("BEACON_AGENT_ID", id)
        .env("BEACON_METRIC", "ticks")
        .env("BEACON_SOURCE", "counter")
        .env("BEACON_COUNT", count.to_string())
        .env("BEACON_INTERVAL_MS", interval_ms.to_string())
        .env("BEACON_BUFFER", buffer.to_string())
        .env("BEACON_BATCH", batch.to_string());
    command
}

fn hello(agent: &str) -> Message {
    Message::Hello {
        agent: agent.to_owned(),
    }
}

fn batch(seq: u64, metric: &str, samples: &[(u64, i64)]) -> Message {
    Message::Batch {
        seq,
        metric: metric.to_owned(),
        samples: samples
            .iter()
            .map(|&(ts_ms, value)| Sample { ts_ms, value })
            .collect(),
    }
}

const WAIT: Duration = Duration::from_secs(20);

// ---------------------------------------------------------------------------
// The server
// ---------------------------------------------------------------------------

#[test]
fn s1_server_refuses_to_start_without_its_secrets() {
    let db = TempDb::new("s1");
    for missing in [
        "BEACON_ADMIN_PASSWORD",
        "BEACON_TOKEN_KEY",
        "BEACON_ADMIN_USER",
    ] {
        let mut command = server_command(&db.url());
        command.env_remove(missing);
        let mut child = ChildGuard::spawn("beacon-server", &mut command);
        assert_eq!(
            child.wait(60),
            Some(78),
            "without {missing} the server must exit with code 78 (configuration \
             error) instead of running with a default. stderr: {}",
            child.stderr()
        );
        let stderr = child.stderr();
        assert!(
            stderr.contains(missing),
            "the message must name the variable {missing}, got: {stderr}"
        );
        assert!(
            !stderr.contains(PASSWORD) && !stderr.contains(TOKEN_KEY),
            "a configuration error must not print secrets: {stderr}"
        );
    }
}

#[test]
fn s2_health_login_and_protected_query() {
    let db = TempDb::new("s2");
    let server = start_server(&db);

    let health = http(&server.http, "GET", "/healthz", &[], None);
    assert_eq!(health.status, 200);

    let series = "/v1/agents/pump-7/metrics/ticks/samples";
    let wrong_password = json!({ "user": ADMIN, "password": "wrong" }).to_string();
    let unknown_user = json!({ "user": "mallory", "password": PASSWORD }).to_string();
    let first = http(
        &server.http,
        "POST",
        "/v1/login",
        &[],
        Some(&wrong_password),
    );
    let second = http(&server.http, "POST", "/v1/login", &[], Some(&unknown_user));
    assert_eq!(first.status, 401);
    assert_eq!(first.json(), json!({ "error": "invalid_credentials" }));
    assert_eq!(
        (second.status, &second.body),
        (first.status, &first.body),
        "an unknown user and a wrong password must look the same"
    );

    let no_token = http(&server.http, "GET", series, &[], None);
    assert_eq!(no_token.status, 401);
    assert_eq!(no_token.json(), json!({ "error": "unauthorized" }));
    let forged = get_as(&server, "v1.61646d696e.9999999999.00", series);
    assert_eq!(forged.status, 401);
    assert_eq!(forged.json(), json!({ "error": "unauthorized" }));

    let token = login(&server);
    let empty = get_as(&server, &token, series);
    assert_eq!(empty.status, 200, "{}", empty.body);
    assert_eq!(
        empty.json(),
        json!({ "samples": [], "next_after": null }),
        "a series nobody has written yet is empty, not an error"
    );

    let lost = get_as(&server, &token, "/v1/nothing/here");
    assert_eq!(lost.status, 404);
    assert_eq!(lost.json(), json!({ "error": "not_found" }));
    for bad in ["?limit=0", "?limit=5000", "?limit=x", "?after=yesterday"] {
        let reply = get_as(&server, &token, &format!("{series}{bad}"));
        assert_eq!(reply.status, 400, "{bad}: {}", reply.body);
        assert_eq!(reply.json(), json!({ "error": "invalid_query" }), "{bad}");
    }
}

#[test]
fn s3_ingest_stores_before_it_acknowledges_and_ignores_duplicates() {
    let db = TempDb::new("s3");
    let mut server = start_server(&db);
    let samples = [
        (1_800_000_000_000, 5),
        (1_800_000_000_010, 6),
        (1_800_000_000_020, -7),
    ];

    let mut agent = Peer::connect(&server.ingest);
    assert!(agent.send(&hello("pump-7")));
    assert!(agent.send(&batch(1, "pressure", &samples)));
    assert_eq!(agent.recv(WAIT), Event::Message(Message::Ack { seq: 1 }));

    // The moment the Ack is here, the power fails.
    server.child.kill();
    drop(agent);

    let server = start_server(&db);
    let token = login(&server);
    assert_eq!(
        all_samples(&server, &token, "pump-7", "pressure", 100),
        samples,
        "an acknowledged batch must survive a crash: commit first, then send the Ack"
    );

    // Suppose the Ack had been lost on the way. The agent then sends the batch
    // again on a new connection, and later a batch that overlaps with it.
    let mut agent = Peer::connect(&server.ingest);
    assert!(agent.send(&hello("pump-7")));
    assert!(agent.send(&batch(1, "pressure", &samples)));
    assert_eq!(agent.recv(WAIT), Event::Message(Message::Ack { seq: 1 }));
    let more = [samples[2], (1_800_000_000_030, 8)];
    assert!(agent.send(&batch(2, "pressure", &more)));
    assert_eq!(agent.recv(WAIT), Event::Message(Message::Ack { seq: 2 }));
    // Another metric is a separate series, even with the same times.
    assert!(agent.send(&batch(3, "flow", &samples[..1])));
    assert_eq!(agent.recv(WAIT), Event::Message(Message::Ack { seq: 3 }));

    let stored = all_samples(&server, &token, "pump-7", "pressure", 2);
    assert_eq!(
        stored,
        [samples[0], samples[1], samples[2], (1_800_000_000_030, 8)],
        "a sample that arrives twice is stored once (same agent, metric and time)"
    );
    assert_eq!(
        all_samples(&server, &token, "pump-7", "flow", 100),
        [samples[0]]
    );
    assert!(all_samples(&server, &token, "pump-8", "pressure", 100).is_empty());
}

#[test]
fn s4_server_survives_clients_that_break_the_protocol() {
    let db = TempDb::new("s4");
    let server = start_server(&db);

    // Something that is not our protocol at all.
    let mut stranger = Peer::connect(&server.ingest);
    assert!(stranger.send_raw(b"GET / HTTP/1.1\r\nhost: example\r\n\r\n"));
    assert_eq!(
        stranger.recv(WAIT),
        Event::Closed,
        "the server must close a connection that does not speak the protocol"
    );

    // The right protocol in the wrong order: a batch before Hello.
    let mut rude = Peer::connect(&server.ingest);
    assert!(rude.send(&batch(1, "pressure", &[(1, 1)])));
    assert_eq!(
        rude.recv(WAIT),
        Event::Closed,
        "a batch before Hello is not acknowledged"
    );

    // A header that announces a huge payload, then silence.
    let mut greedy = Peer::connect(&server.ingest);
    assert!(greedy.send_raw(&[b'B', b'W', 1, 2, 0xff, 0xff, 0xff, 0xf0]));
    assert_eq!(
        greedy.recv(WAIT),
        Event::Closed,
        "an oversized frame is refused at once"
    );

    // A client that connects and just goes away.
    drop(Peer::connect(&server.ingest));

    // After all that, the server is alive and correct.
    assert_eq!(http(&server.http, "GET", "/healthz", &[], None).status, 200);
    let mut agent = Peer::connect(&server.ingest);
    assert!(agent.send(&hello("pump-9")));
    assert!(agent.send(&batch(1, "pressure", &[(10, 1), (20, 2)])));
    assert_eq!(agent.recv(WAIT), Event::Message(Message::Ack { seq: 1 }));
    let token = login(&server);
    assert_eq!(
        all_samples(&server, &token, "pump-9", "pressure", 100),
        [(10, 1), (20, 2)],
        "nothing from the broken connections was stored"
    );
}

// ---------------------------------------------------------------------------
// The agent
// ---------------------------------------------------------------------------

#[test]
fn a1_agent_reports_bad_configuration() {
    let mut command = Command::new(env!("CARGO_BIN_EXE_beacon-agent"));
    command.env("BEACON_AGENT_ID", "pump-7");
    let mut agent = ChildGuard::spawn("beacon-agent", &mut command);
    assert_eq!(
        agent.wait(60),
        Some(78),
        "without BEACON_SERVER the agent exits with code 78. stderr: {}",
        agent.stderr()
    );
    assert!(
        agent.stderr().contains("BEACON_SERVER"),
        "{}",
        agent.stderr()
    );
}

#[test]
fn a2_agent_ships_everything_to_a_healthy_server() {
    let ingest = FakeIngest::start(true, false);
    let mut agent = ChildGuard::spawn(
        "beacon-agent",
        &mut agent_command(&ingest.addr, "pump-7", 25, 2, 1000, 6),
    );
    assert_eq!(agent.line(60), "sampled=25", "stderr: {}", agent.stderr());
    assert_eq!(
        agent.line(60),
        "shipped=25 dropped=0",
        "stderr: {}",
        agent.stderr()
    );
    assert_eq!(agent.wait(60), Some(0), "stderr: {}", agent.stderr());

    let logs = ingest.finish();
    assert_eq!(logs.len(), 1, "one healthy connection is enough: {logs:?}");
    assert_eq!(
        logs[0].agent.as_deref(),
        Some("pump-7"),
        "Hello carries the agent id"
    );
    assert_eq!(
        logs[0].acked,
        (0..25).collect::<Vec<i64>>(),
        "the counter source produces 0, 1, 2, ... and they arrive in order"
    );
    assert!(
        logs[0].times.windows(2).all(|pair| pair[0] < pair[1]),
        "sample times must increase strictly: the server identifies a sample by its time"
    );
    assert!(
        logs[0].times[0] > 1_600_000_000_000,
        "times are milliseconds since 1970"
    );
}

#[test]
fn a3_agent_resends_what_was_not_acknowledged() {
    // The first connection is cut right after its first batch, before any Ack.
    let ingest = FakeIngest::start(true, true);
    let mut agent = ChildGuard::spawn(
        "beacon-agent",
        &mut agent_command(&ingest.addr, "pump-7", 20, 2, 1000, 5),
    );
    assert_eq!(agent.line(60), "sampled=20", "stderr: {}", agent.stderr());
    assert_eq!(
        agent.line(60),
        "shipped=20 dropped=0",
        "the agent must reconnect and deliver everything. stderr: {}",
        agent.stderr()
    );
    assert_eq!(agent.wait(60), Some(0), "stderr: {}", agent.stderr());

    let logs = ingest.finish();
    assert!(logs.len() >= 2, "the agent must have reconnected: {logs:?}");
    let lost = &logs[0].unacked;
    assert!(
        !lost.is_empty(),
        "the first connection carried one batch: {logs:?}"
    );
    let mut delivered: Vec<i64> = logs.iter().flat_map(|log| log.acked.clone()).collect();
    delivered.sort_unstable();
    delivered.dedup();
    assert_eq!(
        delivered,
        (0..20).collect::<Vec<i64>>(),
        "every sample must be acknowledged at some point, including the batch \
         {lost:?} that was sent on the connection that broke"
    );
    assert!(
        logs.iter()
            .skip(1)
            .all(|log| log.agent.as_deref() == Some("pump-7")),
        "every new connection starts with Hello: {logs:?}"
    );
}

#[test]
fn a4_agent_buffer_is_bounded_and_keeps_the_newest_samples() {
    // The "server" accepts connections and reads, but acknowledges nothing.
    let ingest = FakeIngest::start(false, false);
    let mut agent = ChildGuard::spawn(
        "beacon-agent",
        &mut agent_command(&ingest.addr, "pump-7", 60, 1, 8, 4),
    );
    // Sampling must go on although nothing is being acknowledged.
    assert_eq!(
        agent.line(60),
        "sampled=60",
        "the sampler must not wait for the network. stderr: {}",
        agent.stderr()
    );

    // Now the server recovers.
    ingest.start_acking();
    assert_eq!(
        agent.line(60),
        "shipped=8 dropped=52",
        "with BEACON_BUFFER=8 the agent keeps the 8 newest unacknowledged \
         samples and counts the rest as dropped. stderr: {}",
        agent.stderr()
    );
    assert_eq!(agent.wait(60), Some(0), "stderr: {}", agent.stderr());

    let logs = ingest.finish();
    let last = logs
        .iter()
        .rev()
        .find(|log| !log.acked.is_empty())
        .unwrap_or_else(|| panic!("no batch was acknowledged: {logs:?}"));
    assert_eq!(
        last.acked,
        (52..60).collect::<Vec<i64>>(),
        "after the outage the agent delivers exactly the newest 8 samples, oldest first"
    );
}

// ---------------------------------------------------------------------------
// Both together
// ---------------------------------------------------------------------------

#[test]
fn e1_samples_travel_from_the_agent_to_the_api() {
    let db = TempDb::new("e1");
    let server = start_server(&db);
    let mut agent = ChildGuard::spawn(
        "beacon-agent",
        &mut agent_command(&server.ingest, "boiler-2", 30, 1, 1000, 7),
    );
    assert_eq!(agent.line(60), "sampled=30", "stderr: {}", agent.stderr());
    assert_eq!(
        agent.line(60),
        "shipped=30 dropped=0",
        "stderr: {}",
        agent.stderr()
    );
    assert_eq!(agent.wait(60), Some(0), "stderr: {}", agent.stderr());

    let token = login(&server);
    let whole = all_samples(&server, &token, "boiler-2", "ticks", 1000);
    let values: Vec<i64> = whole.iter().map(|&(_, value)| value).collect();
    assert_eq!(
        values,
        (0..30).collect::<Vec<i64>>(),
        "stderr of the server: {}",
        server.child.stderr()
    );
    assert!(
        whole.windows(2).all(|pair| pair[0].0 < pair[1].0),
        "ordered by time"
    );

    // The same data in pages of 7: 7 + 7 + 7 + 7 + 2, nothing skipped or repeated.
    let paged = all_samples(&server, &token, "boiler-2", "ticks", 7);
    assert_eq!(
        paged, whole,
        "paging must give the same samples as one big page"
    );
    let first_page = get_as(
        &server,
        &token,
        "/v1/agents/boiler-2/metrics/ticks/samples?limit=30",
    );
    assert_eq!(
        first_page.json()["next_after"],
        Value::Null,
        "a page that holds everything has no next page"
    );
}
