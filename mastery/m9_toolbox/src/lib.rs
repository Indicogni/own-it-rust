//! The professional toolbox: Cargo, lints, tests, fuzzing, debugging, docs, supply
//! chain, CI and release. Start with `README.md`.
//!
//! Each `lNN_*` module is one lesson. Many lessons work on a small project under
//! `fixtures/`; the lesson file says which files you edit.

// Starter bodies are `todo!()`, so their parameters are unused until you implement
// them. `unused_imports` is allowed because `guess` is no longer used by a lesson once
// its predictions are answered.
#![allow(dead_code, unused_variables, unused_mut, unused_imports)]

pub mod l01_cargo;
pub mod l02_editor;
pub mod l03_lints;
// insta and tempfile are dev-dependencies, so this lesson exists only in test builds.
#[cfg(test)]
mod l04_testing;
pub mod l05_fuzz;
pub mod l06_debug;
pub mod l07_docs;
pub mod l08_supply;
pub mod l09_release;
pub mod support;

/// Placeholder for a prediction. Replace the call with your answer before
/// running the lesson, for example `let p1: usize = 5;`.
#[track_caller]
pub fn guess<T>() -> T {
    panic!("PREDICT: replace `guess()` with your written prediction, then run again")
}
