//! Fixed-capacity collections and a bump arena for programs without a heap.
//!
//! This is the reference solution of the capstone `nostd_collections`.
//! SPEC.md (in `capstones/nostd_collections/`) is the contract; DESIGN.md
//! next to this crate's `src/` explains the decisions.
//!
//! | Type | What it is | File |
//! | --- | --- | --- |
//! | [`ArrayVec`] | a vector with room for `N` elements, stored inline | `array_vec.rs` |
//! | [`RingBuffer`] | a double-ended queue with room for `N` elements | `ring.rs` |
//! | [`Arena`] | a bump allocator over `N` inline bytes | `arena.rs` |
//! | [`CapacityError`] | "no room"; carries the rejected value | `error.rs` |
//!
//! The crate uses `core` only: no `std`, no `alloc`, no dependencies.

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs, missing_debug_implementations)]
#![warn(clippy::undocumented_unsafe_blocks)]

pub mod arena;
pub mod array_vec;
mod error;
pub mod ring;

pub use arena::Arena;
pub use array_vec::ArrayVec;
pub use error::CapacityError;
pub use ring::RingBuffer;

// Runs the examples of SPEC.md as documentation tests, including the
// programs of SPEC section 7 that must not compile (SPEC 9.2). The file
// `src/spec_examples.rs` comes with the starter.
#[cfg(doctest)]
mod spec_examples;
