//! LESSON 06 — Documentation is part of the interface: examples that are tests
//! Run: cargo test --manifest-path mastery/m8_api/Cargo.toml l06_
//! Read first: "Lesson 06" in CONCEPTS.md (same package).
//!
//! The library of this lesson, `fixtures/l06/slugger/`, works and has no
//! documentation at all. Its code is finished: you write only documentation
//! comments in `src/lib.rs`, and one example program. The tests run `cargo
//! check`, `cargo doc` and `cargo test --doc` on the package.
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then read `fixtures/l06/slugger/src/imp.rs`
//! and consider: which sentences would a user need in order to call `truncate`
//! correctly without reading that file? Which of them can be checked by a
//! machine?
//!
//! IMPLEMENT the contract below. Keep the supplied tests.
//! EXPLAIN afterward: Task D runs your examples against five wrong versions of
//! the library. Which of your examples caught which one? What does that say
//! about the difference between an example that shows a function and an
//! example that specifies it?
//!
//! # The contract
//!
//! A. **Every public item has documentation**: the crate (`//!` at the top of
//!    `src/lib.rs`), both functions, the struct, both methods, the enum, every
//!    variant, and the field of `TooLong`. The first line of each comment is
//!    one sentence that says what the item is or does.
//!
//! B. **The documentation links to what it mentions.** The crate documentation
//!    names `slug`, `truncate` and `Tag::parse` as links, in the form
//!    ``[`slug`]``. `cargo doc` reports no broken link.
//!
//! C. **The crate, `slug`, `truncate`, `Tag::parse` and `Tag::as_str` each have
//!    an example that runs as a test.** The documentation of `Tag::parse` has a
//!    section `# Errors` that says when each error is returned. The
//!    documentation of `truncate` has a section `# Panics`, and an example that
//!    shows the panic (a block that starts with ```` ```should_panic ````).
//!    Examples handle errors with `?`, as real code does, and never call
//!    `unwrap()`.
//!
//! D. **The examples are a specification.** `src/imp.rs` can be built in five
//!    wrong ways. For each of them, at least one of your examples must fail.
//!
//! E. **`examples/titles.rs` is a program that uses the library**: it prints the
//!    slug of each command-line argument, one per line. Without arguments it
//!    prints the slug of `Hello, World!`.
//!
//! Useful commands, from `fixtures/l06/slugger/`:
//!
//! ```text
//! cargo doc --open
//! cargo test --doc
//! RUSTFLAGS="--cfg mutant_slug_keeps_runs" cargo test --doc
//! ```

#[cfg(test)]
mod tests {
    use crate::support::{self, CargoRun};

    const LIB: &str = "fixtures/l06/slugger/src/lib.rs";

    /// Runs `cargo ARGS` in the package `slugger`.
    fn slugger(target_key: &str, args: &[&str], env: &[(&str, &str)]) -> CargoRun {
        support::cargo("fixtures/l06/slugger", target_key, args, env)
    }

    /// The documentation comment directly above the first line of `source`
    /// that contains `item`: its `///` lines, without the slashes.
    fn doc_above(source: &str, item: &str) -> String {
        let lines: Vec<&str> = source.lines().collect();
        let at = lines
            .iter()
            .position(|line| line.contains(item))
            .unwrap_or_else(|| panic!("{LIB} no longer contains `{item}`; keep the signatures"));
        let mut doc: Vec<&str> = lines[..at]
            .iter()
            .rev()
            .map(|line| line.trim_start())
            // Attributes may stand between the comment and the item.
            .skip_while(|line| line.starts_with("#["))
            .take_while(|line| line.starts_with("///"))
            .map(|line| line.trim_start_matches('/').trim_start())
            .collect();
        doc.reverse();
        doc.join("\n")
    }

    /// All lines of documentation comments in `source`.
    fn doc_lines(source: &str) -> Vec<&str> {
        source
            .lines()
            .map(str::trim_start)
            .filter(|line| line.starts_with("///") || line.starts_with("//!"))
            .collect()
    }

    #[test]
    fn a_doc_comments_are_found_above_their_item() {
        let source =
            "/// One.\n///\n/// # Errors\n#[inline]\npub fn f() {}\n// plain\npub fn g() {}\n";
        assert_eq!(doc_above(source, "pub fn f("), "One.\n\n# Errors");
        assert_eq!(doc_above(source, "pub fn g("), "");
    }

    #[test]
    fn a_every_public_item_is_documented() {
        let run = slugger(
            "l06",
            &["check", "--quiet"],
            &[("RUSTFLAGS", "-D missing-docs")],
        );
        assert!(
            run.ok,
            "`cargo check` with the lint `missing_docs` as an error found public items \
             without documentation:\n{}",
            support::head(&run.stderr, 40)
        );
    }

    #[test]
    fn b_the_documentation_links_to_what_it_mentions() {
        let flags = "-D rustdoc::broken-intra-doc-links -D rustdoc::invalid-rust-codeblocks";
        let run = slugger(
            "l06",
            &["doc", "--quiet", "--no-deps"],
            &[("RUSTDOCFLAGS", flags)],
        );
        assert!(
            run.ok,
            "`cargo doc` found a link that leads nowhere, or a code block that is not \
             Rust:\n{}",
            support::head(&run.stderr, 40)
        );
        let source = support::read(LIB);
        let crate_docs: Vec<&str> = source
            .lines()
            .filter(|line| line.starts_with("//!"))
            .collect();
        assert!(
            !crate_docs.is_empty(),
            "{LIB} has no crate documentation: start the file with `//!` lines"
        );
        let crate_docs = crate_docs.join("\n");
        for link in ["[`slug`]", "[`truncate`]", "[`Tag::parse`]"] {
            assert!(
                crate_docs.contains(link),
                "the crate documentation must mention {link}, written exactly like this, \
                 so that rustdoc turns it into a link"
            );
        }
    }

    #[test]
    fn c_the_examples_run_as_tests() {
        let run = slugger("l06", &["test", "--doc"], &[]);
        assert!(
            run.ok,
            "`cargo test --doc` failed: an example in the documentation does not compile \
             or does not hold:\n{}",
            support::head(&run.all(), 60)
        );
        // rustdoc names each test after the file and the item that the example
        // belongs to. (On Windows the file is printed as `src\lib.rs`.)
        let stdout = run.stdout.replace('\\', "/");
        let tests: Vec<&str> = stdout
            .lines()
            .filter(|line| line.starts_with("test src/lib.rs - "))
            .collect();
        for (item, name) in [
            ("the crate", "- (line"),
            ("`slug`", "- slug (line"),
            ("`truncate`", "- truncate (line"),
            ("`Tag::parse`", "- Tag::parse (line"),
            ("`Tag::as_str`", "- Tag::as_str (line"),
        ] {
            assert!(
                tests.iter().any(|test| test.contains(name)),
                "{item} has no example that runs as a test. The tests that rustdoc found:\n{}",
                tests.join("\n")
            );
        }
        assert!(
            tests
                .iter()
                .any(|test| test.contains("- truncate (line") && test.contains("should panic")),
            "the documentation of `truncate` must show its panic in a `should_panic` example"
        );
    }

    #[test]
    fn c_failures_have_their_sections() {
        let source = support::read(LIB);
        assert!(
            doc_above(&source, "pub fn parse(").contains("# Errors"),
            "the documentation of `Tag::parse` needs a section `# Errors`: a caller must \
             know which error to expect when"
        );
        assert!(
            doc_above(&source, "pub fn truncate(").contains("# Panics"),
            "the documentation of `truncate` needs a section `# Panics`: a panic that is \
             not documented is a bug report waiting to be written"
        );
        for line in doc_lines(&source) {
            assert!(
                !line.contains("unwrap()"),
                "an example calls `unwrap()`. Readers copy examples; show them `?`. \
                 End the example with a hidden line such as\n\
                 /// # Ok::<(), slugger::TagError>(())\nThe line is:\n{line}"
            );
        }
    }

    #[test]
    fn d_the_examples_notice_wrong_implementations() {
        // The library must be right for this check to mean anything.
        let run = slugger("l06-mutants", &["test", "--doc", "--quiet"], &[]);
        assert!(
            run.ok,
            "`cargo test --doc` must pass with the correct library first:\n{}",
            support::head(&run.all(), 40)
        );
        for (mutant, bug) in [
            (
                "mutant_slug_keeps_runs",
                "`slug` writes one dash for every separator, so \"a  b\" gives \"a--b\"",
            ),
            (
                "mutant_slug_trailing_dash",
                "`slug` leaves a dash at the end when the title ends with a separator",
            ),
            (
                "mutant_truncate_keeps_dash",
                "`truncate` may return a text that ends with a dash",
            ),
            (
                "mutant_truncate_accepts_zero",
                "`truncate` returns an empty text for `max == 0` and does not panic",
            ),
            (
                "mutant_tag_allows_upper",
                "`Tag::parse` accepts upper case letters",
            ),
        ] {
            let flags = format!("--cfg {mutant}");
            let env = [("RUSTFLAGS", flags.as_str())];
            let run = slugger("l06-mutants", &["test", "--doc", "--quiet"], &env);
            assert!(
                !run.ok,
                "All your examples still pass when the library is wrong in this way: \
                 {bug}.\n  So no example states that part of the behaviour. Add an \
                 assertion to the documentation of that function. Try it yourself with\n  \
                 RUSTFLAGS=\"--cfg {mutant}\" cargo test --doc"
            );
            assert!(
                run.stdout.contains("test result: FAILED"),
                "{mutant}: the wrong library must make an example fail, but the build \
                 itself failed:\n{}",
                support::head(&run.stderr, 30)
            );
        }
    }

    #[test]
    fn e_the_example_program_prints_slugs() {
        let run = slugger(
            "l06",
            &[
                "run",
                "--quiet",
                "--example",
                "titles",
                "--",
                "Rust 2024",
                "Why  SemVer?",
            ],
            &[],
        );
        assert!(
            run.ok,
            "`cargo run --example titles` failed:\n{}",
            support::head(&run.stderr, 30)
        );
        assert_eq!(
            run.stdout, "rust-2024\nwhy-semver\n",
            "one slug per argument, one per line"
        );
        let run = slugger("l06", &["run", "--quiet", "--example", "titles"], &[]);
        assert_eq!(
            run.stdout, "hello-world\n",
            "without arguments the program shows the slug of `Hello, World!`"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// A wrong prediction is useful: read the failure, then explain it in NOTES.md.
    ///
    /// Read `fixtures/l06/fences.md` first. It has nine code blocks. The test
    /// runs `rustdoc --test` on it, exactly as rustdoc treats a documentation
    /// comment.
    #[test]
    fn p_predictions() {
        let output = support::rustdoc_test("fixtures/l06/fences.md");
        let totals = support::totals(&output);
        assert_eq!(
            totals.failed, 0,
            "the blocks of fences.md are right as they are:\n{output}"
        );

        // P1. For how many of the nine blocks does rustdoc report a test that
        // passed? (A block that is compiled but not run counts, and so does a
        // block that is expected to fail and does.)
        let p1: usize = 6;
        assert_eq!(
            p1, totals.passed,
            "P1: every block is a Rust test unless its label says otherwise. \
             rustdoc printed:\n{output}"
        );

        // P2. How many blocks does rustdoc list as "ignored"?
        let p2: usize = 1;
        assert_eq!(
            p2, totals.ignored,
            "P2: only the label `ignore` gives an ignored test; a label that names \
             another language gives no test at all. rustdoc printed:\n{output}"
        );
    }
}
