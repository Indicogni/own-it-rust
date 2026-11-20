//! LESSON 10 — Capstone: one contribution, from issue to merge
//! Run: cargo test --manifest-path mastery/m10_contributing/Cargo.toml l10_
//! Read first: CONCEPTS.md, "Capstone", and the three open issues in `fixture/issues/`:
//! 0022 (a run that never finishes), 0025 (files that `concat` skips) and 0030 (a
//! test that fails every other time).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer).
//! IMPLEMENT: choose ONE of the three issues and take it through the whole path:
//! reproduce it with a new failing test in the fixture, fix it, keep everything
//! else unchanged, add a changelog entry under `### Fixed`, and write the commit
//! message (`answers/l10_commit.txt`) and the pull request (`answers/l10_pr.md`).
//! The test `z_chosen_issue_is_ready_to_merge` reads the issue number from your pull
//! request and checks that issue from end to end. The other two issues stay red
//! until you do them too; they are extra practice, not part of the capstone. There
//! is no `todo!()` in this lesson. Then grade yourself with the rubric in README.md.
//! Where the new test goes: into the `tests` module of the file you fix (`graph.rs`
//! for 22, `glob.rs` for 25, `report.rs` for 30). For issue 30, `tests/report.rs` is
//! fine too, and the test must fail on every run before your fix, not on every
//! other one.
//! EXPLAIN afterward: Tell the story of your contribution in five sentences: the
//! report, the reproduction, the cause, the fix, and how you know nothing else
//! changed.

/// The walk of `tasklane_core::graph::plan` as released in 0.4.0, reduced to a
/// counter: how often is the bottom task `l0` of a ladder visited? A ladder with
/// `levels` levels has the tasks `l0`, `r0`, and for each level `n` from 1 to
/// `levels` the tasks `ln` and `rn`, which both depend on `l(n-1)` and `r(n-1)`. The
/// target is the top-left task. Do not edit this function.
pub fn v0_4_0_visits_of_the_bottom_task(levels: u32) -> u64 {
    // In 0.4.0 a task's dependencies are walked every time the task is reached.
    fn walk(level: u32, bottom_visits: &mut u64) {
        if level == 0 {
            *bottom_visits += 1;
            return;
        }
        // The left and the right task of the level below; only the left one's visits
        // of `l0` are counted separately, so count both walks and halve at the end.
        walk(level - 1, bottom_visits);
        walk(level - 1, bottom_visits);
    }
    let mut both_bottom_tasks = 0;
    walk(levels, &mut both_bottom_tasks);
    both_bottom_tasks / 2
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::{count_tests, mentions_issue, read, require_section, section};
    // Unused once every prediction is answered.
    #[allow(unused_imports)]
    use crate::guess;
    use crate::l06_writing::commit_problems;
    use crate::within;
    use tasklane_core::glob::matches;
    use tasklane_core::{
        Error, Registry, RunOptions, Runner, TaskStatus, Vars, Workspace, parse, plan,
    };

    /// A Lanefile with a ladder of diamonds: every level depends on both tasks of
    /// the level below.
    fn ladder(levels: u32) -> String {
        let mut source = String::from("task l0\ntask r0\n");
        for level in 1..=levels {
            let below = level - 1;
            source.push_str(&format!(
                "task l{level} : l{below} r{below}\ntask r{level} : l{below} r{below}\n"
            ));
        }
        source
    }

    // ---- Issue 22: planning a ladder of dependencies never finishes ----

    #[test]
    fn issue_22_a_ladder_is_planned_in_reasonable_time() {
        let order = within(10, "planning a ladder of 48 levels", || {
            let file = parse(&ladder(48)).unwrap();
            plan(&file, "l48").unwrap()
        });
        assert_eq!(
            order.len(),
            97,
            "every task of the ladder below the target, once"
        );
        assert_eq!(order[..4], ["l0", "r0", "l1", "r1"]);
        assert_eq!(order.last().map(String::as_str), Some("l48"));
    }

    #[test]
    fn issue_22_a_ladder_runs_in_reasonable_time() {
        let done = within(10, "running a ladder of 48 levels", || {
            let file = parse(&ladder(48)).unwrap();
            let registry = Registry::with_builtins();
            let mut ws = Workspace::new();
            let report = Runner::new(&registry)
                .run(&file, "l48", &Vars::new(), &mut ws)
                .unwrap();
            report.count(TaskStatus::Done)
        });
        assert_eq!(done, 97);
    }

    #[test]
    fn issue_22_unchanged_order_is_still_the_written_order() {
        let file = parse("task z\ntask a\ntask m : a z\ntask top : z m a\n").unwrap();
        assert_eq!(
            plan(&file, "top").unwrap(),
            ["z", "a", "m", "top"],
            "dependencies are visited in the order they are written, each task once"
        );
        let file = parse("task base\ntask l : base\ntask r : base\ntask top : l r\n").unwrap();
        assert_eq!(plan(&file, "top").unwrap(), ["base", "l", "r", "top"]);
    }

    #[test]
    fn issue_22_unchanged_cycles_are_still_found_with_their_path() {
        let cycle =
            |names: &[&str]| Err(Error::Cycle(names.iter().map(|n| n.to_string()).collect()));
        let file = parse("task a : b c\ntask b\ntask c : a\n").unwrap();
        assert_eq!(
            plan(&file, "a"),
            cycle(&["a", "c", "a"]),
            "a cycle that is reached after a finished dependency must still be reported"
        );
        let file =
            parse("task top : x y\ntask x : shared\ntask y : shared\ntask shared : y\n").unwrap();
        assert_eq!(
            plan(&file, "top"),
            cycle(&["shared", "y", "shared"]),
            "the path starts at the first task of the cycle, not at the target"
        );
        let file = parse("task a : a\n").unwrap();
        assert_eq!(plan(&file, "a"), cycle(&["a", "a"]));
        let file = parse("task a\n").unwrap();
        assert_eq!(
            plan(&file, "nope"),
            Err(Error::UnknownTask("nope".to_string()))
        );
    }

    // ---- Issue 25: `concat docs/*.md` skips some files ----

    #[test]
    fn issue_25_star_gives_back_characters_the_rest_needs() {
        for (pattern, path) in [
            ("*.md", "notes.md.md"),
            ("docs/*.md", "docs/2026.01.md"),
            ("a*b", "aXbYb"),
            ("*a*b", "aab"),
            ("*?", "ab"),
            ("*??", "ab"),
            ("*-*-*.txt", "a-b-c-d.txt"),
            ("*aab", "aaab"),
        ] {
            assert!(matches(pattern, path), "`{pattern}` must match `{path}`");
        }
    }

    #[test]
    fn issue_25_the_reported_lanefile_works() {
        let source = "task docs\n\
                      \x20 step write docs/intro.md A\n\
                      \x20 step write docs/notes.md.md B\n\
                      \x20 step write docs/v1.2.md C\n\
                      \x20 step write docs/skip.txt D\n\
                      \x20 step concat all.md \"docs/*.md\"\n";
        let file = parse(source).unwrap();
        let registry = Registry::with_builtins();
        let mut ws = Workspace::new();
        Runner::new(&registry)
            .run(&file, "docs", &Vars::new(), &mut ws)
            .unwrap();
        assert_eq!(ws.read("all.md"), Some("A\nB\nC"));
    }

    #[test]
    fn issue_25_unchanged_patterns_do_not_match_more_than_before() {
        for (pattern, path) in [
            ("*.md", "notes.md.txt"),
            ("*.md", "docs/notes.md"),
            ("docs/*.md", "docs/api/notes.md"),
            ("a*b/c", "aXb/Yb/c"),
            ("*", "a/b"),
            ("a?b", "a/b"),
            ("v?", "v"),
            ("v?", "v12"),
            ("*x", "abc"),
            ("a*", "ba"),
            ("", "a"),
        ] {
            assert!(
                !matches(pattern, path),
                "`{pattern}` must not match `{path}`: `*` and `?` never match a `/`, \
                 and the whole path must match"
            );
        }
        for (pattern, path) in [
            ("", ""),
            ("*", ""),
            ("**", "abc"),
            ("build/*", "build/out.txt"),
            ("v?.txt", "v\u{e9}.txt"),
            ("a*/b*", "a1/b2"),
        ] {
            assert!(
                matches(pattern, path),
                "`{pattern}` must still match `{path}`"
            );
        }
    }

    // ---- Issue 30: `summary_names_failed_tasks` fails every other run ----

    #[test]
    fn issue_30_summary_lists_failures_in_plan_order_every_time() {
        let source = "task zeta\n  step fail z\n\
                      task alpha\n  step fail a\n\
                      task mid\n  step fail m\n\
                      task beta\n  step fail b\n\
                      task ci : zeta alpha mid beta\n";
        let file = parse(source).unwrap();
        let registry = Registry::with_builtins();
        // One hundred runs in one process. Four failures have 24 possible orders, so
        // an order that depends on chance cannot survive this.
        for run in 0..100 {
            let mut ws = Workspace::new();
            let report = Runner::new(&registry)
                .with_options(RunOptions::new().keep_going(true))
                .run(&file, "ci", &Vars::new(), &mut ws)
                .unwrap();
            assert_eq!(
                report.summary(),
                "0 done, 4 failed, 1 skipped; failed: zeta (fail: z), alpha (fail: a), mid (fail: m), beta (fail: b)",
                "run {run}: the documentation of `summary` promises plan order"
            );
        }
    }

    #[test]
    fn issue_30_the_fixture_test_still_checks_the_exact_line() {
        let test = read("fixture/crates/tasklane-core/tests/report.rs");
        assert!(
            test.contains(
                "\"1 done, 2 failed, 1 skipped; failed: lint (fail: style), test (fail: 2 tests failed)\""
            ),
            "the flaky test was right and the code was wrong: keep its exact assertion"
        );
        assert!(
            !test.contains("#[ignore"),
            "an ignored test protects nothing; fix the cause instead"
        );
    }

    #[test]
    fn issue_30_unchanged_summary_without_failures() {
        let file = parse("task a\n  step echo hi\ntask b : a\n").unwrap();
        let registry = Registry::with_builtins();
        let mut ws = Workspace::new();
        let report = Runner::new(&registry)
            .run(&file, "b", &Vars::new(), &mut ws)
            .unwrap();
        assert_eq!(report.summary(), "2 done, 0 failed, 0 skipped");
    }

    // ---- The capstone itself ----

    /// The issue number after `Fixes #` in the pull request description.
    fn chosen_issue(pr: &str) -> u32 {
        let number: String = pr
            .split("Fixes #")
            .nth(1)
            .unwrap_or_else(|| {
                panic!("answers/l10_pr.md must say `Fixes #N` for the issue you chose")
            })
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        number
            .parse()
            .unwrap_or_else(|_| panic!("answers/l10_pr.md: write the issue number after `Fixes #`"))
    }

    #[test]
    fn z_chosen_issue_is_ready_to_merge() {
        let pr = read("answers/l10_pr.md");
        for heading in ["## What", "## Why", "## Testing", "## Checklist"] {
            require_section("answers/l10_pr.md", &pr, heading);
        }
        let testing = require_section("answers/l10_pr.md", &pr, "## Testing");
        assert!(
            testing.contains("cargo test"),
            "the pull request must say which command shows that the fix works"
        );
        let checklist = require_section("answers/l10_pr.md", &pr, "## Checklist");
        assert!(
            checklist.matches("- [x]").count() >= 4 && !checklist.contains("- [ ]"),
            "tick every item of the checklist after you have done it"
        );

        let issue = chosen_issue(&pr);
        // The file that should hold the regression test, the number of tests it had
        // in 0.4.0, and the area that the commit subject must name.
        let (test_files, tests_before, area): (&[&str], usize, &str) = match issue {
            22 => (&["fixture/crates/tasklane-core/src/graph.rs"], 6, "plan"),
            25 => (&["fixture/crates/tasklane-core/src/glob.rs"], 4, "glob"),
            30 => (
                &[
                    "fixture/crates/tasklane-core/src/report.rs",
                    "fixture/crates/tasklane-core/tests/report.rs",
                ],
                5,
                "report",
            ),
            other => panic!("issue {other} is not one of the three capstone issues: 22, 25, 30"),
        };

        // The acceptance tests of the chosen issue.
        match issue {
            22 => {
                issue_22_a_ladder_is_planned_in_reasonable_time();
                issue_22_a_ladder_runs_in_reasonable_time();
                issue_22_unchanged_order_is_still_the_written_order();
                issue_22_unchanged_cycles_are_still_found_with_their_path();
            }
            25 => {
                issue_25_star_gives_back_characters_the_rest_needs();
                issue_25_the_reported_lanefile_works();
                issue_25_unchanged_patterns_do_not_match_more_than_before();
            }
            _ => {
                issue_30_summary_lists_failures_in_plan_order_every_time();
                issue_30_the_fixture_test_still_checks_the_exact_line();
                issue_30_unchanged_summary_without_failures();
            }
        }

        let tests_now: usize = test_files.iter().map(|path| count_tests(&read(path))).sum();
        assert!(
            tests_now > tests_before,
            "the fixture needs a regression test for issue {issue} in {}",
            test_files.join(" or ")
        );

        let changelog = read("fixture/CHANGELOG.md");
        let unreleased = section(&changelog, "## [Unreleased]")
            .expect("fixture/CHANGELOG.md has a `## [Unreleased]` section");
        let fixed = section(unreleased, "### Fixed").unwrap_or("");
        assert!(
            mentions_issue(fixed, issue),
            "add an entry with `(#{issue})` under `### Fixed` in `## [Unreleased]`"
        );

        let commit = read("answers/l10_commit.txt");
        assert!(
            !commit.contains("TODO"),
            "answers/l10_commit.txt still contains a TODO"
        );
        let problems = commit_problems(&commit);
        assert!(
            problems.is_empty(),
            "answers/l10_commit.txt breaks rules of fixture/CONTRIBUTING.md:\n- {}",
            problems.join("\n- ")
        );
        let subject = commit
            .lines()
            .find(|line| !line.starts_with('#'))
            .unwrap_or("");
        assert!(
            subject.starts_with(&format!("{area}: ")),
            "the commit subject must start with the area of the fix, `{area}: `"
        );
        assert!(
            mentions_issue(&commit, issue),
            "the commit message must close the same issue as the pull request: `Fixes #{issue}`"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Issue 22. In 0.4.0, how often is the bottom task `l0` visited when a
        // ladder with 10 levels is planned? Read `visit` in the fixture's graph.rs.
        let p1: u64 = guess();
        assert_eq!(
            p1,
            v0_4_0_visits_of_the_bottom_task(10),
            "P1: every level doubles the number of visits of the level below"
        );

        // P2. And for the 48 levels of the acceptance test?
        let p2: u64 = guess();
        assert_eq!(
            p2,
            // The same doubling, computed directly: 48 levels would take the counter
            // above longer than you want to wait.
            (11..=48).fold(v0_4_0_visits_of_the_bottom_task(10), |visits, _| visits * 2),
            "P2: you may write the answer as a power of two, for example `1 << 20`"
        );

        // P3. Issue 30. In how many different orders can a hash map hand out four
        // failed tasks?
        let p3: usize = guess();
        assert_eq!(
            p3,
            (1..=4).product::<usize>(),
            "P3: the orders of four things"
        );
    }
}
