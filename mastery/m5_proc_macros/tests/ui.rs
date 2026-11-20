//! End-to-end tests for lessons 02 to 08 and the code review lesson.
//! Run one lesson with, for example, `cargo test l02_`.
//!
//! A unit test in `macros/src/` can look at the tokens that a macro returns.
//! It cannot tell whether those tokens compile, what the compiler says about
//! them, or what the generated code does when it runs. These tests can. Each
//! one hands some of the small programs in `tests/ui/` to the `trybuild`
//! crate, which compiles every program on its own, as a user of your macros:
//!
//! - a `pass_*.rs` program must compile, run and exit normally;
//! - a `fail_*.rs` program must be rejected, and the compiler's output must be
//!   equal to the `.stderr` file next to it.
//!
//! You do not need to edit this file. Read the programs in `tests/ui/`: they
//! are the precise contract. When a test fails, the compiler output that
//! trybuild prints above the test summary says why. A message such as
//! `proc-macro derive panicked ... not yet implemented` means that the macro
//! still contains a `todo!()`.
//!
//! Lesson 04 explains trybuild and the `.stderr` files, and asks you to add
//! a case of your own.

use std::path::{Path, PathBuf};

/// The files in `tests/ui/<lesson>/` whose names start with `prefix` and end
/// with `.rs`.
fn own_cases(lesson: &str, prefix: &str) -> Vec<PathBuf> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("ui")
        .join(lesson);
    let mut found: Vec<PathBuf> = std::fs::read_dir(&directory)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", directory.display()))
        .filter_map(|entry| Some(entry.ok()?.path()))
        .filter(|path| {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            name.starts_with(prefix) && name.ends_with(".rs")
        })
        .collect();
    found.sort();
    found
}

/// Fails with instructions when there is no compile-fail case of your own, or
/// when one has no recorded `.stderr` file yet.
fn require_own_fail_cases(lesson: &str, prefix: &str) {
    let cases = own_cases(lesson, prefix);
    assert!(
        !cases.is_empty(),
        "add a compile-fail case of your own: a file tests/ui/{lesson}/{prefix}NAME.rs that \
         uses the macro in a way that must not compile. The lesson file says how"
    );
    // With TRYBUILD=overwrite, trybuild is about to write the missing files.
    if std::env::var_os("TRYBUILD").is_some_and(|mode| mode == "overwrite") {
        return;
    }
    for case in cases {
        let recorded = case.with_extension("stderr");
        assert!(
            recorded.is_file(),
            "{} has no recorded compiler output yet. Run this test once with the environment \
             variable TRYBUILD=overwrite, read the new file {} line by line, and keep it only \
             if every message and every `^^^` marker is what a user should see",
            case.display(),
            recorded.display()
        );
    }
}

// ---------------------------------------------------------------- lesson 02

#[test]
fn l02_b_changes_runs() {
    trybuild::TestCases::new().pass("tests/ui/l02/pass_changes.rs");
}

#[test]
fn l02_c_setters_run() {
    trybuild::TestCases::new().pass("tests/ui/l02/pass_setters.rs");
}

#[test]
fn l02_compiler_errors_in_generated_code() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/l02/fail_no_partial_eq.rs");
    cases.compile_fail("tests/ui/l02/fail_private_setter.rs");
}

// ---------------------------------------------------------------- lesson 03

#[test]
fn l03_c_redacted_runs() {
    trybuild::TestCases::new().pass("tests/ui/l03/pass_redacted.rs");
}

#[test]
fn l03_c_generics_and_bounds_compile() {
    trybuild::TestCases::new().pass("tests/ui/l03/pass_generics.rs");
}

#[test]
fn l03_c_a_shown_field_needs_debug() {
    trybuild::TestCases::new().compile_fail("tests/ui/l03/fail_not_debug.rs");
}

// ---------------------------------------------------------------- lesson 04

#[test]
fn l04_c_label_runs() {
    trybuild::TestCases::new().pass("tests/ui/l04/pass_label.rs");
}

#[test]
fn l04_b_c_errors_are_shown_where_the_mistake_is() {
    trybuild::TestCases::new().compile_fail("tests/ui/l04/fail_*.rs");
}

#[test]
fn l04_d_own_case() {
    require_own_fail_cases("l04", "own_");
    trybuild::TestCases::new().compile_fail("tests/ui/l04/own_*.rs");
}

// ---------------------------------------------------------------- lesson 05

#[test]
fn l05_b_logged_runs() {
    trybuild::TestCases::new().pass("tests/ui/l05/pass_logged.rs");
}

#[test]
fn l05_b_errors_keep_the_item() {
    trybuild::TestCases::new().compile_fail("tests/ui/l05/fail_*.rs");
}

// ---------------------------------------------------------------- lesson 06

#[test]
fn l06_flags_run() {
    trybuild::TestCases::new().pass("tests/ui/l06/pass_*.rs");
}

#[test]
fn l06_errors_are_shown_where_the_mistake_is() {
    trybuild::TestCases::new().compile_fail("tests/ui/l06/fail_*.rs");
}

// ---------------------------------------------------------------- lesson 07

#[test]
fn l07_a_attempt_runs() {
    trybuild::TestCases::new().pass("tests/ui/l07/pass_attempt.rs");
}

#[test]
fn l07_a_attempt_is_hygienic() {
    trybuild::TestCases::new().pass("tests/ui/l07/pass_attempt_hygiene.rs");
}

#[test]
fn l07_b_named_runs() {
    trybuild::TestCases::new().pass("tests/ui/l07/pass_named.rs");
}

#[test]
fn l07_b_errors_are_shown_where_the_mistake_is() {
    trybuild::TestCases::new().compile_fail("tests/ui/l07/fail_*.rs");
}

// ---------------------------------------------------------------- lesson 08

#[test]
fn l08_builder_runs() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/l08/pass_basic.rs");
    cases.pass("tests/ui/l08/pass_generic.rs");
}

#[test]
fn l08_defaults_are_lazy_and_hygienic() {
    trybuild::TestCases::new().pass("tests/ui/l08/pass_defaults.rs");
}

#[test]
fn l08_builder_works_in_a_hostile_module() {
    trybuild::TestCases::new().pass("tests/ui/l08/pass_hostile.rs");
}

#[test]
fn l08_errors_are_shown_where_the_mistake_is() {
    trybuild::TestCases::new().compile_fail("tests/ui/l08/fail_*.rs");
}

#[test]
fn l08_own_cases() {
    assert!(
        !own_cases("l08", "own_pass_").is_empty(),
        "add a passing case of your own: a file tests/ui/l08/own_pass_NAME.rs with a `main` \
         that uses `#[derive(Builder)]` in a way the supplied cases do not cover"
    );
    require_own_fail_cases("l08", "own_fail_");
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/l08/own_pass_*.rs");
    cases.compile_fail("tests/ui/l08/own_fail_*.rs");
}

// ------------------------------------------------------------- code review

#[test]
fn c01_a_plain_struct_resets() {
    trybuild::TestCases::new().pass("tests/ui/c01/pass_plain.rs");
}

#[test]
fn c01_a_b_other_shapes_reset() {
    trybuild::TestCases::new().pass("tests/ui/c01/pass_shapes.rs");
}

#[test]
fn c01_c_errors_are_shown_where_the_mistake_is() {
    trybuild::TestCases::new().compile_fail("tests/ui/c01/fail_*.rs");
}
