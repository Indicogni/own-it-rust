//! LESSON 04 — Testing at scale: snapshots, fixtures, flaky tests, coverage, mutants
//! Run: cargo test --manifest-path mastery/m9_toolbox/Cargo.toml l04_
//! Read first: CONCEPTS.md, "Lesson 04 · Testing at scale".
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: a test suite runs every line of a
//! function. What can still be wrong with the function?
//! IMPLEMENT the tasks below. Task C edits `fixtures/l04_doctests/src/lib.rs`.
//! Preserve the tests and add an edge case.
//! EXPLAIN afterward: Explain what line coverage measures and what a surviving
//! mutant tells you that coverage cannot.
//! JUDGMENT: Use example tests for documented behaviour, snapshots for large
//! human-readable output, doc tests for the promises in your API docs, and mutation
//! testing to find out whether any of them would notice a bug.
//!
//! This module is compiled only for tests, because it uses the dev-dependencies
//! `insta` and `tempfile` (see `#[cfg(test)] mod l04_testing;` in `lib.rs`).

use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// The result of one test suite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suite {
    pub name: String,
    pub passed: u32,
    pub failed: u32,
    pub ignored: u32,
}

/// A. Renders a summary table. The accepted snapshot
/// `src/snapshots/m9_toolbox__l04_testing__tests__report.snap` is the specification
/// of the layout: read it, then make this function produce exactly that text.
///
/// - One row per suite, sorted by name. Then a `total` row. Then a result line.
/// - The first column is as wide as the longest name in it (`suite` and `total`
///   count as names). The number columns are right-aligned under their headings.
/// - The result line is `result: ok`, or `result: FAILED (N failed)` when any test
///   failed.
/// - Every line ends with `\n`. No trailing spaces.
///
/// Do not edit the snapshot file. In real work you review a changed snapshot with
/// `cargo insta review`; here the snapshot plays the role of the reviewed version.
pub fn render_report(suites: &[Suite]) -> String {
    todo!("Implement this task")
}

/// Produces a run log like a test runner would. Supplied; do not edit. The log
/// contains two things that differ on every run: real durations and the path `root`.
pub fn run_log(root: &Path) -> String {
    let mut log = format!("running 3 checks in {}/cases\n", root.display());
    let started = std::time::Instant::now();
    for (name, spins) in [("parse", 10_u64), ("render", 1_000), ("search", 100_000)] {
        let before = std::time::Instant::now();
        let mut sum = 0_u64;
        for i in 0..spins {
            sum = std::hint::black_box(sum.wrapping_add(i));
        }
        log.push_str(&format!("check {name} ... ok ({:?})\n", before.elapsed()));
    }
    log.push_str(&format!("wrote {}/report.txt in 3 steps\n", root.display()));
    log.push_str(&format!(
        "finished in {:?}; 3 checks, 0 failures\n",
        started.elapsed()
    ));
    log
}

/// B. Makes a log stable enough for a snapshot.
///
/// - Every occurrence of `root` (as printed by `root.display()`) becomes `[ROOT]`.
/// - Every duration becomes `[TIME]`. A duration is what `Duration`'s `Debug` output
///   looks like: digits, optionally with one fractional part (`12`, `1.5`,
///   `83.25`), directly followed by one of the units `ns`, `µs`, `ms`, `s`. The unit
///   must not be followed by a letter or a digit: `3 steps`, `3steps` and `v2s3` stay.
/// - Everything else is unchanged, including line ends.
pub fn redact(log: &str, root: &Path) -> String {
    todo!("Implement this task")
}

// C. Doc tests as a contract. Edit the documentation comments in
// `fixtures/l04_doctests/src/lib.rs` (the crate `thermo`) until `cargo test --doc`
// passes there with at least four doc tests:
//
//   - repair the two stale examples (wrong function name, wrong values);
//   - the `parse_celsius` example uses `?` instead of `unwrap()`, with a hidden last
//     line that gives the example its `Result` type;
//   - add an example that proves a `Fahrenheit` cannot be passed to `to_fahrenheit`:
//     a `compile_fail` block that names the expected error code;
//   - add a `should_panic` example to `Celsius::new`.

/// D. A scratch directory for one test. The supplied version works when you run one
/// test at a time and fails in a parallel run. Fix it; you may change the fields.
///
/// - Every `Scratch` is a new, empty directory that no other `Scratch` uses.
/// - The directory and its content are removed when the `Scratch` is dropped,
///   also when the test panics.
pub struct Scratch {
    dir: PathBuf,
}

impl Scratch {
    pub fn new() -> io::Result<Self> {
        let dir = std::env::temp_dir().join("m9-toolbox-scratch");
        std::fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    /// The directory itself.
    pub fn path(&self) -> &Path {
        &self.dir
    }

    /// Writes `text` to the file `name` inside the directory and returns its path.
    pub fn write(&self, name: &str, text: &str) -> io::Result<PathBuf> {
        let path = self.path().join(name);
        std::fs::write(&path, text)?;
        Ok(path)
    }
}

/// D. Word frequencies: `(word, count)`, most frequent first; words with the same
/// count in alphabetical order. Words are separated by whitespace and compared
/// exactly. The supplied version gives the right counts in an order that changes
/// from run to run. Fix it.
pub fn word_counts(text: &str) -> Vec<(String, usize)> {
    let mut counts = std::collections::HashMap::new();
    for word in text.split_whitespace() {
        *counts.entry(word.to_owned()).or_insert(0) += 1;
    }
    let mut pairs: Vec<(String, usize)> = counts.into_iter().collect();
    pairs.sort_by_key(|pair| std::cmp::Reverse(pair.1));
    pairs
}

/// D. Is an entry created at `created` older than `ttl` (or exactly as old) at the
/// moment `now`? The caller passes the clock reading, so a test can pick any moment
/// and never sleeps. If `now` is before `created` (the system clock was set back),
/// the entry is not expired, and nothing panics.
pub fn is_expired_at(created: SystemTime, ttl: Duration, now: SystemTime) -> bool {
    todo!("Implement this task")
}

/// The price of a parcel in cents. Supplied and correct; do not edit.
///
/// - up to 500 g: 300
/// - up to 2000 g: 550
/// - above 2000 g: 550, plus 120 for every started kilogram above 2000 g
/// - express doubles the price
pub fn shipping_cents(weight_g: u32, express: bool) -> u32 {
    let base = if weight_g <= 500 {
        300
    } else if weight_g <= 2000 {
        550
    } else {
        550 + (weight_g - 2000).div_ceil(1000) * 120
    };
    if express { base * 2 } else { base }
}

/// E. A test table for [`shipping_cents`]: `((weight_g, express), expected price)`.
///
/// A test runs every case against the real function and against seven slightly
/// wrong versions of it ("mutants"). Your table must hold for the real function and
/// must expose every mutant, with at most eight cases. Choose the cases by thinking
/// about where a small change in the code would change the result.
pub fn shipping_cases() -> Vec<((u32, bool), u32)> {
    todo!("Implement this task")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guess;
    use crate::support::{self, cargo, fixture};
    use std::sync::{Arc, Barrier};

    fn sample_suites() -> Vec<Suite> {
        let suite = |name: &str, passed, failed, ignored| Suite {
            name: name.to_owned(),
            passed,
            failed,
            ignored,
        };
        vec![
            suite("unit", 140, 0, 4),
            suite("cli_end_to_end", 3, 2, 0),
            suite("api", 12, 0, 1),
        ]
    }

    #[test]
    fn a_report_matches_the_accepted_snapshot() {
        insta::assert_snapshot!("report", render_report(&sample_suites()));
    }

    #[test]
    fn a_report_edge_cases() {
        assert_eq!(
            render_report(&[]),
            "suite passed failed ignored\ntotal      0      0       0\nresult: ok\n",
            "with no suites, the first column is as wide as `suite` and `total`"
        );
        let one = [Suite {
            name: "db".to_owned(),
            passed: 7,
            failed: 0,
            ignored: 0,
        }];
        assert_eq!(
            render_report(&one),
            "suite passed failed ignored\n\
             db         7      0       0\n\
             total      7      0       0\n\
             result: ok\n"
        );
    }

    #[test]
    fn a_snapshot_files_are_the_reviewed_ones() {
        // In this exercise the snapshots are the specification. A snapshot test can
        // always be made green by accepting whatever the code prints; this check
        // makes sure that did not happen here.
        for (file, expected) in [
            ("m9_toolbox__l04_testing__tests__report.snap", REPORT_SNAP),
            ("m9_toolbox__l04_testing__tests__run_log.snap", RUN_LOG_SNAP),
        ] {
            let path = support::package_root().join("src/snapshots").join(file);
            let text = support::read(&path);
            assert_eq!(
                support::fnv1a(text.as_bytes()),
                expected,
                "{} was changed. Restore it from version control, then fix the code, \
                 not the snapshot.",
                support::shown(&path)
            );
        }
    }

    const REPORT_SNAP: u64 = 0x708a19449a25362b;
    const RUN_LOG_SNAP: u64 = 0xea32400fb4316429;

    #[test]
    fn b_redact_replaces_durations_and_the_root() {
        crate::support::within(30, "b_redact_replaces_durations_and_the_root", || {
            let root = Path::new("/tmp/run-41");
            assert_eq!(
                redact("check a ... ok (12ms)\n", root),
                "check a ... ok ([TIME])\n"
            );
            assert_eq!(
                redact("83ns 1.5µs 20.25ms 3s 0.001s", root),
                "[TIME] [TIME] [TIME] [TIME] [TIME]",
                "all four units, with and without a fraction"
            );
            assert_eq!(
                redact("wrote /tmp/run-41/a.txt and /tmp/run-41/b.txt", root),
                "wrote [ROOT]/a.txt and [ROOT]/b.txt",
                "every occurrence of the root is replaced"
            );
            assert_eq!(redact("", root), "");
        });
    }

    #[test]
    fn b_redact_leaves_other_numbers_alone() {
        crate::support::within(30, "b_redact_leaves_other_numbers_alone", || {
            let root = Path::new("/tmp/run-41");
            for text in [
                "3 steps",
                "3steps",
                "5 checks, 0 failures",
                "version 1.2.3",
                "v2s3",
                "12 ms",
                "100%",
                "10seconds",
                "port 8080",
            ] {
                assert_eq!(
                    redact(text, root),
                    text,
                    "`{text}` contains no duration and must stay as it is"
                );
            }
            assert_eq!(
                redact("took 5s.", root),
                "took [TIME].",
                "punctuation after the unit is fine"
            );
            assert_eq!(redact("1.5s,2ms;3ns", root), "[TIME],[TIME];[TIME]");
        });
    }

    #[test]
    fn b_redacted_run_log_matches_the_snapshot() {
        // The directory name is different in every run, and so are the durations.
        let scratch = tempfile::tempdir().expect("a temporary directory");
        let log = run_log(scratch.path());
        assert!(
            log.contains(&scratch.path().display().to_string()),
            "the raw log contains the real path"
        );
        // A `redact` that never returns must fail here, not hang the snapshot check.
        let (probe_log, probe_root) = (log.clone(), scratch.path().to_path_buf());
        support::within(30, "redact on the run log", move || {
            redact(&probe_log, &probe_root)
        });
        insta::assert_snapshot!("run_log", redact(&log, scratch.path()));
    }

    #[test]
    fn c_doc_tests_are_a_checked_contract() {
        let krate = fixture("l04_doctests");
        let source = support::read(&krate.join("src/lib.rs"));
        let run = cargo(&krate, &["test", "--doc"]);
        run.expect_ok("`cargo test --doc` must pass in fixtures/l04_doctests");
        let passed: usize = run
            .stdout
            .lines()
            .filter(|line| line.starts_with("test ") && line.ends_with("... ok"))
            .count();
        assert!(
            passed >= 4,
            "expected at least four passing doc tests, found {passed}:\n{}",
            run.stdout
        );
        assert!(
            run.stdout.contains("- compile fail ... ok"),
            "add a `compile_fail` example for the Fahrenheit/Celsius mix-up"
        );
        assert!(
            source.contains("compile_fail,E0308"),
            "a bare `compile_fail` passes for any error, even a typo. Name the error \
             code you expect: ```compile_fail,E0308"
        );
        assert!(
            run.stdout.contains("- should panic ... ok"),
            "add a `should_panic` example to `Celsius::new`"
        );
        let doc_lines: Vec<&str> = source
            .lines()
            .map(str::trim_start)
            .filter(|line| line.starts_with("///"))
            .collect();
        assert!(
            !doc_lines.iter().any(|line| line.contains("unwrap()")),
            "examples teach by imitation: use `?` in them, not `unwrap()`"
        );
        assert!(
            doc_lines.iter().any(|line| line.starts_with("/// # Ok::<")),
            "an example that uses `?` ends with a hidden line such as \
             `# Ok::<(), ParseError>(())`"
        );
    }

    #[test]
    fn d_scratch_directories_are_isolated() {
        crate::support::within(60, "d_scratch_directories_are_isolated", || {
            let first = Scratch::new().unwrap();
            let second = Scratch::new().unwrap();
            assert_ne!(
                first.path(),
                second.path(),
                "two Scratch values must not share a directory"
            );
            assert!(first.path().is_dir());
            assert_eq!(
                std::fs::read_dir(first.path()).unwrap().count(),
                0,
                "a new Scratch must be empty, whatever earlier tests left behind"
            );

            // Technique: eight "tests" run at the same moment. Each writes its own
            // number to `data.txt`, waits for the others, and reads the file back.
            let barrier = Arc::new(Barrier::new(8));
            let workers: Vec<_> = (0..8)
                .map(|id| {
                    let barrier = Arc::clone(&barrier);
                    std::thread::spawn(move || {
                        let scratch = Scratch::new().unwrap();
                        let file = scratch.write("data.txt", &id.to_string()).unwrap();
                        barrier.wait();
                        std::fs::read_to_string(file).unwrap()
                    })
                })
                .collect();
            for (id, worker) in workers.into_iter().enumerate() {
                assert_eq!(
                    worker.join().unwrap(),
                    id.to_string(),
                    "test {id} read another test's data: the tests share a directory"
                );
            }
        });
    }

    #[test]
    fn d_scratch_cleans_up_even_on_panic() {
        let scratch = Scratch::new().unwrap();
        let path = scratch.path().to_path_buf();
        scratch.write("left-over.txt", "x").unwrap();
        drop(scratch);
        assert!(
            !path.exists(),
            "dropping a Scratch must remove its directory"
        );

        let (tx, rx) = std::sync::mpsc::channel();
        let result = std::thread::spawn(move || {
            let scratch = Scratch::new().unwrap();
            tx.send(scratch.path().to_path_buf()).unwrap();
            panic!("a failing test (this panic is expected)");
        })
        .join();
        assert!(result.is_err());
        let path = rx.recv().unwrap();
        assert!(
            !path.exists(),
            "a panicking test must not leave its directory behind"
        );
    }

    #[test]
    fn d_word_counts_have_one_defined_order() {
        assert_eq!(word_counts(""), []);
        assert_eq!(
            word_counts("b a b"),
            [("b".to_owned(), 2), ("a".to_owned(), 1)]
        );
        // Technique: eight words with the same count. A hash map hands them out in a
        // different order for every map, so an unsorted tie fails here.
        let expected: Vec<(String, usize)> = [
            "the", "ant", "bee", "cat", "dog", "eel", "fox", "gnu", "hen",
        ]
        .iter()
        .map(|word| ((*word).to_owned(), if *word == "the" { 3 } else { 1 }))
        .collect();
        for _ in 0..20 {
            assert_eq!(
                word_counts("the hen gnu the fox eel dog the cat bee ant"),
                expected,
                "ties must come out in alphabetical order, every time"
            );
        }
    }

    #[test]
    fn d_expiry_takes_the_clock_as_an_argument() {
        let created = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        let ttl = Duration::from_secs(60);
        let at = |secs: u64| SystemTime::UNIX_EPOCH + Duration::from_secs(secs);
        assert!(!is_expired_at(created, ttl, at(1_000)));
        assert!(!is_expired_at(created, ttl, at(1_059)));
        assert!(
            is_expired_at(created, ttl, at(1_060)),
            "exactly as old as the ttl counts as expired"
        );
        assert!(is_expired_at(created, ttl, at(5_000)));
        assert!(
            !is_expired_at(created, ttl, at(900)),
            "a clock that was set back must not expire anything, and must not panic"
        );
        assert!(is_expired_at(created, Duration::ZERO, created));
    }

    type Price = fn(u32, bool) -> u32;

    /// Seven versions of `shipping_cents`, each with one small bug.
    fn mutants() -> [(&'static str, Price); 7] {
        fn light_tier_ends_early(weight_g: u32, express: bool) -> u32 {
            let base = if weight_g < 500 {
                300
            } else if weight_g <= 2000 {
                550
            } else {
                550 + (weight_g - 2000).div_ceil(1000) * 120
            };
            if express { base * 2 } else { base }
        }
        fn heavy_starts_from_light_price(weight_g: u32, express: bool) -> u32 {
            let base = if weight_g <= 500 {
                300
            } else if weight_g <= 2000 {
                550
            } else {
                300 + (weight_g - 2000).div_ceil(1000) * 120
            };
            if express { base * 2 } else { base }
        }
        fn started_kilogram_rounds_down(weight_g: u32, express: bool) -> u32 {
            let base = if weight_g <= 500 {
                300
            } else if weight_g <= 2000 {
                550
            } else {
                550 + (weight_g - 2000) / 1000 * 120
            };
            if express { base * 2 } else { base }
        }
        fn surcharge_added_not_multiplied(weight_g: u32, express: bool) -> u32 {
            let base = if weight_g <= 500 {
                300
            } else if weight_g <= 2000 {
                550
            } else {
                550 + (weight_g - 2000).div_ceil(1000) + 120
            };
            if express { base * 2 } else { base }
        }
        fn kilograms_counted_from_1000(weight_g: u32, express: bool) -> u32 {
            let base = if weight_g <= 500 {
                300
            } else if weight_g <= 2000 {
                550
            } else {
                550 + (weight_g - 1000).div_ceil(1000) * 120
            };
            if express { base * 2 } else { base }
        }
        fn express_adds_two(weight_g: u32, express: bool) -> u32 {
            let base = if weight_g <= 500 {
                300
            } else if weight_g <= 2000 {
                550
            } else {
                550 + (weight_g - 2000).div_ceil(1000) * 120
            };
            if express { base + 2 } else { base }
        }
        fn express_skips_the_surcharge(weight_g: u32, express: bool) -> u32 {
            let factor = if express { 2 } else { 1 };
            if weight_g <= 500 {
                300 * factor
            } else if weight_g <= 2000 {
                550 * factor
            } else {
                550 * factor + (weight_g - 2000).div_ceil(1000) * 120
            }
        }
        [
            (
                "the light tier ends one gram early (`<` for `<=`)",
                light_tier_ends_early,
            ),
            (
                "the heavy tier starts from the light price (300 for 550)",
                heavy_starts_from_light_price,
            ),
            (
                "a started kilogram is rounded down (`/` for `div_ceil`)",
                started_kilogram_rounds_down,
            ),
            (
                "the surcharge is added, not multiplied (`+ 120` for `* 120`)",
                surcharge_added_not_multiplied,
            ),
            (
                "the kilograms are counted from 1000 g, not from 2000 g",
                kilograms_counted_from_1000,
            ),
            (
                "express adds 2 instead of doubling (`+` for `*`)",
                express_adds_two,
            ),
            (
                "express doubles the base price but not the surcharge",
                express_skips_the_surcharge,
            ),
        ]
    }

    #[test]
    fn e_the_cases_hold_and_kill_every_mutant() {
        let cases = shipping_cases();
        assert!(
            (1..=8).contains(&cases.len()),
            "write between one and eight cases, found {}",
            cases.len()
        );
        for ((weight_g, express), expected) in &cases {
            assert_eq!(
                shipping_cents(*weight_g, *express),
                *expected,
                "your case ({weight_g} g, express: {express}) expects the wrong price"
            );
        }
        for (description, mutant) in mutants() {
            let killed = cases
                .iter()
                .any(|((weight_g, express), expected)| mutant(*weight_g, *express) != *expected);
            assert!(
                killed,
                "no case notices this bug: {description}. Add or move a case."
            );
        }
    }

    #[test]
    fn e_full_coverage_is_not_enough() {
        // These four cases run every line and every branch of `shipping_cents`.
        let weak = [
            ((100, false), 300),
            ((1000, false), 550),
            ((3000, false), 670),
            ((1000, true), 1100),
        ];
        let survivors = mutants()
            .iter()
            .filter(|(_, mutant)| weak.iter().all(|((w, e), want)| mutant(*w, *e) == *want))
            .count();
        // E-P. How many of the seven mutants pass all four weak cases? Work it out
        // from the mutant descriptions before you run this test.
        let survivors_predicted: usize = guess();
        assert_eq!(
            survivors_predicted, survivors,
            "E-P: a case kills a mutant only if the bug changes the result for that input"
        );
    }

    /// LAB 4.4 (needs cargo-mutants; see LABS.md). Run with:
    /// `cargo test --manifest-path mastery/m9_toolbox/Cargo.toml lab_l04 -- --ignored`
    #[test]
    #[ignore = "LAB: needs cargo-mutants; see LABS.md"]
    fn lab_l04_mutants() {
        // How many mutants does `cargo mutants` report as MISSED in
        // `fixtures/l04_coverage`, before you improve its tests?
        let missed: &str = guess();
        assert!(
            support::lab_answer_is_right("l04.mutants.missed", missed),
            "not what cargo-mutants 27.1 reports for the unchanged fixture"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    ///
    /// Read `fixtures/l04_layout/`: a package with a library, a binary, and four
    /// files under `tests/`.
    #[test]
    fn p_predictions() {
        let layout = fixture("l04_layout");

        // P1. How many test executables does `cargo test` build for this package?
        // (The doc tests are not an executable of this kind; do not count them.)
        let build = cargo(&layout, &["test", "--no-run", "--message-format=json"]);
        build.expect_ok("the layout fixture must build");
        let executables = build
            .json_lines()
            .iter()
            .filter(|line| line["reason"] == "compiler-artifact")
            .filter(|line| line["profile"]["test"] == true && line["executable"].is_string())
            .count();
        let p1: usize = guess();
        assert_eq!(
            p1, executables,
            "P1: the library, the binary, and every file directly in tests/ become \
             one executable each; a subdirectory with mod.rs does not"
        );

        // P2. `cargo test` runs them all, then the doc tests. Add up the "passed"
        // counts of all "test result" lines.
        let run = cargo(&layout, &["test"]);
        run.expect_ok("the layout fixture's tests must pass");
        let count = |label: &str| -> usize {
            run.stdout
                .lines()
                .filter(|line| line.starts_with("test result:"))
                .filter_map(|line| {
                    let before = line.split(label).next()?;
                    before
                        .rsplit([' ', '.'])
                        .find(|w| !w.is_empty())?
                        .parse::<usize>()
                        .ok()
                })
                .sum()
        };
        let p2: usize = guess();
        assert_eq!(
            p2,
            count(" passed"),
            "P2: unit tests, integration tests and runnable doc examples all count"
        );

        // P3. And the sum of the "ignored" counts?
        let p3: usize = guess();
        assert_eq!(
            p3,
            count(" ignored"),
            "P3: an `#[ignore]` test and an `ignore` doc block are reported; \
             a `text` block is not a test at all"
        );
    }
}
