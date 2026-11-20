//! Mastery module M11: Rust as a service.
//!
//! Each `lNN_*` module is one lesson. Read the matching section of `CONCEPTS.md`
//! first, then the lesson header. `README.md` has the lesson table and commands.
//! The `wire/` crate holds the first half of lesson 07 and `capstone/` holds the
//! final project.

// Starter bodies are `todo!()`, so parameters and imports stay unused until you
// implement them.
#![allow(dead_code, unused_variables, unused_imports)]

pub mod support;

pub mod c01_code_review;
pub mod l01_anatomy;
pub mod l02_middleware;
pub mod l03_persistence;
pub mod l04_auth;
pub mod l05_api_design;
pub mod l06_cpu_work;
pub mod l07_protocol;
pub mod l08_operations;
pub mod l09_testing;

pub use support::{guess, within};

/// The teaching text, compiled as documentation tests: `cargo test --doc`
/// builds and runs every `rust` block in `CONCEPTS.md`.
#[cfg(doctest)]
#[doc = include_str!("../CONCEPTS.md")]
pub struct ConceptsDoctests;
