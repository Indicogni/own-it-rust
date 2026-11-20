//! Mastery module M4: an async runtime from scratch, then a real service.
//!
//! Each `lNN_*` module is one lesson. Read the matching section of
//! `CONCEPTS.md` first, then the lesson header. `README.md` has the lesson
//! table and the commands.

// Starter bodies are `todo!()`, so parameters, imports and helper items stay
// unused until you implement them.
#![allow(
    dead_code,
    unused_variables,
    unused_imports,
    unused_mut,
    unreachable_code
)]
// The tests drop values of types that get their `Drop` impl from you (a
// `Sleep`, a `Reactor`, a `Receiver`). Until that impl exists, clippy would
// report each of those `drop(..)` calls as useless.
#![allow(clippy::drop_non_drop)]

pub mod support;

pub mod c01_code_review;
pub mod l01_raw_waker;
pub mod l02_tasks;
pub mod l03_timers;
pub mod l04_reactor;
pub mod l05_projection;
pub mod l06_streams;
pub mod l07_select;
pub mod l08_capstone_chat;
pub mod l09_tokio_service;

pub use support::{guess, within};

// Lets tests count heap allocations, so they can check how you solved a task.
#[cfg(test)]
#[global_allocator]
static ALLOC: support::alloc::Counting = support::alloc::Counting;

/// The teaching text, compiled as documentation tests: `cargo test --doc`
/// builds and runs every `rust` block in `CONCEPTS.md`.
#[cfg(doctest)]
#[doc = include_str!("../CONCEPTS.md")]
pub struct ConceptsDoctests;
