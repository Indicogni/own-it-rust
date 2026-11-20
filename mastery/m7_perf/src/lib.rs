//! Mastery module M7: performance engineering.
//!
//! Each `lNN_*` module is one lesson. Read the matching section of
//! `CONCEPTS.md` first, then the lesson header. `README.md` has the lesson
//! table and the commands, and `LABS.md` has the work with real tools: the
//! criterion benchmarks, a sampling profiler, and the compiler's own output.
//!
//! No test in this module reads a wall clock. A test counts something instead:
//! heap allocations, bytes, clock reads, comparisons, modelled cache misses,
//! bounds checks, virtual calls, or calls to `read` and `write`. Such a count
//! is the same on every machine and in every run, so a budget on it is a test
//! that cannot flake. The benchmarks in `benches/` then show what the counts
//! are worth in time on your machine.
//!
//! Lessons 01 to 04 give you signatures. Lesson 05 gives you signatures and
//! leaves the structure of the solution to you. From lesson 06 on, the lesson
//! states a contract in prose; the file holds only the names that the tests
//! use, and the representation behind them is your design.

// Starter bodies are `todo!()`, so parameters, imports and `mut` markers stay
// unused until you implement them.
#![allow(dead_code, unused_variables, unused_imports, unused_mut)]

pub mod support;

pub mod c01_code_review;
pub mod l01_bench_design;
pub mod l02_profile;
pub mod l03_allocations;
pub mod l04_layout;
pub mod l05_traversal;
pub mod l06_dispatch;
pub mod l07_io;
pub mod l08_capstone;

pub use support::{guess, within};

// The unit tests measure heap use. Benchmarks and examples link the library
// without `cfg(test)`, so they keep the system allocator.
#[cfg(test)]
#[global_allocator]
static ALLOC: support::alloc::Counting = support::alloc::Counting;

/// The teaching text, compiled as documentation tests: `cargo test --doc`
/// builds and runs every `rust` block in `CONCEPTS.md`.
#[cfg(doctest)]
#[doc = include_str!("../CONCEPTS.md")]
pub struct ConceptsDoctests;
