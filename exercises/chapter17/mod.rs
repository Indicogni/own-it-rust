//! Chapter 17: Working with text. Original guided practice.
//! Run one lesson: cargo test --test chapter17 l01_
// `unused_imports` is allowed because every lesson imports `guess` for its
// predictions, and that import is no longer used once you have answered them.
#![allow(dead_code, unused_variables, unused_mut, unused_imports)]

mod c01_code_review;
mod l01_unicode;
mod l02_editing;
mod l03_search;
mod l04_iteration;
mod l05_parsing;
mod l06_formatting;
mod l07_custom_format;
mod l08_representations;
mod l09_capstone;
mod r01_review;

// Lets tests count heap allocations, so they can check how you solved a task.
#[global_allocator]
static ALLOC: practice::alloc::Counting = practice::alloc::Counting;
