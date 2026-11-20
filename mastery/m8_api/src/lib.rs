//! Mastery module M8: library design, semver and stewardship.
//!
//! Each `lNN_*` module is one lesson, and `c01_code_review` is the API review.
//! Read the matching section of `CONCEPTS.md` first, then the lesson header.
//! `README.md` has the lesson table and the commands.
//!
//! Lessons 01 to 03 give you signatures. From lesson 04 on, more and more of
//! the design is yours: a lesson states a contract in prose, and the checks are
//! small programs in `fixtures/` that are compiled against your library, the
//! way a user of your library would compile them.

// Starter bodies are `todo!()`, so parameters and imports stay unused until you
// implement them.
#![allow(dead_code, unused_variables, unused_imports)]

pub mod support;

pub mod c01_code_review;
pub mod l01_breaking;
pub mod l02_traits;
pub mod l03_future_proofing;
pub mod l04_errors;
pub mod l05_features;
pub mod l06_docs;
pub mod l07_msrv;
pub mod l08_capstone;

pub use support::guess;
