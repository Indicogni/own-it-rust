//! LESSON 08 — Capstone: the next release of a library that people already use
//! Run: cargo test --manifest-path mastery/m8_api/Cargo.toml l08_
//! Read first: "Lesson 08" in CONCEPTS.md (same package).
//!
//! `fixtures/l08/pacer/` is a library at version 1.0.0. It is yours: the
//! manifest, `src/lib.rs` and `CHANGELOG.md`. `fixtures/l08/users/` holds two
//! programs that are not yours:
//!
//! - `user_of_1_0` was written against 1.0.0. Its author is on holiday.
//! - `user_of_next` is waiting for your release and already uses it.
//!
//! Make the release that satisfies the second user without breaking the first.
//! Nobody gives you signatures: the requests below are prose, as they would be
//! in an issue tracker, and `user_of_next/src/lib.rs` shows how the new items
//! are used. Every tool of lessons 01 to 07 has its place here.
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then read `user_of_1_0/src/lib.rs` and
//! write down, for each request below, the obvious change that would break
//! that file.
//!
//! EXPLAIN afterward: Which of the five requests could not be fulfilled in the
//! way the requester first imagined, because of a decision that was made in
//! 1.0.0? For each, name the decision, and say what 1.0.0 should have done.
//!
//! # The requests
//!
//! 1. **A growth factor and a largest delay.** "Doubling is too fast for us, and
//!    no delay should be longer than 30 seconds." `Config` cannot take new
//!    fields (why?). Add a type `Policy` with private fields:
//!    - `Policy::new(retries: u32, base_ms: u64)` gives a policy with the growth
//!      factor 2 and no largest delay, or an error if `retries` is 0.
//!    - `with_factor(self, factor: u32)` gives the policy with another factor, or
//!      an error if `factor` is 0. The factor 1 means that all delays are equal.
//!    - `with_cap_ms(self, cap_ms: u64)` gives the policy with a largest delay.
//!      It cannot fail.
//!    - `retries()`, `base_ms()`, `factor()` and `cap_ms()` read the settings;
//!      `cap_ms()` is an `Option<u64>`.
//!    - `delays(&self)` returns an iterator over the delays as `u64`. The delays
//!      are computed one at a time; no vector is built. A delay that does not
//!      fit into a `u64` is `u64::MAX`; then the cap is applied. The return type
//!      is `Delays`, a public type of the library with private fields that
//!      implements `Iterator`, `Debug` and `Clone`. It is not `impl Iterator`:
//!      users store the iterator in their own structs, so they must be able to
//!      name its type (lesson 03).
//!    - `total_ms(&self) -> u64` is the sum of the delays, at most `u64::MAX`.
//!    - `Policy::try_from(config)` converts a `Config`.
//!    - `Policy` implements `Debug`, `Clone`, `Copy`, `PartialEq` and `Eq`.
//!
//! 2. **An error for policies.** The existing `Error` cannot take new variants
//!    (why?). Add `PolicyError`, a struct with the method
//!    `kind(&self) -> PolicyErrorKind`, and the enum `PolicyErrorKind` with the
//!    variants `ZeroRetries` and `ZeroFactor`. More kinds will come; make that
//!    possible. `PolicyError` implements `std::error::Error`, `Display`, `Debug`,
//!    `Clone`, `PartialEq` and `Eq`, and is `Send` and `Sync`. `PolicyErrorKind`
//!    implements `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` and `Hash`.
//!
//! 3. **A better name.** `get_total_ms` breaks the naming guideline C-GETTER.
//!    Add the free function `total_ms` with the same signature and behaviour.
//!    Deprecate the old name since this release, with a note that names
//!    `total_ms` in backticks.
//!
//! 4. **Elapsed time on every clock.** Add the method
//!    `elapsed_since(&self, start_ms: u64) -> u64` to the trait `Clock`: the
//!    milliseconds from `start_ms` to now, and 0 if `start_ms` is later.
//!
//! 5. **Release it.** Set the version in `Cargo.toml`: the smallest version
//!    number that is correct for this release. Add a section for it at the top
//!    of `CHANGELOG.md`, with the headings `### Added` and `### Deprecated`,
//!    that names every new item and the deprecated one. Every public item has
//!    documentation; `Policy` has an example that runs as a test; the library
//!    builds without warnings.
//!
//! Useful commands, from `fixtures/l08/users/`:
//!
//! ```text
//! cargo test --package user_of_1_0
//! cargo test --package user_of_next
//! ```

#[cfg(test)]
mod tests {
    use crate::support::{self, CargoRun};

    /// Runs `cargo ARGS` in the package `pacer`.
    fn pacer(args: &[&str], env: &[(&str, &str)]) -> CargoRun {
        support::cargo("fixtures/l08/pacer", "l08", args, env)
    }

    /// Runs `cargo ARGS` in the workspace of the two users.
    fn users(args: &[&str]) -> CargoRun {
        support::cargo("fixtures/l08/users", "l08", args, &[])
    }

    /// The part of a Markdown text from the line `heading` to the next heading
    /// of the same or a higher level.
    fn section<'a>(markdown: &'a str, heading: &str) -> Option<&'a str> {
        let level = heading.chars().take_while(|c| *c == '#').count();
        let start = markdown
            .lines()
            .scan(0, |offset, line| {
                let at = *offset;
                *offset += line.len() + 1;
                Some((at, line))
            })
            .find(|(_, line)| line.trim_end() == heading)
            .map(|(at, line)| at + line.len())?;
        let rest = &markdown[start.min(markdown.len())..];
        let mut end = rest.len();
        let mut offset = 0;
        for line in rest.split_inclusive('\n') {
            let hashes = line.chars().take_while(|c| *c == '#').count();
            if offset > 0 && hashes > 0 && hashes <= level && line[hashes..].starts_with(' ') {
                end = offset;
                break;
            }
            offset += line.len();
        }
        Some(&rest[..end])
    }

    #[test]
    fn a_markdown_sections_end_at_the_next_heading() {
        let text = "# Log\n\n## 1.1.0\n\n### Added\n\n- x\n\n### Deprecated\n\n- y\n\n## 1.0.0\n\n### Added\n\n- z\n";
        let release = section(text, "## 1.1.0").expect("the section exists");
        assert!(release.contains("- x") && release.contains("- y") && !release.contains("- z"));
        let added = section(release, "### Added").expect("the subsection exists");
        assert!(added.contains("- x") && !added.contains("- y"));
        assert_eq!(section(text, "## 2.0.0"), None);
    }

    #[test]
    fn a_the_user_of_1_0_still_builds_and_passes() {
        let run = users(&["test", "--quiet", "--package", "user_of_1_0"]);
        assert!(
            run.ok,
            "fixtures/l08/users/user_of_1_0 no longer builds or passes. It was correct \
             for pacer 1.0.0, so a change of yours is a breaking change:\n{}",
            support::head(&run.all(), 60)
        );
    }

    #[test]
    fn b_the_user_of_the_next_release_builds_and_passes() {
        let run = users(&["test", "--quiet", "--package", "user_of_next"]);
        assert!(
            run.ok,
            "fixtures/l08/users/user_of_next does not build or pass yet. Its source \
             shows how the new items are used:\n{}",
            support::head(&run.all(), 60)
        );
    }

    #[test]
    fn c_the_old_name_is_deprecated_with_a_pointer_to_the_new_one() {
        let source = support::read("fixtures/l08/pacer/src/lib.rs");
        let attribute: Option<String> = source
            .split("#[deprecated(")
            .nth(1)
            .and_then(|rest| rest.split(")]").next())
            .map(|inner| inner.split_whitespace().collect());
        let attribute =
            attribute.expect("`get_total_ms` must carry #[deprecated(since = .., note = ..)]");
        assert!(
            attribute.contains("since=\"1.1.0\""),
            "the deprecation starts with this release: since = \"1.1.0\""
        );
        // The first user still calls the old name, and must be told what to do.
        let run = users(&["check", "--package", "user_of_1_0"]);
        assert!(
            run.ok,
            "user_of_1_0 must still build:\n{}",
            support::head(&run.stderr, 40)
        );
        // `total_ms` in backticks, alone or at the end of a path.
        let told = run.stderr.lines().any(|line| {
            line.contains("use of deprecated")
                && line.contains("get_total_ms")
                && (line.contains("`total_ms`") || line.contains("::total_ms`"))
        });
        assert!(
            told,
            "user_of_1_0 calls `get_total_ms` and must get a warning that names \
             `total_ms`. Cargo printed:\n{}",
            support::head(&run.stderr, 40)
        );
    }

    #[test]
    fn d_the_version_is_the_smallest_correct_one() {
        let run = pacer(&["metadata", "--no-deps", "--format-version", "1"], &[]);
        assert!(
            run.ok,
            "cargo cannot read the manifest of pacer:\n{}",
            run.stderr
        );
        let version = support::json_string(&run.stdout, "version").unwrap_or_default();
        support::assert_answer(
            "version of the release",
            &version,
            0x38a9_cdf7_8ab8_baac,
            "Does the release break a user of 1.0.0? Does it add something? \
             Lesson 01 has the rule.",
        );
    }

    #[test]
    fn d_the_changelog_describes_the_release() {
        let changelog = support::read("fixtures/l08/pacer/CHANGELOG.md");
        let run = pacer(&["metadata", "--no-deps", "--format-version", "1"], &[]);
        let version = support::json_string(&run.stdout, "version").unwrap_or_default();
        assert_ne!(
            version, "1.0.0",
            "Cargo.toml still says 1.0.0. Choose the version of the release first; \
             the changelog must have a section for exactly that version"
        );
        let heading = format!("## {version}");
        let release = section(&changelog, &heading).unwrap_or_else(|| {
            panic!("CHANGELOG.md has no section `{heading}` for the version in Cargo.toml")
        });
        assert!(
            changelog.find(&heading) < changelog.find("## 1.0.0"),
            "the newest release comes first in a changelog"
        );
        let added = section(release, "### Added").expect("the release needs a part `### Added`");
        for item in [
            "Policy",
            "Delays",
            "PolicyError",
            "total_ms",
            "elapsed_since",
        ] {
            assert!(added.contains(item), "`### Added` must mention `{item}`");
        }
        let deprecated =
            section(release, "### Deprecated").expect("the release needs a part `### Deprecated`");
        assert!(
            deprecated.contains("get_total_ms") && deprecated.contains("total_ms"),
            "`### Deprecated` must name `get_total_ms` and its replacement"
        );
    }

    #[test]
    fn e_the_library_is_documented_and_free_of_warnings() {
        let env = [("RUSTFLAGS", "-D warnings -D missing-docs")];
        let run = pacer(&["check", "--quiet"], &env);
        assert!(
            run.ok,
            "pacer must build without warnings, and every public item needs \
             documentation:\n{}",
            support::head(&run.stderr, 50)
        );
    }

    #[test]
    fn e_the_examples_in_the_documentation_run() {
        let run = pacer(&["test", "--doc"], &[]);
        assert!(
            run.ok,
            "`cargo test --doc` failed for pacer:\n{}",
            support::head(&run.all(), 60)
        );
        // On Windows rustdoc prints the file as `src\lib.rs`.
        let stdout = run.stdout.replace('\\', "/");
        assert!(
            stdout
                .lines()
                .any(|line| line.starts_with("test src/lib.rs - Policy") && line.ends_with("ok")),
            "`Policy` needs an example in its documentation that runs as a test"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    #[test]
    fn p_predictions() {
        // P1. Suppose the release removed `get_total_ms` and did not deprecate
        // it. What would be the three numbers of the smallest correct version?
        let p1: (u64, u64, u64) = crate::guess();
        support::assert_answer(
            "P1 version after a removal",
            &format!("{p1:?}"),
            0xbef7_a363_d899_903a,
            "Removing a public item breaks every user who names it.",
        );

        // P2. Suppose pacer were at 0.4.2 and made the release of this capstone
        // (additions and one deprecation). The smallest correct version?
        let p2: (u64, u64, u64) = crate::guess();
        support::assert_answer(
            "P2 version before 1.0",
            &format!("{p2:?}"),
            0xe29d_31eb_809a_aee3,
            "Before 1.0.0 the second number is the incompatible one. A compatible \
             release must stay inside what `pacer = \"0.4.2\"` accepts.",
        );

        // P3. A user's manifest says `pacer = "1.0.0"`, and its Cargo.lock has
        // 1.0.0. After your release is published, the user runs `cargo update`.
        // Does the user get the release without editing the manifest?
        let p3: bool = crate::guess();
        support::assert_answer(
            "P3 update reaches the user",
            &p3.to_string(),
            0xf6fc_56a3_0b12_f4de,
            "A plain version in a manifest is a caret requirement.",
        );
    }
}
