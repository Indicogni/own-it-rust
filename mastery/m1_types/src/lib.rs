//! Mastery module M1: the type system in depth.
//!
//! Each `lNN_*` module is one lesson. Read the matching section of
//! `CONCEPTS.md` first, then the lesson header. `README.md` has the lesson
//! table and the commands.
//!
//! Lessons 01 to 05 give you signatures and unit tests. From lesson 06 on, more
//! and more of the design is yours: the lesson states a contract in prose, and
//! the checks live in `tests/cases/`, as small programs that are compiled
//! against this library the way a user of your code would compile them.

// Starter bodies are `todo!()`, so parameters and imports stay unused until you
// implement them.
#![allow(dead_code, unused_variables, unused_imports, unused_mut)]

pub mod support;

pub mod c01_code_review;
pub mod l01_variance;
pub mod l02_bounds_hrtb;
pub mod l03_lending;
pub mod l04_phantom;
pub mod l05_trait_objects;
pub mod l06_zst_never;
pub mod l07_sealed_typestate;
pub mod l08_coherence;
pub mod l09_capstone;

pub use support::guess;

// Lets the unit tests count heap allocations, so that they can check how you
// solved a task. A program that uses this library is not affected.
#[cfg(test)]
#[global_allocator]
static ALLOC: support::alloc::Counting = support::alloc::Counting;

/// The teaching text, compiled as documentation tests: `cargo test --doc`
/// builds and runs every `rust` block in `CONCEPTS.md`.
#[cfg(doctest)]
#[doc = include_str!("../CONCEPTS.md")]
pub struct ConceptsDoctests;
