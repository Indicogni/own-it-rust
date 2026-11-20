//! LESSON 05 — Code review, both sides
//! Run: cargo test --manifest-path mastery/m10_contributing/Cargo.toml l05_
//! Read first: CONCEPTS.md, "Lesson 05", then `reviews/pr-33/README.md`.
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: What is the difference between "I
//! would have written this differently" and "this must not be merged"?
//! IMPLEMENT: A. review the four patches of pull request 33 and answer the questions
//! about them. Write your own findings in NOTES.md first. B. judge the thirteen
//! comments in `reviews/pr-33/candidate-comments.md`. C. you are now the author:
//! act on `reviews/on-your-change.md` in the fixture and answer every comment in
//! `answers/l05_replies.md`. Preserve the tests.
//! EXPLAIN afterward: Pick the comment you found hardest to judge and explain your
//! verdict. Then explain why declining a request can be the helpful answer.

/// A. Which patch of pull request 33 (1 to 4) does each sentence describe?
pub mod which_patch {
    /// The patch that makes a test of the fixture fail, as `ci.txt` shows.
    pub fn turns_ci_red() -> u8 {
        1
    }

    /// The patch after which a mistake in a Lanefile can make tasklane panic.
    pub fn lets_a_user_mistake_panic() -> u8 {
        1
    }

    /// The patch that makes a failing test pass without removing the cause of the
    /// failure.
    pub fn hides_a_bug() -> u8 {
        2
    }

    /// The patch after which `downstream/pinned` would no longer compile.
    pub fn breaks_downstream_users() -> u8 {
        3
    }

    /// The patch that changes the result for some valid inputs while every test
    /// still passes.
    pub fn changes_results_silently() -> u8 {
        4
    }
}

/// Your judgement of a review comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Must be fixed before the merge: it breaks a merge requirement of the project.
    Blocking,
    /// True and worth saying, but the merge does not depend on it.
    NonBlocking,
    /// The comment is mistaken.
    Wrong,
}

/// B. Your verdict for each comment `'A'` to `'M'` of
/// `reviews/pr-33/candidate-comments.md`.
pub fn verdict(comment: char) -> Verdict {
    match comment {
        // A user can trigger both panics with a short Lanefile.
        'A' | 'B' => Verdict::Blocking,
        'C' => Verdict::NonBlocking,
        // The borrow of `ws` ends with the last use of `lines`, before `ws.write`.
        'D' => Verdict::Wrong,
        'E' => Verdict::Blocking,
        'F' => Verdict::Blocking,
        // The exact comparison was the only thing that pinned the documented order.
        'G' => Verdict::Wrong,
        'H' => Verdict::Blocking,
        // `&String` coerces to `&str` at a call site.
        'I' => Verdict::Wrong,
        'J' => Verdict::Blocking,
        'K' | 'M' => Verdict::NonBlocking,
        // Indexing a slice is safe: it is bounds-checked and panics, it is not UB.
        'L' => Verdict::Wrong,
        other => panic!("there is no comment {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::{assert_answer, digest, mentions_issue, read, section};
    // Unused once every prediction is answered.
    #[allow(unused_imports)]
    use crate::guess;

    #[test]
    fn a_which_patch() {
        assert_answer(
            "A, turns_ci_red",
            &format!("red {}", which_patch::turns_ci_red()),
            0xa24a_6df3_13e5_ffbf,
        );
        assert_answer(
            "A, lets_a_user_mistake_panic",
            &format!("panic {}", which_patch::lets_a_user_mistake_panic()),
            0x9b2b_213a_45bc_d2f7,
        );
        assert_answer(
            "A, hides_a_bug",
            &format!("hides {}", which_patch::hides_a_bug()),
            0x5269_da33_a16d_97f4,
        );
        assert_answer(
            "A, breaks_downstream_users",
            &format!("breaks {}", which_patch::breaks_downstream_users()),
            0xfec7_4d71_0e75_b0b0,
        );
        assert_answer(
            "A, changes_results_silently",
            &format!("silent {}", which_patch::changes_results_silently()),
            0x9311_833d_fe7d_20a8,
        );
    }

    #[test]
    fn b_verdicts() {
        let model = [
            ('A', 0x9ace_1cd2_6f67_d663_u64),
            ('B', 0xc8e0_4731_da73_c538),
            ('C', 0x81cb_2187_da8c_ea2a),
            ('D', 0x103f_c1ce_0cf1_5d92),
            ('E', 0x77bc_a06f_441f_d6ef),
            ('F', 0x9a00_b47d_3a06_8ee4),
            ('G', 0xe3a6_88fd_a960_e5af),
            ('H', 0x0529_3b70_cf46_0e8e),
            ('I', 0x570b_f182_f3ad_8d69),
            ('J', 0x5522_536a_1e99_9770),
            ('K', 0xed96_e952_2c08_5fa2),
            ('L', 0x0204_915f_2ccf_862a),
            ('M', 0xbe77_b542_bc3a_d3f4),
        ];
        let different: Vec<char> = model
            .iter()
            .filter(|(comment, model)| {
                digest(&format!("{comment} {:?}", verdict(*comment))) != *model
            })
            .map(|(comment, _)| *comment)
            .collect();
        assert!(
            different.is_empty(),
            "B: your verdict differs from the model review for {different:?}. Read the \
             patch line again, try the claim in a scratch file if you can, and compare \
             with the merge requirements in fixture/CONTRIBUTING.md."
        );
    }

    /// Runs the tasklane command line in this process.
    fn tasklane(source: &str, args: &[&str]) -> (u8, String) {
        let args: Vec<String> = args.iter().map(|arg| arg.to_string()).collect();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = tasklane_cli::run_with(&args, &|_| Ok(source.to_string()), &mut out, &mut err);
        (code, String::from_utf8(out).unwrap())
    }

    #[test]
    fn c_comment_1_dry_run_logs_what_would_run() {
        let source = "set OUT = build\n\
                      task clean\n  flags silent\n  step remove ${OUT}/*\n\
                      task page : clean\n  step write ${OUT}/a.txt \"two words\"\n  step upper ${OUT}/a.txt\n\
                      task all : page\n  step count ${OUT}/a.txt\n";
        let (code, out) = tasklane(source, &["run", "all", "--dry-run"]);
        assert_eq!(code, 0);
        assert_eq!(
            out,
            "[page] would run: write build/a.txt two words\n\
             [page] would run: upper build/a.txt\n\
             [all] would run: count build/a.txt\n\
             0 done, 0 failed, 3 skipped\n",
            "C1: one line per step, expanded arguments, plan order, nothing for silent tasks"
        );
        let (_, out) = tasklane(source, &["run", "all"]);
        assert!(
            !out.contains("would run"),
            "C1: a real run must log as before"
        );
    }

    #[test]
    fn c_comment_4_fixture_tests_combine_dry_run_and_keep_going() {
        let found = [
            "fixture/crates/tasklane-core/tests/run.rs",
            "fixture/crates/tasklane-core/src/run.rs",
            "fixture/crates/tasklane-cli/tests/cli.rs",
            "fixture/crates/tasklane-cli/src/args.rs",
        ]
        .iter()
        .map(|path| read(path))
        .any(|source| {
            source.split("#[test]").skip(1).any(|test| {
                (test.contains("dry_run") || test.contains("--dry-run") || test.contains("\"-n\""))
                    && (test.contains("keep_going")
                        || test.contains("--keep-going")
                        || test.contains("\"-k\""))
            })
        });
        assert!(
            found,
            "C4: no single test in the fixture uses the dry run together with keep-going"
        );
    }

    /// The `Action:` word and the `Reply:` text of one section of the replies file.
    fn reply(replies: &str, number: usize) -> (String, String) {
        let heading = format!("## Comment {number}");
        let body = section(replies, &heading)
            .unwrap_or_else(|| panic!("answers/l05_replies.md: `{heading}` is missing"));
        let action = body
            .lines()
            .find_map(|line| line.trim().strip_prefix("Action:"))
            .unwrap_or_else(|| {
                panic!("answers/l05_replies.md, comment {number}: no `Action:` line")
            })
            .trim()
            .to_lowercase();
        let text = body
            .split_once("Reply:")
            .map(|(_, text)| text.trim().to_string())
            .unwrap_or_else(|| panic!("answers/l05_replies.md, comment {number}: no `Reply:`"));
        (action, text)
    }

    #[test]
    fn c_every_comment_has_a_reply() {
        let replies = read("answers/l05_replies.md");
        for number in 1..=4 {
            let (action, text) = reply(&replies, number);
            assert!(
                ["done", "answered", "declined"].contains(&action.as_str()),
                "C: comment {number}: the action must be done, answered or declined, not `{action}`"
            );
            assert!(
                !text.contains("TODO") && text.split_whitespace().count() >= 12,
                "C: comment {number}: write a reply of at least a sentence or two; a \
                 reviewer cannot act on `ok` or `fixed`"
            );
        }
    }

    #[test]
    fn c_actions_fit_the_comments() {
        let replies = read("answers/l05_replies.md");
        let model = [
            0xddb8_fb1e_b331_0014_u64,
            0x7896_d9eb_093f_2890,
            0x4151_07c0_a1c5_c600,
            0xa443_26c7_1c71_8345,
        ];
        let hints = [
            "a maintainer states a condition for the merge",
            "the comment asks for reasoning and says no change is wanted",
            "the request is a breaking change that has nothing to do with your pull request",
            "a maintainer asks for a test",
        ];
        for number in 1..=4 {
            let (action, _) = reply(&replies, number);
            assert!(
                digest(&format!("{number} {action}")) == model[number - 1],
                "C: comment {number}: `{action}` does not fit. Think again: {}.",
                hints[number - 1]
            );
        }
        let (_, third) = reply(&replies, 3);
        assert!(
            mentions_issue(&third, 27),
            "C: comment 3: point to the issue where the idea is discussed, so that it is not lost"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer. All three
    /// are facts that a reviewer of pull request 33 should know.
    #[test]
    fn p_predictions() {
        // P1. Patch 1 sorts lines with `sort()`:
        //     let mut lines = ["b", "a", "B"]; lines.sort(); lines.join(" ")
        let p1: &str = "B a b";
        let mut lines = ["b", "a", "B"];
        lines.sort();
        assert_eq!(p1, lines.join(" "), "P1: how `str` values are ordered");

        // P2. Patch 1 rebuilds the file with `text.lines()` and `join("\n")`. What is
        // the length of the result for the text "a\nb\n"?
        let p2: usize = 3;
        let text = "a\nb\n";
        let rebuilt = text.lines().collect::<Vec<_>>().join("\n");
        assert_eq!(
            p2,
            rebuilt.len(),
            "P2: what `lines` and `join` do to the last line end"
        );

        // P3. Patch 4 walks bytes instead of chars. For the path "v\u{e9}.txt"
        // (the second character is e with an acute accent): (bytes, chars).
        let p3: (usize, usize) = (7, 6);
        let path = "v\u{e9}.txt";
        assert_eq!(
            p3,
            (path.len(), path.chars().count()),
            "P3: bytes and characters of a non-ASCII path"
        );
    }
}
