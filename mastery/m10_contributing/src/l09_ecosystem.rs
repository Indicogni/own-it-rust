//! LESSON 09 — Contributing to the wider ecosystem
//! Run: cargo test --manifest-path mastery/m10_contributing/Cargo.toml l09_
//! Read first: CONCEPTS.md, "Lesson 09". Every answer in this lesson is in a primary
//! source that the section links: the RFC repository, the rustc-dev-guide, the
//! std-dev-guide, the Rust Forge, the Clippy book. Read the source, not a summary:
//! processes change, and the linked pages are where they change first.
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer) from what you believe today. Then look each one up.
//! IMPLEMENT: A. the process facts below. B. the LAB: build Clippy from source, run
//! its tests, and record what you saw in `answers/l09_lab.md`. C. the plan for a
//! first real contribution, in NOTES.md (not checked by a test). Preserve the tests.
//! EXPLAIN afterward: Describe the path of a new standard-library function from an
//! idea to a stable release, naming each document or issue that is created on the
//! way. Then say which of the project's rules surprised you.

/// A. Facts about how the Rust project is run. Give short answers in the format the
/// doc comment asks for.
pub mod process {
    /// The RFC repository README: how many calendar days does a final comment period
    /// (FCP) last?
    pub fn fcp_days() -> u32 {
        todo!("Implement this task")
    }

    /// The Forge, "Issue Assignment": the comment that assigns an issue to yourself.
    /// Give the whole comment, starting with `@`.
    pub fn claim_comment() -> &'static str {
        todo!("Implement this task")
    }

    /// The Forge, "Shortcuts": after you have pushed the changes a reviewer asked
    /// for, the comment that puts your pull request back into the review queue.
    pub fn ready_comment() -> &'static str {
        todo!("Implement this task")
    }

    /// The rustc-dev-guide: the name of the tool that builds and tests every crate on
    /// crates.io to measure the breakage of a compiler change. One word.
    pub fn ecosystem_test_tool() -> &'static str {
        todo!("Implement this task")
    }

    /// The std-dev-guide, "The feature lifecycle": in which repository do you open an
    /// API Change Proposal (ACP)? Answer as `owner/name`.
    pub fn acp_repository() -> &'static str {
        todo!("Implement this task")
    }

    /// Clippy's CONTRIBUTING.md: every pull request description must contain a line
    /// that starts with this word and a colon. Give the word with the colon.
    pub fn clippy_pr_line() -> &'static str {
        todo!("Implement this task")
    }

    /// The editions that the pinned compiler accepts as stable, oldest first,
    /// separated by one space. `rustc --help -v` lists them under `--edition`; the
    /// word `future` in that list is not an edition.
    pub fn stable_editions() -> &'static str {
        todo!("Implement this task")
    }
}

/// What the LLM usage policy of `rust-lang/rust` says about a way of using a large
/// language model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmRule {
    /// Allowed, and nobody needs to be told.
    Allowed,
    /// Allowed only if you disclose that an LLM was used.
    AllowedWithDisclosure,
    /// Not allowed.
    Banned,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::{assert_answer, digest, read};
    // Unused once every prediction is answered.
    #[allow(unused_imports)]
    use crate::guess;

    #[test]
    fn a_process_facts() {
        assert_answer(
            "A, fcp_days",
            &process::fcp_days().to_string(),
            0x07f8_9207_b4ba_08a4,
        );
        assert_answer(
            "A, claim_comment",
            process::claim_comment(),
            0x3239_ca68_2e9f_11aa,
        );
        assert_answer(
            "A, ready_comment",
            process::ready_comment(),
            0x67a7_f62f_55a5_2185,
        );
        assert_answer(
            "A, ecosystem_test_tool",
            process::ecosystem_test_tool(),
            0xc1c2_bdc6_da7b_f296,
        );
        assert_answer(
            "A, acp_repository",
            process::acp_repository(),
            0xbbd4_aacb_cf41_2767,
        );
        assert_answer(
            "A, clippy_pr_line",
            process::clippy_pr_line(),
            0xf281_0299_b4a0_b9f9,
        );
        assert_answer(
            "A, stable_editions",
            process::stable_editions(),
            0x5b20_d273_8c98_7841,
        );
    }

    /// The value after `label:` in the lab notes.
    fn field<'a>(notes: &'a str, label: &str) -> &'a str {
        notes
            .lines()
            .find_map(|line| line.trim().strip_prefix(label))
            .unwrap_or_else(|| {
                panic!("B: answers/l09_lab.md has no line that starts with `{label}`")
            })
            .trim()
    }

    #[test]
    fn b_lab_notes_record_a_real_run() {
        let notes = read("answers/l09_lab.md");
        let commit = field(&notes, "Commit:");
        assert!(
            commit.len() == 40 && commit.chars().all(|c| c.is_ascii_hexdigit()),
            "B: `Commit:` must be the full 40-character hash from `git rev-parse HEAD`, not `{commit}`"
        );
        let toolchain = field(&notes, "Toolchain:");
        let date = toolchain.strip_prefix("nightly-").unwrap_or("");
        assert!(
            date.len() == 10 && date.chars().all(|c| c.is_ascii_digit() || c == '-'),
            "B: `Toolchain:` must be the channel from Clippy's rust-toolchain.toml, \
             like `nightly-2026-01-31`, not `{toolchain}`"
        );
        let result = field(&notes, "UI test result:");
        assert!(
            result.starts_with("test result: ok.") && result.contains("passed"),
            "B: `UI test result:` must be the `test result: ok. ...` line that \
             `cargo uitest` printed, not `{result}`"
        );
        let finding = field(&notes, "One thing I learned about the layout:");
        assert!(
            !finding.contains("TODO") && finding.split_whitespace().count() >= 8,
            "B: write one sentence about where something lives in the Clippy repository"
        );
    }

    /// P. Predict BEFORE you read the policy: replace each `guess()` with
    /// `LlmRule::Allowed`, `LlmRule::AllowedWithDisclosure` or `LlmRule::Banned`.
    /// Then read https://forge.rust-lang.org/policies/llm-usage.html and correct your
    /// answers. The model answers follow the policy text as it was in November 2026;
    /// if the page has changed since, the page is right and this test is out of date.
    #[test]
    fn p_predictions() {
        // P1. You ask an LLM to explain a module of rustc to you. Nobody else sees
        // the answer.
        let p1: LlmRule = guess();
        // P2. An LLM writes the description of your pull request and you post it
        // from your account.
        let p2: LlmRule = guess();
        // P3. An LLM writes the doc comment of a function you add.
        let p3: LlmRule = guess();
        // P4. An LLM fixes a typo in a comment for you, and you open a pull request
        // with that one-word change.
        let p4: LlmRule = guess();
        // P5. An LLM points you at a possible bug. You reproduce the bug yourself
        // and then report it in your own words.
        let p5: LlmRule = guess();

        let model = [
            0x8e9b_f3e7_4020_f6d2_u64,
            0x7028_68b3_7cd3_c259,
            0xcce6_6993_adf4_bd80,
            0x229a_a1ae_02ea_a864,
            0x1acf_56b4_3e28_4a5f,
        ];
        let topics = [
            "private use, where you are the only one who sees the output",
            "comments, issue bodies and pull request descriptions created by an LLM",
            "documentation created by an LLM, which includes doc comments",
            "trivial changes",
            "using an LLM to discover bugs that you verify personally",
        ];
        for (i, answer) in [p1, p2, p3, p4, p5].iter().enumerate() {
            assert!(
                digest(&format!("P{} {answer:?}", i + 1)) == model[i],
                "P{}: read what the policy says about {}",
                i + 1,
                topics[i]
            );
        }
    }
}
