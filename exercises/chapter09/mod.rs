// `unused_imports` is allowed because every lesson imports `guess` for its
// predictions, and that import is no longer used once you have answered them.
#![allow(dead_code, unused_variables, unused_mut, unused_imports)]

mod c01_code_review;
mod l01_named;
mod l02_kinds_layout;
mod l03_methods;
mod l04_smart_self;
mod l05_generics;
mod l06_lifetime_const;
mod l07_derive;
mod l08_interior;
mod l09_capstone;
mod l10_weak_parents;
mod r01_review;

// Lets tests count heap allocations, so they can check how you solved a task.
#[global_allocator]
static ALLOC: practice::alloc::Counting = practice::alloc::Counting;
