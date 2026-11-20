//! E07 capstone: the tracing contract of `run_jobs`, checked with your E04 `capture`.
//! Run: cargo test --manifest-path ecosystem/Cargo.toml --test e07_tracing
//!
//! Why a separate test binary? tracing caches each callsite's "interest" globally.
//! When other tests in the same process hit the same callsites at the same moment
//! a scoped subscriber is being installed, that cache can go stale and events
//! are silently dropped. One test per process avoids the race.

// Shared test helpers; this file uses the time limit (`within`).
#[allow(dead_code)]
mod test_support {
    include!("../src/test_support.rs");
}

use ecosystem::e04_tracing::capture;
use ecosystem::e07_capstone::{Config, Job, Task, run_jobs};

fn job(id: &str, task: Task) -> Job {
    Job {
        id: id.to_owned(),
        task,
    }
}

#[test]
fn c_jobs_are_traced_in_spans() {
    let cfg = Config {
        workers: 1,
        queue_capacity: 2,
        job_timeout_ms: 20,
        jobs: vec![
            job("a", Task::Sleep { ms: 1 }),
            job(
                "b",
                Task::Fail {
                    message: "boom".into(),
                },
            ),
            job("c", Task::Sleep { ms: 30 }),
        ],
    };
    // `within` fails the test, instead of hanging, if `run_jobs` never returns
    // (for example because the workers wait on a queue that is never closed).
    let (report, out) = test_support::within(move || {
        let runtime = test_support::paused_time();
        capture(tracing::Level::INFO, || runtime.block_on(run_jobs(&cfg)))
    });
    assert_eq!(report.jobs.len(), 3);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(
        lines,
        [
            " INFO job{id=a}: job started",
            " INFO job{id=a}: job finished status=\"succeeded\"",
            " INFO job{id=b}: job started",
            " WARN job{id=b}: job finished status=\"failed\"",
            " INFO job{id=c}: job started",
            " WARN job{id=c}: job finished status=\"timed_out\"",
        ],
        "{out}"
    );
}
