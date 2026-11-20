//! LESSON 07 — Documentation: rustdoc, links, examples, README, changelog
//! Run: cargo test --manifest-path mastery/m9_toolbox/Cargo.toml l07_
//! Read first: CONCEPTS.md, "Lesson 07 · Documentation".
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: your README shows an example. What
//! makes sure it still compiles after the next API change?
//! IMPLEMENT: no Rust bodies here. You document `fixtures/l07_docs/` (the crate
//! `slugs`) the way a published crate is documented. The tests run `cargo doc`,
//! `cargo test --doc` and `cargo run --example` there, and read the files.
//! EXPLAIN afterward: Explain who reads each of these, and when: the README, the
//! crate-level docs, an item's docs, an example program, the changelog.
//! JUDGMENT: Write the README once and include it as the crate documentation, so
//! its example is compiled and tested. Link items with intra-doc links, so a rename
//! breaks the docs build instead of leaving a dead reference.
//!
//! TASKS (edit only files under `fixtures/l07_docs/`; in `src/lib.rs` change only
//! comments and attributes, not code):
//!
//! ```text
//! A. `README.md` says what the crate does and shows a Rust example in a fenced
//!    code block that uses `slugify` and `Slug::new`. The README is the crate-level
//!    documentation (`#![doc = include_str!(...)]`), so the example runs as a doc
//!    test. The README lists the feature `titles` and links to `CHANGELOG.md`.
//! B. Every public item is documented. `slugify` and `Slug::new` each have an
//!    `# Examples` section with a doc test. Item docs refer to other items with
//!    intra-doc links: the page of `slugify` links to `Slug` and to `Slug::new`, and
//!    `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` passes.
//! C. docs.rs readiness: `[package.metadata.docs.rs]` documents all features and
//!    passes `--cfg docsrs` to rustdoc; the crate root has
//!    `#![cfg_attr(docsrs, feature(doc_cfg))]`.
//! D. `examples/basic.rs` prints the slug of the title "The Professional Toolbox".
//! E. Package metadata: `description` (one sentence, at most 100 characters),
//!    `readme`, `repository`, up to five `keywords`, at least one of `categories`.
//!    Use `https://example.com/...` for the repository; this crate is not real.
//! F. `CHANGELOG.md` in the "Keep a Changelog" layout: an `## [Unreleased]` section
//!    first, then one `## [version] - YYYY-MM-DD` section per release, newest first.
//!    Each release groups its entries under `### Added`, `### Changed`, `### Fixed`
//!    (and so on). The newest release is the version in `Cargo.toml`. The history is
//!    in the starter README.
//! ```

#[cfg(test)]
mod tests {
    use crate::guess;
    use crate::support::{self, Value, cargo, cargo_with, fixture, list_at, text_at};
    use std::path::PathBuf;

    fn krate() -> PathBuf {
        fixture("l07_docs")
    }

    fn doc_tests() -> support::Run {
        cargo(&krate(), &["test", "--doc", "--all-features"])
    }

    #[test]
    fn a_readme_is_the_crate_documentation() {
        let readme = support::read(&krate().join("README.md"));
        let source = support::read(&krate().join("src/lib.rs"));
        assert!(
            support::squeeze(&source).contains("#![doc=include_str!(\"../README.md\")]"),
            "src/lib.rs must use the README as crate documentation: \
             `#![doc = include_str!(\"../README.md\")]`"
        );
        assert!(
            readme.contains("```rust") || readme.contains("```\n"),
            "README.md needs a fenced code block with a Rust example"
        );
        for needed in ["slugify", "Slug::new", "titles", "CHANGELOG.md"] {
            assert!(readme.contains(needed), "README.md must mention `{needed}`");
        }
        let _turn = support::take_turn();
        let run = doc_tests();
        run.expect_ok("`cargo test --doc --all-features` must pass in fixtures/l07_docs");
        assert!(
            run.stdout
                .lines()
                .any(|line| line.contains("README.md") && line.ends_with("... ok")),
            "the README example did not run as a doc test:\n{}",
            run.stdout
        );
    }

    #[test]
    fn b_items_are_documented_and_linked() {
        let _turn = support::take_turn();
        let check = cargo(
            &krate(),
            &["check", "--all-features", "--message-format=json"],
        );
        check.expect_ok("the crate must build");
        let warnings = check.diagnostics("warning");
        assert!(
            warnings.is_empty(),
            "undocumented public items: {:?}\n{}",
            support::codes(&warnings),
            warnings[0].rendered
        );

        let doc = cargo_with(
            &krate(),
            &["doc", "--no-deps", "--all-features"],
            &[("RUSTDOCFLAGS", "-D warnings")],
        );
        doc.expect_ok(
            "`RUSTDOCFLAGS=\"-D warnings\" cargo doc --no-deps --all-features` must pass",
        );

        // Technique: the generated page must contain real links. Back-ticks alone
        // (`Slug`) give code font; brackets ([`Slug`]) give a checked link.
        let page = support::nested_target_dir(&krate()).join("doc/slugs/fn.slugify.html");
        let html = support::read(&page);
        for target in ["struct.Slug.html\"", "struct.Slug.html#method.new\""] {
            assert!(
                html.contains(target),
                "the documentation of `slugify` must link to `{}` with an intra-doc link",
                target.trim_end_matches('"')
            );
        }

        let run = doc_tests();
        // On Windows rustdoc prints the file as `src\lib.rs`.
        let stdout = run.stdout.replace('\\', "/");
        for item in ["slugify", "Slug::new"] {
            assert!(
                stdout.lines().any(|line| {
                    line.contains(&format!("src/lib.rs - {item} (line")) && line.ends_with("... ok")
                }),
                "`{item}` needs an `# Examples` section with a passing doc test:\n{}",
                run.stdout
            );
        }
    }

    #[test]
    fn c_docs_rs_metadata() {
        let manifest = support::toml(&krate().join("Cargo.toml"));
        assert_eq!(
            manifest.pointer("/package/metadata/docs/rs/all-features"),
            Some(&Value::Bool(true)),
            "[package.metadata.docs.rs] must set `all-features = true`; otherwise the \
             published docs do not show `title_case` at all"
        );
        assert_eq!(
            list_at(&manifest, "/package/metadata/docs/rs/rustdoc-args"),
            ["--cfg", "docsrs"],
            "[package.metadata.docs.rs] must pass `--cfg docsrs` to rustdoc"
        );
        let source = support::squeeze(&support::read(&krate().join("src/lib.rs")));
        assert!(
            source.contains("#![cfg_attr(docsrs,feature(doc_cfg))]"),
            "the crate root must enable `doc_cfg` when `docsrs` is set, and only then: \
             `#![cfg_attr(docsrs, feature(doc_cfg))]`"
        );
    }

    #[test]
    fn d_example_program_runs() {
        let _turn = support::take_turn();
        let run = cargo(&krate(), &["run", "-q", "--example", "basic"]);
        run.expect_ok("`cargo run --example basic` must work: create examples/basic.rs");
        assert_eq!(
            run.stdout.trim(),
            "the-professional-toolbox",
            "the example prints the slug of \"The Professional Toolbox\""
        );
        let example = support::read(&krate().join("examples/basic.rs"));
        assert!(
            example.contains("slugs::"),
            "an example uses the crate through its public API, like a user would"
        );
    }

    #[test]
    fn e_package_metadata() {
        let manifest = support::toml(&krate().join("Cargo.toml"));
        let description = text_at(&manifest, "/package/description").unwrap_or("");
        assert!(
            (10..=100).contains(&description.chars().count()),
            "`description` is one sentence of at most 100 characters, found {description:?}"
        );
        assert_eq!(text_at(&manifest, "/package/readme"), Some("README.md"));
        assert!(
            text_at(&manifest, "/package/repository")
                .is_some_and(|url| url.starts_with("https://example.com/")),
            "`repository` must be an https://example.com/... address in this exercise"
        );
        let keywords = list_at(&manifest, "/package/keywords");
        assert!(
            (1..=5).contains(&keywords.len()),
            "crates.io accepts at most five keywords, found {keywords:?}"
        );
        assert!(
            !list_at(&manifest, "/package/categories").is_empty(),
            "pick at least one category slug from crates.io, for example `text-processing`"
        );
    }

    /// One `## [version] - date` section of a changelog.
    #[derive(Debug)]
    struct Release {
        version: String,
        date: String,
        groups: Vec<String>,
        entries: usize,
    }

    fn releases(changelog: &str) -> (bool, Vec<Release>) {
        let mut unreleased_first = false;
        let mut releases: Vec<Release> = Vec::new();
        for line in changelog.lines() {
            if let Some(heading) = line.strip_prefix("## ") {
                if heading.trim() == "[Unreleased]" {
                    unreleased_first = releases.is_empty();
                    continue;
                }
                let (version, date) = heading.split_once(" - ").unwrap_or((heading, ""));
                releases.push(Release {
                    version: version
                        .trim()
                        .trim_start_matches('[')
                        .trim_end_matches(']')
                        .to_owned(),
                    date: date.trim().to_owned(),
                    groups: Vec::new(),
                    entries: 0,
                });
            } else if let Some(group) = line.strip_prefix("### ") {
                if let Some(release) = releases.last_mut() {
                    release.groups.push(group.trim().to_owned());
                }
            } else if line.starts_with("- ")
                && let Some(release) = releases.last_mut()
            {
                release.entries += 1;
            }
        }
        (unreleased_first, releases)
    }

    fn is_iso_date(date: &str) -> bool {
        let parts: Vec<&str> = date.split('-').collect();
        parts.len() == 3
            && [4, 2, 2]
                .iter()
                .zip(&parts)
                .all(|(len, part)| part.len() == *len && part.bytes().all(|b| b.is_ascii_digit()))
    }

    #[test]
    fn f_changelog_layout() {
        let text = support::read(&krate().join("CHANGELOG.md"));
        let (unreleased_first, releases) = releases(&text);
        assert!(
            unreleased_first,
            "the first `## ` section must be `## [Unreleased]`: it collects changes \
             that are merged but not released"
        );
        let versions: Vec<&str> = releases.iter().map(|r| r.version.as_str()).collect();
        assert_eq!(
            versions,
            ["0.2.0", "0.1.0"],
            "one `## [version] - date` section per release, newest first"
        );
        let manifest = support::toml(&krate().join("Cargo.toml"));
        assert_eq!(
            text_at(&manifest, "/package/version"),
            Some(versions[0]),
            "the newest changelog section must be the version in Cargo.toml"
        );
        const GROUPS: [&str; 6] = [
            "Added",
            "Changed",
            "Deprecated",
            "Removed",
            "Fixed",
            "Security",
        ];
        for release in &releases {
            assert!(
                is_iso_date(&release.date),
                "release {} needs a date in the form YYYY-MM-DD, found {:?}",
                release.version,
                release.date
            );
            assert!(
                !release.groups.is_empty() && release.entries >= release.groups.len(),
                "release {} needs `### ` groups with `- ` entries under them",
                release.version
            );
            for group in &release.groups {
                assert!(
                    GROUPS.contains(&group.as_str()),
                    "release {}: `### {group}` is not one of {GROUPS:?}",
                    release.version
                );
            }
        }
        assert!(
            releases[0].date > releases[1].date,
            "ISO dates sort as text; the newer release must have the later date"
        );
        let newest = &releases[0];
        assert!(
            newest.groups.iter().any(|g| g == "Added")
                && newest.groups.iter().any(|g| g == "Fixed"),
            "0.2.0 added something and fixed something; list both under their groups"
        );
        assert!(
            text.contains("title_case"),
            "the changelog names what a user would search for, such as `title_case`"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    ///
    /// These run rustdoc on `fixtures/l04_layout` (read its `src/lib.rs`) and on this
    /// lesson's crate. P2 and P3 need the crate to build, so do tasks A and B first.
    #[test]
    fn p_predictions() {
        let _turn = support::take_turn();

        // P1. `fixtures/l04_layout/src/lib.rs` has three fenced blocks in the docs of
        // `double`: one plain, one `text`, one `ignore`. How many doc tests does
        // `cargo test --doc` LIST there (run or not)?
        let listed = cargo(&fixture("l04_layout"), &["test", "--doc", "--", "--list"]);
        listed.expect_ok("listing the doc tests must work");
        let p1: usize = 2;
        assert_eq!(
            p1,
            listed
                .stdout
                .lines()
                .filter(|l| l.ends_with(": test"))
                .count(),
            "P1: a block is a doc test unless its language says it is not Rust; \
             `ignore` is still listed"
        );

        // P2. `Slug` is a tuple struct with one private field. For a tuple struct
        // with public fields, the generated page has a section "Tuple Fields". Does
        // the page `struct.Slug.html` of this crate contain that text?
        cargo(&krate(), &["doc", "--no-deps", "--all-features"])
            .expect_ok("fixtures/l07_docs must build its docs");
        let page =
            support::read(&support::nested_target_dir(&krate()).join("doc/slugs/struct.Slug.html"));
        let p2: bool = false;
        assert_eq!(
            p2,
            page.contains("Tuple Fields"),
            "P2: rustdoc documents the public API; private fields are left out"
        );

        // P3. Without `--all-features`, does the generated documentation contain a
        // page for `title_case` (which sits behind the feature `titles`)?
        let dir = support::nested_target_dir(&krate()).join("doc/slugs");
        let _ = std::fs::remove_file(dir.join("fn.title_case.html"));
        cargo(&krate(), &["doc", "--no-deps"]).expect_ok("fixtures/l07_docs must build its docs");
        let p3: bool = false;
        assert_eq!(
            p3,
            dir.join("fn.title_case.html").exists(),
            "P3: rustdoc documents what is compiled; items behind a disabled feature \
             do not exist for it"
        );
    }
}
