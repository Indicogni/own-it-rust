//! Tests for lesson 01 (`bare/src/l01_raw_tokens.rs`).
//! Run: cargo test l01_
//!
//! The lesson's code uses the compiler's `proc_macro` crate, which works only
//! while the compiler runs a macro. So these tests cannot call your functions.
//! They compile small programs that use your derives (the files in
//! `tests/ui/l01/`) and look at the result: a `pass_*.rs` program must compile
//! and run, a `fail_*.rs` program must be rejected with exactly the compiler
//! output recorded in the `.stderr` file next to it. The `trybuild` crate does
//! this work; lesson 04 explains it.
//!
//! When a test fails, read the compiler output that trybuild prints above the
//! test summary. A message such as `proc-macro derive panicked ... not yet
//! implemented` means that the macro still contains a `todo!()`.

// `guess` is unused once every prediction is written.
#![allow(unused_imports)]

use m5_proc_macros::guess;
use proc_macro2::{Spacing, TokenStream, TokenTree};

#[test]
fn l01_a_c_type_name_is_derived() {
    trybuild::TestCases::new().pass("tests/ui/l01/pass_type_name.rs");
}

#[test]
fn l01_d_e_field_names_are_derived() {
    trybuild::TestCases::new().pass("tests/ui/l01/pass_field_names.rs");
}

#[test]
fn l01_b_errors_point_at_the_item_name() {
    trybuild::TestCases::new().compile_fail("tests/ui/l01/fail_unsupported.rs");
}

#[test]
fn l01_the_macro_crate_has_no_dependencies() {
    let manifest = include_str!("../bare/Cargo.toml");
    let mut in_dependencies = false;
    for line in manifest.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.starts_with('[') {
            assert!(
                !line.contains("dependencies."),
                "bare/Cargo.toml declares a dependency in `{line}`. Lesson 01 uses only the \
                 compiler's own `proc_macro` crate"
            );
            in_dependencies = line.contains("dependencies");
        } else if in_dependencies {
            assert!(
                line.is_empty(),
                "bare/Cargo.toml declares the dependency `{line}`. Lesson 01 uses only the \
                 compiler's own `proc_macro` crate; `syn` and `quote` come in lesson 02"
            );
        }
    }
}

/// The top-level token trees of `source`.
///
/// `proc_macro2` has the same token types as the compiler's `proc_macro`, and
/// it also works in a test. What you predict here is what your macro receives.
fn trees(source: &str) -> Vec<TokenTree> {
    let stream: TokenStream = source.parse().expect("valid tokens");
    stream.into_iter().collect()
}

/// P. Predict BEFORE running: replace each `guess()` with your answer.
#[test]
fn l01_p_predictions() {
    // P1. How many token trees does a derive receive for this struct?
    let p1: usize = 4;
    assert_eq!(
        p1,
        trees("pub struct Pair { left: u8, right: u8 }").len(),
        "P1: top-level token trees of the struct (left: your prediction, right: actual)"
    );

    // P2. And how many are inside these braces?
    let p2: usize = 10;
    assert_eq!(
        p2,
        trees("left: Vec<u8>, right: u8").len(),
        "P2: token trees of `left: Vec<u8>, right: u8` (left: your prediction, right: actual)"
    );

    // P3. A doc comment in front of a unit struct.
    let p3: usize = 5;
    assert_eq!(
        p3,
        trees("/// The smallest type.\nstruct Unit;").len(),
        "P3: token trees of a documented unit struct (left: your prediction, right: actual)"
    );

    // P4. `->` is two punctuation tokens. Is the first one marked `Joint`,
    // which means "the next token follows directly"? And the second one?
    let arrow: Vec<bool> = trees("->")
        .iter()
        .map(|tree| matches!(tree, TokenTree::Punct(punct) if punct.spacing() == Spacing::Joint))
        .collect();
    let p4: (bool, bool) = (true, false);
    assert_eq!(
        p4,
        (arrow[0], arrow[1]),
        "P4: (`-` is Joint, `>` is Joint) (left: your prediction, right: actual)"
    );
}
