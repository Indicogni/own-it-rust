//! Mastery module M6: FFI across a real language boundary.
//!
//! Each `lNN_*` module is one lesson. Read the matching section of
//! `CONCEPTS.md` first, then the lesson header. `README.md` has the lesson
//! table and the commands.
//!
//! The foreign code is real: `build.rs` compiles the C library in
//! `vendor/kvtab` with your system's C compiler and links it into this crate.
//!
//! Lessons 01 to 03 give you signatures. Lessons 04 to 06 give you the public
//! signatures of a safe wrapper and leave its representation and its safety
//! arguments to you. Lesson 07 and the capstone give you a contract in prose
//! and a C test program; every Rust item there is yours to design.

// Starter bodies are `todo!()`, so parameters and imports stay unused until you
// implement them.
#![allow(dead_code, unused_variables, unused_imports)]

pub mod support;

pub mod c_reference;
// Under Miri, the C library is replaced by a model written in Rust.
#[cfg(miri)]
pub mod model;
pub mod probe;
pub mod sys;

#[cfg(test)]
mod harness;

pub mod c01_code_review;
pub mod l01_libc;
pub mod l02_sys_crate;
pub mod l03_bindings;
pub mod l04_safe_wrapper;
pub mod l05_callbacks;
pub mod l06_strings_buffers;
pub mod l07_export;
pub mod l08_capstone;

pub use support::{guess, within};

// Lets tests count the heap allocations that Rust code makes.
#[cfg(test)]
#[global_allocator]
static ALLOC: support::alloc::Counting = support::alloc::Counting;

/// The teaching text, compiled as documentation tests: `cargo test --doc`
/// builds and runs every `rust` block in `CONCEPTS.md`.
#[cfg(doctest)]
#[doc = include_str!("../CONCEPTS.md")]
pub struct ConceptsDoctests;
