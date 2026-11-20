//! LESSON 06 — Writing for maintainers
//! Run: cargo test --manifest-path mastery/m10_contributing/Cargo.toml l06_
//! Read first: CONCEPTS.md, "Lesson 06", then the sections "Commit messages" and
//! "Pull requests" of `fixture/CONTRIBUTING.md`. The tests check the project's rules,
//! not general taste: in a real project you follow the house style, too.
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: Who reads a commit message, and when?
//! What does a maintainer need from an issue report to act without asking back?
//! IMPLEMENT: there is no `todo!()` here. Write three texts about your fix from
//! lesson 02: A. the commit message, in `answers/l06_commit.txt`; B. the pull request
//! description, in `answers/l06_pr.md`; C. the issue report that the reporter of
//! issue 14 should have written, in `answers/l06_issue.md`. Preserve the tests.
//! EXPLAIN afterward: Explain what belongs in the commit message and what belongs
//! only in the pull request. Then name the two sentences of your issue report that
//! save the maintainer the most time.

/// The problems of a commit message under the rules of `fixture/CONTRIBUTING.md`.
/// An empty list means the message follows every rule that a program can check.
/// Lines that start with `#` are ignored, as git ignores them.
pub fn commit_problems(message: &str) -> Vec<&'static str> {
    let lines: Vec<&str> = message
        .lines()
        .filter(|line| !line.starts_with('#'))
        .collect();
    let subject = lines.first().copied().unwrap_or("").trim_end();
    let mut problems = Vec::new();

    if subject.chars().count() > 60 {
        problems.push("the subject is longer than 60 characters");
    }
    let summary = match subject.split_once(": ") {
        Some((area, summary))
            if !area.is_empty()
                && area.chars().all(|c| {
                    c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_'
                }) =>
        {
            summary
        }
        _ => {
            problems.push("the subject does not start with an area and a colon, like `vars: ...`");
            subject
        }
    };
    if subject.ends_with('.') {
        problems.push("the subject ends with a full stop");
    }
    let verb = summary
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_lowercase();
    // English has imperatives that end like a past tense or an `-ing` form. A check
    // of the ending alone would reject `plan: speed up ...` or `cli: bring back ...`.
    let imperative_anyway = [
        "bring", "embed", "exceed", "feed", "need", "proceed", "seed", "shed", "speed", "string",
        "succeed",
    ]
    .contains(&verb.as_str());
    if ((verb.ends_with("ed") || verb.ends_with("ing")) && !imperative_anyway)
        || ["fixes", "adds", "changes", "updates", "removes"].contains(&verb.as_str())
    {
        problems.push("the summary does not start with an imperative verb (`fix`, not `fixed`, `fixes` or `fixing`)");
    }
    if lines.get(1).is_some_and(|line| !line.trim().is_empty()) {
        problems.push("the second line is not blank");
    }
    let is_trailer = |line: &str| line.trim().starts_with("Fixes #");
    let body: Vec<&str> = lines
        .iter()
        .skip(1)
        .copied()
        .filter(|line| !is_trailer(line))
        .collect();
    if body
        .iter()
        .map(|line| line.split_whitespace().count())
        .sum::<usize>()
        < 15
    {
        problems.push("the body does not explain what was wrong and why the change is right");
    }
    if body.iter().any(|line| line.chars().count() > 72) {
        problems.push("a body line is longer than 72 characters");
    }
    if !lines.iter().any(|line| {
        line.trim()
            .strip_prefix("Fixes #")
            .is_some_and(|number| !number.is_empty() && number.chars().all(|c| c.is_ascii_digit()))
    }) {
        problems.push("there is no `Fixes #N` line for the issue");
    }
    problems
}

/// A commit message somebody wrote for the fix of issue 14.
pub const SAMPLE_1: &str = "Fixed the dollar bug.\nSee issue 14\n";

/// Another one.
pub const SAMPLE_2: &str = "vars: keep the character after a lone dollar sign in expand so that prices survive\n\nFixes #14\n";

/// And a third.
pub const SAMPLE_3: &str = "vars: fixing lone dollar signs\n\nexpand() dropped the character after a `$` that does not start a variable or an escape, so `$3` became `$`. The character is kept now.\n";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::{mentions_issue, read, require_section};
    // Unused once every prediction is answered.
    #[allow(unused_imports)]
    use crate::guess;

    #[test]
    fn a_commit_message_follows_the_project_rules() {
        let message = read("answers/l06_commit.txt");
        assert!(
            !message.contains("TODO"),
            "A: answers/l06_commit.txt still contains a TODO"
        );
        let problems = commit_problems(&message);
        assert!(
            problems.is_empty(),
            "A: answers/l06_commit.txt breaks rules of fixture/CONTRIBUTING.md:\n- {}",
            problems.join("\n- ")
        );
    }

    #[test]
    fn a_commit_message_is_about_issue_14() {
        let message: String = read("answers/l06_commit.txt")
            .lines()
            .filter(|line| !line.starts_with('#'))
            .map(|line| format!("{line}\n"))
            .collect();
        assert!(
            mentions_issue(&message, 14),
            "A: the message must close the issue it fixes: `Fixes #14`"
        );
        assert!(
            message.lines().next().unwrap_or("").starts_with("vars: "),
            "A: the area of this fix is the module you changed"
        );
        assert!(
            message.lines().skip(1).any(|line| line.contains('$')),
            "A: the body must show the input that went wrong, so that somebody who \
             meets the bug can find this commit by searching the log"
        );
    }

    #[test]
    fn b_pull_request_has_every_section_of_the_template() {
        let pr = read("answers/l06_pr.md");
        for heading in ["## What", "## Why", "## Testing", "## Checklist"] {
            require_section("answers/l06_pr.md", &pr, heading);
        }
        assert!(
            mentions_issue(&pr, 14),
            "B: the description must reference the issue, for example `Fixes #14`"
        );
    }

    #[test]
    fn b_pull_request_says_how_the_change_was_tested() {
        let pr = read("answers/l06_pr.md");
        let testing = require_section("answers/l06_pr.md", &pr, "## Testing");
        assert!(
            testing.contains("cargo test"),
            "B: `## Testing` must name the command you ran, so that the reviewer can run it too"
        );
        assert!(
            testing.split_whitespace().count() >= 15,
            "B: `## Testing` must also say what the new test checks and what you tried by hand"
        );
        let checklist = require_section("answers/l06_pr.md", &pr, "## Checklist");
        let ticked = checklist.matches("- [x]").count();
        let open = checklist.matches("- [ ]").count();
        assert!(
            ticked >= 4 && open == 0,
            "B: do every item of the checklist, then tick it: {ticked} ticked, {open} open"
        );
    }

    #[test]
    fn c_issue_report_has_a_specific_title() {
        let issue = read("answers/l06_issue.md");
        let title = issue
            .lines()
            .find_map(|line| line.strip_prefix("# "))
            .expect("C: answers/l06_issue.md starts with a `# Title` line")
            .trim();
        assert!(!title.contains("TODO"), "C: write the title");
        assert!(
            title.chars().count() <= 72,
            "C: the title has {} characters; at most 72 fit in a list of issues",
            title.chars().count()
        );
        let lower = title.to_lowercase();
        for vague in [
            "wrong",
            "broken",
            "not work",
            "doesn't work",
            "weird",
            "help",
            "bug",
        ] {
            assert!(
                !lower.contains(vague),
                "C: `{vague}` in a title says nothing. Name the input and what happens to it."
            );
        }
        assert!(
            title.contains('$') || lower.contains("dollar"),
            "C: the title must name the thing that triggers the problem"
        );
    }

    #[test]
    fn c_issue_report_can_be_reproduced_from_the_text_alone() {
        let issue = read("answers/l06_issue.md");
        let steps = require_section("answers/l06_issue.md", &issue, "## Steps to reproduce");
        let blocks: Vec<&str> = steps.split("```").skip(1).step_by(2).collect();
        assert!(
            !blocks.is_empty(),
            "C: `## Steps to reproduce` needs a code block with the Lanefile and the command"
        );
        let code_lines: usize = blocks
            .iter()
            .map(|block| {
                block
                    .lines()
                    .skip(1)
                    .filter(|line| !line.trim().is_empty())
                    .count()
            })
            .sum();
        assert!(
            code_lines <= 8,
            "C: the reproduction has {code_lines} lines of code; a minimal one needs at most 8"
        );
        assert!(
            blocks.iter().any(|block| block.contains('$')),
            "C: the reproduction must contain the input that triggers the problem"
        );
        let expected = require_section("answers/l06_issue.md", &issue, "## Expected");
        let actual = require_section("answers/l06_issue.md", &issue, "## Actual");
        assert_ne!(
            expected, actual,
            "C: expected and actual result must differ"
        );
        let version = require_section("answers/l06_issue.md", &issue, "## Version");
        assert!(
            version.contains("0.4.0"),
            "C: `## Version` must give the tasklane version the reporter used"
        );
    }

    /// The checker itself: the ending of a word is not enough to call it a past tense.
    #[test]
    fn g_imperatives_that_end_in_ed_or_ing_are_accepted() {
        let body = "Planning a ladder of shared dependencies took time that doubled\n\
                    with every level, because each task was walked again and again.\n";
        for subject in [
            "plan: speed up planning of shared dependencies",
            "cli: bring back the default for --show",
            "docs: embed the example Lanefile",
        ] {
            let message = format!("{subject}\n\n{body}\nFixes #22\n");
            assert_eq!(
                commit_problems(&message),
                Vec::<&str>::new(),
                "`{subject}` starts with an imperative verb"
            );
        }
        for subject in ["plan: speeding up the planner", "plan: fixed the planner"] {
            let message = format!("{subject}\n\n{body}\nFixes #22\n");
            assert_eq!(
                commit_problems(&message).len(),
                1,
                "`{subject}` does not start with an imperative verb"
            );
        }
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer. For each
    /// sample message above, count the rules of `commit_problems` that it breaks.
    /// Count by reading the samples and the CONTRIBUTING rules, then check.
    #[test]
    fn p_predictions() {
        // P1. SAMPLE_1
        let p1: usize = 6;
        assert_eq!(
            p1,
            commit_problems(SAMPLE_1).len(),
            "P1: {:?}",
            commit_problems(SAMPLE_1)
        );

        // P2. SAMPLE_2
        let p2: usize = 2;
        assert_eq!(
            p2,
            commit_problems(SAMPLE_2).len(),
            "P2: {:?}",
            commit_problems(SAMPLE_2)
        );

        // P3. SAMPLE_3
        let p3: usize = 3;
        assert_eq!(
            p3,
            commit_problems(SAMPLE_3).len(),
            "P3: {:?}",
            commit_problems(SAMPLE_3)
        );
    }
}
