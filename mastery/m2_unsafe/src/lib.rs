//! Mastery module M2: unsafe Rust by rebuilding the standard library.
//!
//! Each `lNN_*` module is one lesson. Read the matching section of `CONCEPTS.md`
//! first, then the lesson header. `README.md` has the lesson table, the commands
//! and the Miri setup. The programs in `examples/` are the Miri labs.

// Starter bodies are `todo!()`, so parameters, fields and imports stay unused
// until you implement them.
#![allow(dead_code, unused_variables, unused_imports)]

pub mod support;

pub mod c01_code_review;
pub mod l01_unsafe_cell;
pub mod l02_box;
pub mod l03_vec;
pub mod l04_iterators;
pub mod l05_rc;
pub mod l06_aliasing;
pub mod l07_send_sync;
pub mod l08_capstone;

pub use support::{guess, within};

// The audit allocator replaces the system allocator in native test builds only.
// Under Miri it is left out, so that Miri's own allocator checks apply.
#[cfg(all(test, not(miri)))]
#[global_allocator]
static ALLOC: support::alloc::Counting = support::alloc::Counting;

/// The teaching text, compiled as documentation tests: `cargo test --doc`
/// builds and runs every `rust` block in `CONCEPTS.md`.
#[cfg(doctest)]
#[doc = include_str!("../CONCEPTS.md")]
pub struct ConceptsDoctests;
