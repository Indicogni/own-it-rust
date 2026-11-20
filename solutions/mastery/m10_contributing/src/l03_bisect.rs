//! LESSON 03 — Regression hunting with git bisect
//! Run: cargo test --manifest-path mastery/m10_contributing/Cargo.toml l03_
//! Read first: CONCEPTS.md, "Lesson 03". This lesson needs `git` on your PATH.
//!
//! LAB setup: create the practice repository. It is a scripted history of an early
//! tasklane, written to a directory outside the course repository:
//!
//! ```text
//! cargo run --manifest-path mastery/m10_contributing/Cargo.toml --bin make-history
//! ```
//!
//! The command prints the directory. Work there for this lesson; never run
//! `git bisect` inside the course repository.
//!
//! The report you are hunting: "With tasklane 0.1.0, `step write a.txt color=#ff0000`
//! wrote `color=#ff0000`. With 0.2.0 the file contains `color=`." The tag `v0.1.0` is
//! good and `v0.2.0` (the newest commit) is bad.
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: The test for the report does not exist
//! in the old commits. Where can it live so that every checkout sees it? What should
//! happen at a commit that does not compile?
//! IMPLEMENT the four tasks in order. Preserve the tests.
//! EXPLAIN afterward: Explain why the plain run blamed the wrong commit, what exit
//! code 125 means, and why bisecting needs a history in which most commits build.

/// A. In the practice repository, write the report as a test in the untracked file
/// `tests/probe.rs` (untracked files survive every checkout). Then run the obvious
/// command:
///
/// ```text
/// git bisect start v0.2.0 v0.1.0
/// git bisect run cargo test --test probe
/// git bisect reset
/// ```
///
/// Return the subject line of the commit that git calls "the first bad commit".
/// Look at that commit with `git show`: can it change what `parse` returns?
pub fn blamed_by_the_plain_run() -> &'static str {
    "refactor: move the planner into its own module (1/3)"
}

/// C. Run the bisection again with your script from task B
/// (`answers/l03_bisect.sh`; copy it into the practice repository or give its full
/// path): `git bisect run sh PATH/l03_bisect.sh`. Return the subject line of the
/// first bad commit.
pub fn first_bad_commit() -> &'static str {
    "parse: allow a comment at the end of a line"
}

/// D. How many commits after `v0.1.0` do not build? Find out with a loop over
/// `git rev-list v0.1.0..v0.2.0` that checks out each commit and runs `cargo build`,
/// not by reading the history generator.
pub fn commits_that_do_not_build() -> usize {
    2
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::{assert_answer, read};
    // Unused once every prediction is answered.
    #[allow(unused_imports)]
    use crate::guess;
    use crate::history::{self, COMMITS};

    #[test]
    fn a_commit_blamed_by_the_plain_run() {
        assert_answer("A", blamed_by_the_plain_run(), 0xdf6b_fc5b_03f1_e85f);
    }

    /// B. `answers/l03_bisect.sh` is the script for `git bisect run`. It must tell
    /// "this commit cannot be tested" apart from "this commit is bad".
    #[test]
    fn b_script_skips_commits_that_do_not_build() {
        let script = read("answers/l03_bisect.sh");
        assert!(
            !script.contains("TODO"),
            "B: answers/l03_bisect.sh still contains a TODO"
        );
        let build = script
            .find("cargo build")
            .or_else(|| script.find("cargo check"));
        let test = script.find("cargo test");
        assert!(
            script.contains("exit 125"),
            "B: the script never exits with the code that means `skip this commit`"
        );
        match (build, test) {
            (Some(build), Some(test)) => assert!(
                build < test,
                "B: build first, and skip if the build fails; only then run the test"
            ),
            _ => panic!(
                "B: the script needs a separate build step (`cargo build` or `cargo check`) \
                 and a `cargo test` step"
            ),
        }
    }

    #[test]
    fn c_first_bad_commit() {
        assert_ne!(
            first_bad_commit(),
            blamed_by_the_plain_run(),
            "C: this is the commit the plain run blamed; it only moves code"
        );
        assert_answer("C", first_bad_commit(), 0x2fe9_093d_028b_1be4);
    }

    #[test]
    fn d_commits_that_do_not_build() {
        assert_answer(
            "D",
            &commits_that_do_not_build().to_string(),
            0xaf63_af4c_8601_a015,
        );
    }

    /// The generator itself: it must produce the whole history, with the tags, in a
    /// directory of its own.
    #[test]
    fn g_history_builds_a_repository() {
        let dir = std::env::temp_dir().join(format!(
            "m10-history-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let built = history::build(&dir);
        let checks = built.and_then(|()| {
            let count = history::git_output(&dir, &["rev-list", "--count", "HEAD"])?;
            let tags = history::git_output(&dir, &["tag", "--list"])?;
            let status = history::git_output(&dir, &["status", "--porcelain"])?;
            Ok((count, tags, status))
        });
        let _ = std::fs::remove_dir_all(&dir);
        let (count, tags, status) = checks.unwrap_or_else(|message| panic!("{message}"));
        assert_eq!(
            count,
            COMMITS.len().to_string(),
            "one git commit per scripted commit"
        );
        assert_eq!(
            tags.lines().collect::<Vec<_>>(),
            ["v0.1.0", "v0.1.1", "v0.2.0"]
        );
        assert_eq!(status, "", "the work tree is clean after the last commit");
    }

    #[test]
    fn g_history_refuses_a_directory_inside_a_repository() {
        // `target` lies inside the course repository, or at least may do so; a
        // directory with a `.git` entry above it must always be refused.
        let outer = std::env::temp_dir().join(format!("m10-history-guard-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&outer);
        std::fs::create_dir_all(outer.join(".git")).unwrap();
        let result = history::build(&outer.join("inner"));
        let _ = std::fs::remove_dir_all(&outer);
        let message = result.expect_err("a directory below a `.git` entry must be refused");
        assert!(message.contains("inside a git repository"), "{message}");
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. The history has 40 commits. `v0.1.0` is the 8th and is good; the 40th is
        // bad. How many commits can be the first bad one?
        let p1: usize = 32;
        let good = COMMITS
            .iter()
            .position(|commit| commit.tag == Some("v0.1.0"))
            .unwrap();
        let candidates = COMMITS.len() - 1 - good;
        assert_eq!(
            p1, candidates,
            "P1: the commits after the good one, up to the bad one"
        );

        // P2. Each test halves the candidates. How many commits must git test, at
        // most, to find the first bad one among them when none is skipped?
        let p2: u32 = 5;
        assert_eq!(
            p2,
            candidates.next_power_of_two().trailing_zeros(),
            "P2: the base-2 logarithm of the number of candidates, rounded up"
        );

        // P3. Which exit code of the script tells `git bisect run` to skip the commit?
        let p3: i32 = 125;
        assert_eq!(
            p3,
            history::SKIP_EXIT_CODE,
            "P3: the skip code of git bisect run"
        );
    }
}
