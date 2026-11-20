//! LESSON 03 — Lints and formatting as policy
//! Run: cargo test --manifest-path mastery/m9_toolbox/Cargo.toml l03_
//! Read first: CONCEPTS.md, "Lesson 03 · Lints and formatting as policy".
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: the manifest says a lint is `deny`,
//! an attribute on one function says `warn`. Which one wins for that function?
//! IMPLEMENT: no Rust bodies here. You turn `fixtures/l03_lints/` (the crate
//! `invoice`) into a crate with a written lint policy that passes
//! `cargo clippy -- -D warnings` and `cargo fmt --check`. The tests run both.
//! EXPLAIN afterward: Explain why `#![deny(warnings)]` in source code is a trap, and
//! where "warnings are errors" belongs instead.
//! JUDGMENT: A lint table is a team agreement. Turn a group on, then decide lint by
//! lint: fix it, allow it for the whole crate with a comment, or `#[expect]` it in one
//! place with a reason. Never silence a lint without leaving the reason behind.
//!
//! TASKS (edit only files under `fixtures/l03_lints/`; never edit `src/seeded.rs`):
//!
//! ```text
//! A. Add a `[lints]` table to `Cargo.toml`:
//!    - rustc: `unsafe_code` is forbidden; `missing_docs` warns.
//!    - Clippy: the `pedantic` group warns, with a priority that lets single lints
//!      override it; `dbg_macro` is denied.
//!    - You may allow single pedantic lints you disagree with. Say why in a comment.
//! B. Remove the blanket `#![deny(warnings)]` from `src/lib.rs`. Replace every
//!    `#[allow(...)]` in that file: delete it if the lint no longer fires, or turn it
//!    into `#[expect(..., reason = "...")]` if the code should stay as it is.
//! C. Create `clippy.toml` and ban `std::process::exit` with `disallowed-methods`,
//!    giving a reason. (`src/seeded.rs` calls it. A test compiles that file with
//!    `--features seeded` and checks that Clippy reports the call.)
//! D. Fix `rustfmt.toml`: only options that stable rustfmt accepts, `max_width = 80`,
//!    `use_field_init_shorthand = true`, `use_try_shorthand = true`. Then format.
//! E. Fix the code until `cargo clippy --all-targets -- -D warnings` and
//!    `cargo fmt --check` pass, and the crate's own tests still pass.
//! ```

#[cfg(test)]
mod tests {
    use crate::guess;
    use crate::support::{self, Value, cargo, fixture, text_at};
    use std::path::PathBuf;

    fn krate() -> PathBuf {
        fixture("l03_lints")
    }

    fn lib_source() -> String {
        support::read(&krate().join("src/lib.rs"))
    }

    #[test]
    fn a_lint_table_states_the_policy() {
        let manifest = support::toml(&krate().join("Cargo.toml"));
        assert_eq!(
            text_at(&manifest, "/lints/rust/unsafe_code"),
            Some("forbid"),
            "[lints.rust] must forbid unsafe code"
        );
        assert_eq!(
            text_at(&manifest, "/lints/rust/missing_docs"),
            Some("warn"),
            "[lints.rust] must warn about missing docs"
        );
        assert_eq!(
            text_at(&manifest, "/lints/clippy/pedantic/level"),
            Some("warn"),
            "[lints.clippy] must set `pedantic = {{ level = \"warn\", priority = ... }}`"
        );
        let priority = manifest
            .pointer("/lints/clippy/pedantic/priority")
            .and_then(Value::as_i64);
        assert!(
            priority.is_some_and(|p| p < 0),
            "the pedantic group needs a negative priority, so that single lints \
             in the same table can override it; found {priority:?}"
        );
        assert_eq!(
            text_at(&manifest, "/lints/clippy/dbg_macro"),
            Some("deny"),
            "[lints.clippy] must deny `dbg_macro`"
        );
        assert!(
            manifest.pointer("/lints/rust/warnings").is_none(),
            "do not set `warnings = \"deny\"` in the manifest either: \
             it breaks the build for everyone when a new compiler adds a warning"
        );
    }

    #[test]
    fn b_no_blanket_deny_and_no_silent_allow() {
        let source = lib_source();
        let squeezed = support::squeeze(&source);
        assert!(
            !squeezed.contains("deny(warnings)"),
            "src/lib.rs still has `#![deny(warnings)]`"
        );
        assert!(
            !squeezed.contains("[allow("),
            "src/lib.rs still has an `allow` attribute; use `#[expect(.., reason = \"..\")]` \
             or delete it"
        );
        let expects: Vec<&str> = squeezed.split("#[expect(").skip(1).collect();
        assert!(
            !expects.is_empty(),
            "keep `line_from_columns` as it is and `#[expect]` the lint it triggers"
        );
        for expect in expects {
            let attribute = expect.split(")]").next().unwrap_or("");
            let reason = attribute.split("reason=\"").nth(1).unwrap_or("");
            assert!(
                reason.find('"').is_some_and(|end| end >= 10),
                "every `#[expect]` needs a `reason = \"...\"` that says why \
                 (at least a few words): `#[expect({attribute})]`"
            );
        }
    }

    #[test]
    fn c_clippy_config_bans_process_exit() {
        let config = support::toml(&krate().join("clippy.toml"));
        let banned = config
            .pointer("/disallowed-methods")
            .and_then(Value::as_array)
            .expect("clippy.toml must have a `disallowed-methods` list");
        let entry = banned
            .iter()
            .find(|entry| {
                entry.as_str() == Some("std::process::exit")
                    || text_at(entry, "/path") == Some("std::process::exit")
            })
            .expect("`disallowed-methods` must list `std::process::exit`");
        assert!(
            text_at(entry, "/reason").is_some_and(|reason| reason.len() >= 10),
            "give the ban a `reason`: Clippy prints it next to every violation"
        );

        // Technique: the configuration must really fire on the seeded call.
        let _turn = support::take_turn();
        let run = cargo(
            &krate(),
            &["clippy", "--features", "seeded", "--message-format=json"],
        );
        let found = run
            .diagnostics("warning")
            .into_iter()
            .chain(run.diagnostics("error"))
            .any(|d| d.code == "clippy::disallowed_methods");
        assert!(
            found,
            "Clippy did not report the `std::process::exit` call in src/seeded.rs.\n{}",
            run.stderr
        );
    }

    #[test]
    fn d_rustfmt_config_is_stable_and_applied() {
        let config = support::toml(&krate().join("rustfmt.toml"));
        assert_eq!(
            config.get("max_width").and_then(Value::as_i64),
            Some(80),
            "rustfmt.toml must set `max_width = 80`"
        );
        for option in ["use_field_init_shorthand", "use_try_shorthand"] {
            assert_eq!(
                config.get(option),
                Some(&Value::Bool(true)),
                "rustfmt.toml must set `{option} = true`"
            );
        }
        let run = cargo(&krate(), &["fmt", "--check"]);
        assert!(
            !run.stderr.contains("unstable features"),
            "rustfmt.toml has an option that only nightly rustfmt accepts. Stable \
             rustfmt ignores it, so the team's formatting differs by toolchain:\n{}",
            run.stderr
        );
        run.expect_ok("`cargo fmt --check` must pass: run `cargo fmt` in the fixture");
    }

    #[test]
    fn e_clean_under_deny_warnings_and_still_correct() {
        let _turn = support::take_turn();
        let clippy = cargo(
            &krate(),
            &[
                "clippy",
                "--all-targets",
                "--message-format=json",
                "--",
                "-D",
                "warnings",
            ],
        );
        let problems = clippy.diagnostics("error");
        assert!(
            clippy.ok() && problems.is_empty(),
            "`cargo clippy --all-targets -- -D warnings` must pass. Still reported: {:?}\n{}",
            support::codes(&problems),
            problems.first().map_or("", |d| d.rendered.as_str())
        );
        let manifest = support::toml(&krate().join("Cargo.toml"));
        assert!(
            manifest.pointer("/lints/clippy/pedantic").is_some(),
            "passing Clippy counts only with the pedantic group switched on (task A)"
        );
        cargo(&krate(), &["test", "-q"]).expect_ok(
            "the fixture's own tests must still pass: a lint fix must not change behaviour",
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    ///
    /// Read `fixtures/l03_levels/`: two crates, `levels` and `forbidden`, each with a
    /// `[lints.rust]` table in its manifest and lint attributes in `src/lib.rs`.
    #[test]
    fn p_predictions() {
        let _turn = support::take_turn();
        let levels = cargo(
            &fixture("l03_levels"),
            &["check", "-p", "levels", "--message-format=json"],
        );
        let warnings = levels.diagnostics("warning");
        let errors = levels.diagnostics("error");

        // P1. How many warnings does `cargo check -p levels` report?
        // Go through the functions `a` to `e` one by one.
        let p1: usize = 3;
        assert_eq!(
            p1,
            warnings.len(),
            "P1: `allow` and a fulfilled `expect` are silent; an unfulfilled `expect` \
             is itself a warning. Reported: {:?}",
            support::codes(&warnings)
        );

        // P2. The manifest of `levels` says `unused_mut = "deny"`, and function `e`
        // carries `#[warn(unused_mut)]`. How many errors does the check report?
        let p2: usize = 0;
        assert_eq!(
            p2,
            errors.len(),
            "P2: a manifest level is a command-line flag; an attribute in the source \
             is nearer to the code and wins"
        );

        // P3. The manifest of `forbidden` says `unused_mut = "forbid"`, and its
        // function carries `#[allow(unused_mut)]`. Does `cargo check -p forbidden` pass?
        let forbidden = cargo(&fixture("l03_levels"), &["check", "-p", "forbidden"]);
        let p3: bool = false;
        assert_eq!(
            p3,
            forbidden.ok(),
            "P3: `forbid` is `deny` that nothing below it may relax (error E0453)"
        );
    }
}
