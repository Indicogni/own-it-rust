//! LESSON 07 — Changing slowly: deprecation, the minimum Rust version, editions
//! Run: cargo test --manifest-path mastery/m8_api/Cargo.toml l07_
//! Read first: "Lesson 07" in CONCEPTS.md (same package).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). The files it uses are in
//! `fixtures/l07/editions/`. Then consider: a deprecation breaks nobody's build.
//! Is that true for every user?
//!
//! IMPLEMENT:
//! - Task A, in this file. The file is the whole source of the library
//!   `clockwork`, version 1.3.0. The tests compile it as a crate of its own and
//!   build the programs in `fixtures/l07/cases/` against it.
//! - Task B, in `fixtures/l07/oldtimer/src/lib.rs`. The tests run `cargo clippy`
//!   and `cargo test` on that package.
//!
//! Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: Task B is checked by a lint. Name one way in which code
//! can pass that lint and still fail to build with Rust 1.74, and say what a
//! project must do to be able to promise its `rust-version` honestly.
//!
//! # Task A: release 1.4.0 without breaking a user of 1.3.0
//!
//! Two names in this library are not good Rust names: a getter does not start
//! with `get_` (API Guidelines, C-GETTER), and `fmt_secs` abbreviates what it
//! does and names its parameter, not its result. Version 1.4.0 introduces
//! better ones:
//!
//! | Since 1.0.0             | New in 1.4.0        |
//! | ----------------------- | ------------------- |
//! | `fmt_secs`              | `format_duration`   |
//! | `Timer::get_elapsed_ms` | `Timer::elapsed_ms` |
//!
//! - The new items have the signatures and the behaviour of the old ones.
//! - The old items stay, and still work: 1.4.0 is a minor release.
//! - Each old item is marked `#[deprecated]` with `since = "1.4.0"` and a `note`
//!   that names the replacement in backticks, as in
//!   ``use `format_duration` instead``; for the method, `` `elapsed_ms` `` and
//!   `` `Timer::elapsed_ms` `` are both fine. The note is the text that a user
//!   of the old name sees in the compiler's warning.
//! - The library itself compiles without a warning. (It calls one of the old
//!   names today.)
//!
//! # Task B: keep the promise in `rust-version`
//!
//! `fixtures/l07/oldtimer/Cargo.toml` promises that the library builds with
//! Rust 1.74. Its `src/lib.rs` uses four things from the standard library that
//! are newer. Find them with
//!
//! ```text
//! cargo clippy -- -D clippy::incompatible_msrv
//! ```
//!
//! (run in `fixtures/l07/oldtimer/`), and rewrite them with what Rust 1.74 has.
//! The manifest stays as it is: raising `rust-version` would leave the pinned
//! user behind. The behaviour stays as it is: `tests/behaviour.rs` must pass.

/// Formats a number of seconds for people: `5s`, `2m 05s`, `1h 02m 03s`.
pub fn format_duration(secs: u64) -> String {
    let (hours, minutes, seconds) = (secs / 3600, secs / 60 % 60, secs % 60);
    if hours > 0 {
        format!("{hours}h {minutes:02}m {seconds:02}s")
    } else if minutes > 0 {
        format!("{minutes}m {seconds:02}s")
    } else {
        format!("{seconds}s")
    }
}

/// Formats a number of seconds for people: `5s`, `2m 05s`, `1h 02m 03s`.
// The old name is a thin wrapper, so the two can never drift apart.
#[deprecated(since = "1.4.0", note = "use `format_duration` instead")]
pub fn fmt_secs(secs: u64) -> String {
    format_duration(secs)
}

/// A running timer. Times are milliseconds on a clock that the caller reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timer {
    started_ms: u64,
}

impl Timer {
    /// A timer that starts at `now_ms`.
    pub fn start(now_ms: u64) -> Timer {
        Timer { started_ms: now_ms }
    }

    /// The milliseconds from the start to `now_ms`; 0 if `now_ms` is earlier.
    pub fn elapsed_ms(&self, now_ms: u64) -> u64 {
        now_ms.saturating_sub(self.started_ms)
    }

    /// The milliseconds from the start to `now_ms`; 0 if `now_ms` is earlier.
    #[deprecated(since = "1.4.0", note = "use `elapsed_ms` instead")]
    pub fn get_elapsed_ms(&self, now_ms: u64) -> u64 {
        self.elapsed_ms(now_ms)
    }
}

/// The time that all `timers` together have run at `now_ms`, in milliseconds.
pub fn total_elapsed_ms(timers: &[Timer], now_ms: u64) -> u64 {
    timers.iter().map(|timer| timer.elapsed_ms(now_ms)).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::{self, Build, CargoRun};
    use std::path::Path;

    const LESSON: &str = "src/l07_msrv.rs";

    fn clockwork() -> std::path::PathBuf {
        support::library_from_file("clockwork", LESSON)
    }

    /// Runs `cargo ARGS` in the package `oldtimer`.
    fn oldtimer(args: &[&str]) -> CargoRun {
        support::cargo("fixtures/l07/oldtimer", "l07", args, &[])
    }

    // The old names must keep working. The test may call them without a warning.
    #[test]
    #[allow(deprecated)]
    fn a_the_old_names_still_work() {
        assert_eq!(fmt_secs(5), "5s");
        assert_eq!(fmt_secs(125), "2m 05s");
        assert_eq!(fmt_secs(3723), "1h 02m 03s");
        assert_eq!(fmt_secs(0), "0s");
        let timer = Timer::start(1_000);
        assert_eq!(timer.get_elapsed_ms(1_250), 250);
        assert_eq!(
            timer.get_elapsed_ms(900),
            0,
            "a clock that went back gives 0, not a panic"
        );
        assert_eq!(total_elapsed_ms(&[timer, Timer::start(1_200)], 1_250), 300);
    }

    #[test]
    fn a_a_user_of_1_3_still_builds_and_runs() {
        let clockwork = clockwork();
        support::case(
            "fixtures/l07/cases/user_of_1_3.rs",
            &[("clockwork", &clockwork)],
        );
    }

    #[test]
    fn a_a_user_of_the_new_names_gets_no_warning() {
        let clockwork = clockwork();
        support::case(
            "fixtures/l07/cases/user_of_1_4.rs",
            &[("clockwork", &clockwork)],
        );
    }

    #[test]
    fn a_a_user_of_the_old_names_is_told_what_to_use() {
        let clockwork = clockwork();
        let build = Build {
            externs: &[("clockwork", &clockwork)],
            warnings: true,
            ..Build::default()
        };
        let path = support::package_dir().join("fixtures/l07/cases/user_of_1_3.rs");
        let run = support::program("l07-warnings", &path, &build);
        assert_eq!(
            run.outcome, "ok",
            "a deprecation must not break the build:\n{}",
            run.output
        );
        for (old, new) in [
            ("fmt_secs", "format_duration"),
            ("get_elapsed_ms", "elapsed_ms"),
        ] {
            // The new name in backticks, alone or at the end of a path:
            // `elapsed_ms` and `Timer::elapsed_ms` are both good notes.
            let alone = format!("`{new}`");
            let in_a_path = format!("::{new}`");
            let told = run.warnings.lines().any(|line| {
                line.contains("use of deprecated")
                    && line.contains(old)
                    && (line.contains(&alone) || line.contains(&in_a_path))
            });
            assert!(
                told,
                "A user who calls `{old}` must get a warning that names `{new}` in \
                 backticks. Mark the old item with \
                 #[deprecated(since = \"1.4.0\", note = \"use `{new}` instead\")]. \
                 The compiler printed:\n{}",
                support::head(&run.warnings, 30)
            );
        }
    }

    #[test]
    fn a_the_deprecations_say_since_when() {
        let source = support::read(LESSON);
        let library = source
            .split("#[cfg(test)]")
            .next()
            .expect("split yields at least one part");
        // Every `#[deprecated(...)]` attribute, without white space.
        let attributes: Vec<String> = library
            .split("#[deprecated(")
            .skip(1)
            .filter_map(|rest| rest.split(")]").next())
            .map(|inner| inner.split_whitespace().collect())
            .collect();
        assert_eq!(
            attributes.len(),
            2,
            "exactly the two old names are deprecated"
        );
        for attribute in &attributes {
            assert!(
                attribute.contains("since=\"1.4.0\""),
                "a deprecation must say in which version it started: \
                 #[deprecated({attribute})] has no since = \"1.4.0\""
            );
        }
    }

    #[test]
    fn a_the_library_itself_has_no_warnings() {
        let build = Build {
            warnings: true,
            flags: &["-D", "warnings"],
            ..Build::default()
        };
        if let Err(errors) = support::library("clockwork", Path::new(LESSON), &build) {
            panic!(
                "The library must compile without warnings. If it calls its own \
                 deprecated function, change that call to the new name:\n{}",
                support::head(&errors, 30)
            );
        }
    }

    #[test]
    fn b_the_manifest_still_promises_rust_1_74() {
        let run = oldtimer(&["metadata", "--no-deps", "--format-version", "1"]);
        assert!(
            run.ok,
            "cargo cannot read the manifest of oldtimer:\n{}",
            run.stderr
        );
        assert_eq!(
            support::json_string(&run.stdout, "rust_version").as_deref(),
            Some("1.74"),
            "do not raise `rust-version`: a user is pinned to Rust 1.74"
        );
        assert_eq!(
            support::json_string(&run.stdout, "edition").as_deref(),
            Some("2021"),
            "do not change the edition: edition 2024 needs Rust 1.85"
        );
    }

    #[test]
    fn b_the_library_uses_nothing_newer_than_rust_1_74() {
        // `-F` (forbid) and not `-D` (deny): an `#[allow(...)]` in the source
        // cannot switch a forbidden lint off.
        let run = oldtimer(&["clippy", "--quiet", "--", "-F", "clippy::incompatible_msrv"]);
        assert!(
            run.ok,
            "clippy found items of the standard library that Rust 1.74 does not have \
             (or it found that the lint was switched off in the source):\n{}",
            support::head(&run.stderr, 50)
        );
    }

    #[test]
    fn b_the_behaviour_is_unchanged() {
        let run = oldtimer(&["test", "--quiet"]);
        assert!(
            run.ok,
            "fixtures/l07/oldtimer/tests/behaviour.rs must still pass:\n{}",
            support::head(&run.all(), 50)
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    ///
    /// Every answer is an outcome word: `"ok"` (compiles and runs), `"panic"`,
    /// `"error"` (a compile error without an error code), or an error code such
    /// as `"E0308"`. The files are in `fixtures/l07/editions/`.
    #[test]
    fn p_predictions() {
        let dir = support::package_dir().join("fixtures/l07/editions");
        let outcome = |label: &str, file: &str, build: &Build<'_>| -> String {
            support::program(label, &dir.join(file), build).outcome
        };
        let edition = |edition: &'static str| Build {
            edition,
            ..Build::default()
        };

        // P1. `keyword.rs` has a function with the name `gen`.
        let p1_edition_2021: &str = "ok";
        let p1_edition_2024: &str = "error";
        assert!(
            support::same_outcome(
                p1_edition_2021,
                &outcome("l07-p1a", "keyword.rs", &edition("2021"))
            ),
            "P1: `gen` is an ordinary name in edition 2021"
        );
        assert!(
            support::same_outcome(
                p1_edition_2024,
                &outcome("l07-p1b", "keyword.rs", &edition("2024"))
            ),
            "P1: edition 2024 reserves `gen` as a keyword; the parser rejects it, and \
             errors of the parser have no code"
        );

        // P2. `capture.rs` returns `impl Iterator` from a function that takes a
        // reference, and uses the iterator after the referenced value is gone.
        let p2_edition_2021: &str = "ok";
        let p2_edition_2024: &str = "E0597";
        assert!(
            support::same_outcome(
                p2_edition_2021,
                &outcome("l07-p2a", "capture.rs", &edition("2021"))
            ),
            "P2: in edition 2021 an `impl Trait` return type can use only the lifetimes \
             that it names, and this one names none"
        );
        assert!(
            support::same_outcome(
                p2_edition_2024,
                &outcome("l07-p2b", "capture.rs", &edition("2024"))
            ),
            "P2: in edition 2024 an `impl Trait` return type may use every lifetime in \
             scope, so the caller must keep `limit` alive"
        );

        // P3. `old_lib.rs` is compiled as edition 2021 and exports a function
        // `gen`. `new_app.rs` is compiled as edition 2024 and calls it.
        let old_lib = support::library("old_lib", &dir.join("old_lib.rs"), &edition("2021"))
            .expect("old_lib.rs compiles as edition 2021");
        let with_old_lib = Build {
            externs: &[("old_lib", &old_lib)],
            ..Build::default()
        };
        let p3: &str = "ok";
        assert!(
            support::same_outcome(p3, &outcome("l07-p3", "new_app.rs", &with_old_lib)),
            "P3: the edition is a property of one crate. Crates of different editions \
             link together, so a library can choose its edition without asking its users"
        );

        // P4. `deprecated_lib.rs` has deprecated `get_area`. Two programs call it.
        // `strict_app.rs` starts with `#![deny(warnings)]`.
        let deprecated_lib = support::library(
            "deprecated_lib",
            &dir.join("deprecated_lib.rs"),
            &Build::default(),
        )
        .expect("deprecated_lib.rs compiles");
        let with_deprecated_lib = Build {
            externs: &[("deprecated_lib", &deprecated_lib)],
            ..Build::default()
        };
        let p4_relaxed: &str = "ok";
        let p4_strict: &str = "error";
        assert!(
            support::same_outcome(
                p4_relaxed,
                &outcome("l07-p4a", "relaxed_app.rs", &with_deprecated_lib)
            ),
            "P4: a deprecation is a warning"
        );
        assert!(
            support::same_outcome(
                p4_strict,
                &outcome("l07-p4b", "strict_app.rs", &with_deprecated_lib)
            ),
            "P4: a project that turns warnings into errors is broken by a deprecation. \
             A lint that became an error has no error code"
        );
    }
}
