//! Mastery module M3: concurrency by building the primitives.
//!
//! Each `lNN_*` module is one lesson. Read the matching section of
//! `CONCEPTS.md` first, then the lesson header. `README.md` has the lesson
//! table and the commands.
//!
//! Lessons 01 to 03 give you the types and their fields. From lesson 04 on the
//! starter gives only the public names, so that the tests compile: the fields,
//! the helper types, the `Send`/`Sync` impls and the `Drop` impls are yours.
//!
//! Two kinds of tests check your work. `cargo test` runs ordinary tests on
//! real threads. `cargo model` runs the `model` module of each lesson under
//! loom, which tries every interleaving and reports orderings that are too
//! weak. [`sync`] explains how one source file serves both.

// Starter bodies are `todo!()`, so parameters and imports stay unused until you
// implement them.
#![allow(dead_code, unused_variables, unused_imports)]

pub mod support;
pub mod sync;

pub mod c01_code_review;
pub mod l01_orderings;
pub mod l02_spinlock;
pub mod l03_oneshot;
pub mod l04_arc;
pub mod l05_mutex;
pub mod l06_channel;
pub mod l07_treiber;
pub mod l08_capstone;

pub use support::{guess, within};

// Test builds count allocations and frees per thread (see `support::alloc`).
#[cfg(all(test, not(loom)))]
#[global_allocator]
static ALLOC: support::alloc::Counting = support::alloc::Counting;

/// The teaching text, compiled as documentation tests: `cargo test --doc`
/// builds and runs every `rust` block in `CONCEPTS.md`.
#[cfg(doctest)]
#[doc = include_str!("../CONCEPTS.md")]
pub struct ConceptsDoctests;
