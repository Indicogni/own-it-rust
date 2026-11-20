//! Chapter 11: Generic code with traits. Original guided practice.
//! Run one lesson: cargo test --test chapter11 l01_
#![allow(dead_code, unused_variables, unused_mut)]

mod c01_code_review;
mod l01_interfaces;
mod l02_dispatch;
mod l03_self;
mod l04_extensions;
mod l05_relationships;
mod l06_bounds;
mod l07_capstone;
mod r01_review;

// Lets tests count heap allocations, so they can check how you solved a task.
#[global_allocator]
static ALLOC: practice::alloc::Counting = practice::alloc::Counting;
