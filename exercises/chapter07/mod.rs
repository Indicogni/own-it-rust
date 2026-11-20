// `unused_imports` is allowed because `guess` is no longer used by a lesson once
// you have answered all of its predictions. Then delete the lesson's `guess`
// import, as the solutions do.
#![allow(dead_code, unused_variables, unused_mut, unused_imports)]

mod c01_code_review;
mod l01_panic;
mod l02_catching;
mod l03_propagation;
mod l04_methods_aliases;
mod l05_multiple_errors;
mod l06_custom;
mod l07_invariants_ignoring;
mod l08_capstone;
mod l09_send_errors_main;
mod r01_review;

// Lets tests count heap allocations, so they can check how you solved a task.
#[global_allocator]
static ALLOC: practice::alloc::Counting = practice::alloc::Counting;
