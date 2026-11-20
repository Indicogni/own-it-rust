//! LESSON 04 — Making a change the maintainers will accept
//! Run: cargo test --manifest-path mastery/m10_contributing/Cargo.toml l04_
//!      cargo test --manifest-path mastery/m10_contributing/Cargo.toml --test downstream
//! Read first: CONCEPTS.md, "Lesson 04", then `fixture/CONTRIBUTING.md` and
//! `fixture/issues/0021-dry-run.md` including the maintainer's comment.
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: Which parts of tasklane's API can grow
//! without breaking a user, and what in the source tells you so?
//! IMPLEMENT the dry-run feature in the fixture, in the shape the maintainers accepted
//! in the issue: code, documentation, a test in the fixture's own suite, a README
//! line, and a changelog entry. There is no `todo!()` in this lesson; the work is in
//! `fixture/`. The tests here and in `tests/downstream.rs` are the acceptance tests.
//! EXPLAIN afterward: Explain why `RunOptions` could grow and `TaskStatus` could not,
//! and what you would have had to do if the feature had needed a new status.

/// Whether a change keeps every correct program of a user compiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Semver {
    /// Every program that compiled against 0.4.0 still compiles. Allowed in 0.4.1.
    Compatible,
    /// Some program that compiled against 0.4.0 stops compiling. Needs 0.5.0.
    Breaking,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::{digest, list_items, mentions_issue, read, section};
    // Unused once every prediction is answered.
    #[allow(unused_imports)]
    use crate::guess;

    const SITE: &str = include_str!("../fixture/examples/site.lane");

    /// Runs the tasklane command line in this process.
    fn tasklane(source: &str, args: &[&str]) -> (u8, String, String) {
        let args: Vec<String> = args.iter().map(|arg| arg.to_string()).collect();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = tasklane_cli::run_with(&args, &|_| Ok(source.to_string()), &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    #[test]
    fn a_dry_run_flag_runs_nothing() {
        for flag in ["--dry-run", "-n"] {
            let (code, out, err) =
                tasklane(SITE, &["run", "site", flag, "--show", "files,summary"]);
            assert_eq!(
                (code, err.as_str()),
                (0, ""),
                "`tasklane run site {flag}` must be accepted and succeed"
            );
            assert_eq!(
                out, "0 done, 0 failed, 5 skipped\n",
                "`{flag}`: no file may exist afterwards, and every planned task is skipped"
            );
        }
    }

    #[test]
    fn a_dry_run_still_reports_lanefile_mistakes() {
        let (code, _, err) = tasklane("task a\n  step echo ${NOPE}\n", &["run", "a", "--dry-run"]);
        assert_eq!(code, 3, "a dry run checks variables like a real run");
        assert_eq!(err, "error: Lanefile: unknown variable `NOPE`\n");
        let (code, _, _) = tasklane("task a\n  step polish x\n", &["run", "a", "--dry-run"]);
        assert_eq!(code, 3, "a dry run checks step names like a real run");
    }

    #[test]
    fn a_without_the_flag_nothing_changed() {
        let (code, out, _) = tasklane(SITE, &["run", "site", "-q"]);
        assert_eq!((code, out.as_str()), (0, "5 done, 0 failed, 0 skipped\n"));
    }

    #[test]
    fn b_help_and_readme_mention_the_flag() {
        let (_, help, _) = tasklane("", &["--help"]);
        assert!(
            help.contains("--dry-run"),
            "B: `tasklane --help` does not mention --dry-run"
        );
        assert!(
            read("fixture/README.md").contains("--dry-run"),
            "B: fixture/README.md lists the options of the command; add --dry-run"
        );
    }

    #[test]
    fn c_changelog_has_an_entry_under_unreleased() {
        let changelog = read("fixture/CHANGELOG.md");
        let unreleased = section(&changelog, "## [Unreleased]")
            .expect("fixture/CHANGELOG.md has a `## [Unreleased]` section");
        let added = section(unreleased, "### Added").unwrap_or("");
        let entries = list_items(added);
        let entry = entries
            .iter()
            .find(|entry| entry.to_lowercase().contains("dry"))
            .unwrap_or_else(|| {
                panic!(
                    "C: add a line about the dry run under `### Added` in the \
                     `## [Unreleased]` section of fixture/CHANGELOG.md"
                )
            });
        assert!(
            mentions_issue(entry, 21),
            "C: the entry must reference the issue like the other entries do: `(#21)`"
        );
    }

    #[test]
    fn d_fixture_tests_cover_the_feature() {
        let covered = [
            "fixture/crates/tasklane-core/tests/run.rs",
            "fixture/crates/tasklane-core/src/run.rs",
            "fixture/crates/tasklane-cli/tests/cli.rs",
        ]
        .iter()
        .map(|path| read(path))
        .any(|source| {
            source
                .split("#[test]")
                .skip(1)
                .any(|test| test.contains("dry_run") || test.contains("--dry-run"))
        });
        assert!(
            covered,
            "D: the fixture's own test suite has no test for the dry run. The tests of \
             this lesson will not be in the project after your pull request; yours will."
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with `Semver::Compatible` or
    /// `Semver::Breaking`. The question is always: does `downstream/pinned`, a user
    /// written against 0.4.0, still compile after this change to the fixture? Read
    /// `downstream/pinned/src/lib.rs` and the fixture's type definitions first.
    #[test]
    fn p_predictions() {
        // P1. `Runner::run` gets a fifth parameter, `dry_run: bool`.
        let p1: Semver = guess();
        // P2. `RunOptions` gets a private field and a setter method.
        let p2: Semver = guess();
        // P3. `TaskStatus` gets a variant `Planned`.
        let p3: Semver = guess();
        // P4. `Error` gets a variant `DryRun`.
        let p4: Semver = guess();
        // P5. `StepCall` gets a public field `dry: bool`.
        let p5: Semver = guess();
        // P6. The trait `Step` gets a method with a default body.
        let p6: Semver = guess();
        // P7. The trait `Step` gets a method without a default body.
        let p7: Semver = guess();
        // P8. `StepContext` gets a public field `dry_run: bool`.
        let p8: Semver = guess();

        let model = [
            0xcd16_6301_6366_679b_u64,
            0x5eb4_0dcc_51c3_53f3,
            0x1236_f4bc_dd09_9edd,
            0x0634_2da4_8656_29bd,
            0x99dd_e2ae_ae2b_82ef,
            0x46d1_fef5_8db1_b8b7,
            0xc79d_6d09_e28a_dc61,
            0xf616_214d_abd8_dc31,
        ];
        let topics = [
            "a new parameter changes every call",
            "private fields behind methods can grow",
            "an enum that users may match without a wildcard arm",
            "`#[non_exhaustive]` on an enum forces a wildcard arm",
            "a struct that users may build with a literal",
            "implementors do not have to write a defaulted method",
            "every implementor must write a required method",
            "`#[non_exhaustive]` on a struct forbids literals outside the crate",
        ];
        for (i, answer) in [p1, p2, p3, p4, p5, p6, p7, p8].iter().enumerate() {
            assert!(
                digest(&format!("P{} {answer:?}", i + 1)) == model[i],
                "P{}: {}",
                i + 1,
                topics[i]
            );
        }
    }
}
