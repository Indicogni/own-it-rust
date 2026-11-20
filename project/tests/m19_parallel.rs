//! MILESTONE m19 — Many files on worker threads, same results (Chapter 19)
//! Run: cargo test --test m19_parallel   (then re-run: cargo test --test m18_cli)
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: which part of the work can run in any
//! order, and which part must happen in input order?
//! IMPLEMENT `src/parallel.rs`: `map_ordered`, `summarize_files`.
//! REFACTOR `src/cli.rs` `run_with`: with `--jobs` other than 1 and several FILEs,
//! process them with `map_ordered`, opening each file inside the worker closure. The
//! m18 CLI tests and the `c_` tests below must pass.
//! EXPLAIN afterward: why scoped threads can borrow `filter` and `paths`; what would
//! break with `std::thread::spawn`; and why the merge order matters for `grep` but not
//! for `count`.
//!
//! Every test runs its body through `common::within`, on a helper thread with a time
//! limit. A solution that blocks forever (the usual cause: a channel sender that is
//! still alive while you wait on the receiver) fails with a message instead of hanging.

mod common;

#[allow(unused_imports)] // unused once every prediction is answered
use common::guess;
use common::{TempDir, fixture, output_within, within};
use loglens::cli::{Command, Input, Options, run, run_with};
use loglens::parallel::{map_ordered, summarize_files};
use loglens::pipeline::process;
use loglens::{Error, Filter, Group, Summary, Tally};
use std::collections::HashSet;
use std::io::{self, BufRead, Cursor};
use std::path::{Path, PathBuf};
use std::process::Command as Process;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::thread::{self, ThreadId};
use std::time::{Duration, Instant};

/// Seconds a test body may run before `within` fails it. Far more than a
/// correct solution needs, even on a slow, busy machine.
const LIMIT: u64 = 30;

#[test]
fn a_map_ordered_keeps_input_order() {
    within(LIMIT, "map_ordered", || {
        let items: Vec<u64> = (0..100).collect();
        let expected: Vec<u64> = items.iter().map(|x| x * x).collect();
        for jobs in [0, 1, 2, 3, 8, 200] {
            assert_eq!(
                map_ordered(&items, jobs, |x| x * x),
                expected,
                "jobs = {jobs}: results must be in input order, whichever worker finished first"
            );
        }
        assert_eq!(map_ordered(&[] as &[u64], 8, |x| *x), Vec::<u64>::new());
    });
}

#[test]
fn a_map_ordered_borrows_its_inputs() {
    within(LIMIT, "map_ordered", || {
        let names = vec![String::from("auth"), String::from("db")];
        let prefix = String::from("svc-");
        let out = map_ordered(&names, 2, |n| format!("{prefix}{n}"));
        assert_eq!(out, ["svc-auth", "svc-db"]);
        assert_eq!(prefix, "svc-", "still ours");
    });
}

#[test]
fn a_one_worker_spawns_no_thread() {
    within(LIMIT, "map_ordered", || {
        let me = thread::current().id();
        let ids = map_ordered(&[1, 2, 3], 1, |_| thread::current().id());
        assert!(
            ids.iter().all(|&id| id == me),
            "jobs = 1 must run every item on the calling thread, without spawning"
        );
        let ids = map_ordered(&[1], 8, |_| thread::current().id());
        assert_eq!(
            ids,
            [me],
            "one item needs one worker, and one worker means the calling thread"
        );
    });
}

/// Each call waits (up to a deadline) until at least two threads are inside
/// `f` at the same time. A sequential implementation fails after the deadline
/// instead of hanging.
#[test]
fn a_several_workers_run_at_once() {
    within(LIMIT, "map_ordered", || {
        let inside: Mutex<HashSet<ThreadId>> = Mutex::new(HashSet::new());
        let changed = Condvar::new();
        let deadline = Instant::now() + Duration::from_secs(10);
        let items: Vec<u32> = (0..8).collect();
        let ids = map_ordered(&items, 4, |_| {
            let mut set = inside.lock().unwrap();
            set.insert(thread::current().id());
            changed.notify_all();
            while set.len() < 2 && Instant::now() < deadline {
                set = changed
                    .wait_timeout(set, Duration::from_millis(50))
                    .unwrap()
                    .0;
            }
            thread::current().id()
        });
        let distinct: HashSet<ThreadId> = ids.into_iter().collect();
        assert!(
            distinct.len() >= 2,
            "all 8 items ran on one thread: with jobs = 4, several workers must run at once"
        );
        assert!(
            distinct.len() <= 4,
            "{} threads did the work, but jobs = 4 allows at most 4",
            distinct.len()
        );
    });
}

#[test]
fn a_work_is_handed_out_dynamically() {
    within(LIMIT, "map_ordered", || {
        // Item 0 is slow. With 2 workers, the other worker must take the rest
        // while item 0 runs, instead of waiting for a fixed share.
        let done_while_slow = AtomicUsize::new(0);
        let slow_done = AtomicUsize::new(0);
        let items: Vec<u32> = (0..6).collect();
        map_ordered(&items, 2, |&i| {
            if i == 0 {
                let deadline = Instant::now() + Duration::from_secs(10);
                while done_while_slow.load(Ordering::SeqCst) < 5 && Instant::now() < deadline {
                    thread::sleep(Duration::from_millis(5));
                }
                slow_done.store(1, Ordering::SeqCst);
            } else if slow_done.load(Ordering::SeqCst) == 0 {
                done_while_slow.fetch_add(1, Ordering::SeqCst);
            }
        });
        assert_eq!(
            done_while_slow.load(Ordering::SeqCst),
            5,
            "the free worker must finish the other 5 items while item 0 is still running; \
             a fixed split of the items between the workers leaves some of them waiting"
        );
    });
}

#[test]
fn a_panics_propagate() {
    for jobs in [1, 2] {
        // `catch_unwind` runs inside the time limit, so a hang is reported as
        // a hang and not mistaken for the expected panic.
        let (panicked, reached) =
            within(LIMIT, "map_ordered with a panicking closure", move || {
                let reached = AtomicUsize::new(0);
                let outcome = std::panic::catch_unwind(|| {
                    map_ordered(&[1, 2, 3, 4], jobs, |&x| {
                        if x == 3 {
                            reached.fetch_add(1, Ordering::SeqCst);
                            panic!("boom");
                        }
                        x
                    })
                });
                (outcome.is_err(), reached.into_inner())
            });
        assert_eq!(
            reached, 1,
            "jobs = {jobs}: `f` must be called exactly once for item 3"
        );
        assert!(
            panicked,
            "jobs = {jobs}: the panic in `f` must reach the caller of map_ordered, \
             not be swallowed"
        );
    }
}

/// Writes `n` generated log files with varied sizes and some malformed lines.
fn generated_files(dir: &TempDir, n: usize) -> Vec<PathBuf> {
    let targets = ["auth", "db", "cache", "http", "billing"];
    let levels = ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"];
    (0..n)
        .map(|f| {
            let mut text = String::new();
            for i in 0..(50 + f * 37) {
                let k = f * 1000 + i;
                if k % 41 == 0 {
                    text.push_str("this line is malformed\n");
                    continue;
                }
                text.push_str(&format!(
                    "2026-10-23T17:{:02}:{:02}Z {} {} user=u{} latency_ms={}\n",
                    k / 60 % 60,
                    k % 60,
                    levels[k % 5],
                    targets[k * 7 % 5],
                    k % 13,
                    k * 31 % 997
                ));
            }
            dir.file(&format!("gen{f}.log"), text)
        })
        .collect()
}

#[test]
fn b_summarize_files_is_the_same_for_every_jobs() {
    within(LIMIT, "summarize_files", || {
        let dir = TempDir::new("par");
        let mut paths = generated_files(&dir, 9);
        paths.extend(["app.log", "web.log", "worker.log"].map(fixture));
        for group in [Group::Target, Group::Field("user".into())] {
            for filter in ["", "level>=warn", "latency_ms>500"] {
                let filter = Filter::parse(filter).unwrap();
                let sequential = summarize_files(&paths, &filter, &group, 1).unwrap();
                for jobs in [0, 2, 3, 16] {
                    let parallel = summarize_files(&paths, &filter, &group, jobs).unwrap();
                    assert_eq!(
                        parallel, sequential,
                        "jobs = {jobs}: the parallel result must equal the sequential one"
                    );
                }

                // And the same as one pass over all the text.
                let mut whole = Summary::new(group.clone());
                let mut tally = Tally::default();
                for p in &paths {
                    let text = std::fs::read_to_string(p).unwrap();
                    tally += process(&text, &filter, &mut whole, &mut |_| {}).unwrap();
                }
                assert_eq!(sequential, (tally, whole));
            }
        }
    });
}

#[test]
fn b_summarize_files_reports_the_first_missing_file() {
    within(LIMIT, "summarize_files", || {
        let paths = [
            fixture("app.log"),
            fixture("missing-1.log"),
            fixture("web.log"),
            fixture("missing-2.log"),
        ];
        for jobs in [1, 4] {
            match summarize_files(&paths, &Filter::all(), &Group::Target, jobs) {
                Err(Error::Open { path, .. }) => assert_eq!(
                    path, paths[1],
                    "jobs = {jobs}: report the first missing path in path order, \
                     not the one a worker happened to reach first"
                ),
                other => panic!("expected Error::Open, got {other:?}"),
            }
        }
    });
}

fn run_in_memory(options: &Options) -> (u8, String, String) {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let status = run(options, &mut Cursor::new(""), &mut out, &mut err);
    (
        status,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

#[test]
fn c_cli_output_is_identical_for_every_jobs() {
    within(LIMIT, "run", || {
        let dir = TempDir::new("par-cli");
        let mut inputs: Vec<Input> = generated_files(&dir, 6)
            .into_iter()
            .map(Input::Path)
            .collect();
        inputs.push(Input::Path(fixture("worker.log")));
        inputs.push(Input::Path(dir.path().join("missing.log")));
        inputs.push(Input::Path(fixture("app.log")));
        for command in [Command::Grep, Command::Count, Command::Report] {
            let sequential = Options {
                command,
                filter: "level>=info".into(),
                inputs: inputs.clone(),
                ..Options::default()
            };
            let expected = run_in_memory(&sequential);
            assert_eq!(expected.0, 2, "the missing file");
            for jobs in [0, 3, 8] {
                let parallel = Options {
                    jobs,
                    ..sequential.clone()
                };
                assert_eq!(
                    run_in_memory(&parallel),
                    expected,
                    "{command:?} with jobs = {jobs}: status, stdout, and stderr must be \
                     byte-for-byte those of --jobs 1"
                );
            }
        }
    });
}

/// What one `run_with` call over in-memory files showed.
struct Observed {
    /// Status, stdout, stderr.
    result: (u8, String, String),
    /// The threads that called the opener.
    openers: HashSet<ThreadId>,
    /// The thread that called `run_with`.
    caller: ThreadId,
}

/// Runs `grep` over eight in-memory "files" (and one that cannot be opened)
/// through `run_with`, with an opener that records which thread asked.
///
/// With `meet`, each opener call also stays inside the opener until a second
/// thread has called it too, up to a shared deadline. So a `run_with` that
/// reads files on several threads is seen reliably, and one that reads them
/// one after the other fails after the deadline instead of hanging.
fn grep_memory_files(jobs: usize, meet: bool) -> Observed {
    within(LIMIT, "run_with", move || {
        let asked: Mutex<HashSet<ThreadId>> = Mutex::new(HashSet::new());
        let changed = Condvar::new();
        let deadline = Instant::now() + Duration::from_secs(10);
        let open = |path: &Path| -> io::Result<Box<dyn BufRead>> {
            let mut set = asked.lock().unwrap();
            set.insert(thread::current().id());
            changed.notify_all();
            while meet && set.len() < 2 && Instant::now() < deadline {
                set = changed
                    .wait_timeout(set, Duration::from_millis(50))
                    .unwrap()
                    .0;
            }
            drop(set);
            let name = path.file_stem().unwrap().to_str().unwrap();
            if name == "missing" {
                return Err(io::Error::new(io::ErrorKind::NotFound, "no such file"));
            }
            // Each file's text names the file, so the order shows in the output.
            Ok(Box::new(Cursor::new(format!(
                "2026-10-23T17:03:00Z INFO auth file={name}\n\
                 this line of {name} is malformed\n\
                 2026-10-23T17:03:01Z WARN db file={name}\n"
            ))))
        };
        let mut inputs: Vec<Input> = (0..8)
            .map(|i| Input::Path(PathBuf::from(format!("memory/f{i}.log"))))
            .collect();
        inputs.insert(5, Input::Path(PathBuf::from("memory/missing.log")));
        let options = Options {
            command: Command::Grep,
            filter: "level>=info".into(),
            jobs,
            inputs,
            ..Options::default()
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let status = run_with(&options, &open, &mut Cursor::new(""), &mut out, &mut err);
        Observed {
            result: (
                status,
                String::from_utf8(out).unwrap(),
                String::from_utf8(err).unwrap(),
            ),
            openers: asked.into_inner().unwrap(),
            caller: thread::current().id(),
        }
    })
}

/// The technique test for the refactor of `run_with`: equal output alone
/// would also pass for a program that ignores `--jobs`.
#[test]
fn c_cli_jobs_reads_files_on_several_threads() {
    let sequential = grep_memory_files(1, false);
    assert_eq!(
        sequential.openers,
        HashSet::from([sequential.caller]),
        "with --jobs 1, every file must be opened and read on the calling thread"
    );
    assert_eq!(sequential.result.0, 2, "one file could not be opened");
    assert_eq!(
        sequential.result.1.lines().count(),
        16,
        "two matching records from each of the eight readable files"
    );

    let parallel = grep_memory_files(4, true);
    assert!(
        parallel.openers.len() >= 2,
        "with --jobs 4, `run_with` still opened all 9 files on one thread. Process the \
         files with `map_ordered` and call `open` inside the closure it runs, so that \
         each worker opens and reads its own file"
    );
    assert!(
        parallel.openers.len() <= 4,
        "{} threads opened files, but --jobs 4 allows at most 4 workers",
        parallel.openers.len()
    );
    assert_eq!(
        parallel.result, sequential.result,
        "status, stdout, and stderr must not depend on --jobs: buffer each file's output \
         and messages, then write them in input order"
    );
}

#[test]
fn c_binary_jobs_flag() {
    let run = |jobs: &str, command: &str| {
        let mut loglens = Process::new(env!("CARGO_BIN_EXE_loglens"));
        loglens
            .args([
                command,
                "--jobs",
                jobs,
                "fixtures/app.log",
                "fixtures/web.log",
                "fixtures/worker.log",
            ])
            .current_dir(env!("CARGO_MANIFEST_DIR"));
        output_within(LIMIT, loglens, b"")
    };
    for command in ["grep", "count", "report"] {
        let expected = run("1", command);
        assert_eq!(
            expected.0, 0,
            "{command}: records matched, so the status is 0"
        );
        assert_eq!(run("4", command), expected, "{command} with --jobs 4");
        assert_eq!(run("0", command), expected, "{command} with --jobs 0");
    }
}

/// P. Predict BEFORE running: replace each `guess()` with your answer.
#[test]
fn p_predictions() {
    // P1. How many values arrive once every sender is dropped?
    let (tx, rx) = mpsc::channel();
    for i in 0..3 {
        let tx = tx.clone();
        thread::spawn(move || tx.send(i).unwrap());
    }
    drop(tx);
    let p1: usize = guess();
    assert_eq!(
        p1,
        rx.iter().count(),
        "P1: the loop ends when all senders are gone"
    );

    // P2. The strong count after a scoped thread cloned and dropped an Arc.
    let shared = Arc::new(5);
    thread::scope(|s| {
        s.spawn(|| {
            let mine = Arc::clone(&shared);
            assert_eq!(*mine, 5);
        });
    });
    let p2: usize = guess();
    assert_eq!(
        p2,
        Arc::strong_count(&shared),
        "P2: the clone died with the thread"
    );

    // P3. What does fetch_add return?
    let n = AtomicUsize::new(5);
    let p3: usize = guess();
    assert_eq!(
        p3,
        n.fetch_add(2, Ordering::SeqCst),
        "P3: the previous value"
    );
}
