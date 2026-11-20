//! `kcore`: every part of the kernel that does not need hardware.
//!
//! The crate is `no_std` and does not use `alloc`, so the same code runs
//! inside the kernel (target `aarch64-unknown-none`) and in ordinary host
//! tests, where Miri can check its unsafe code.
//!
//! The items re-exported below are the facade that SPEC.md fixes. How the
//! crate is split into modules behind them is a free choice.

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]

mod heap;
mod ipc;
pub mod reg;
mod ring;
mod sched;

pub use heap::{Bump, FreeList};
pub use ipc::{Endpoint, IpcError, Message};
pub use ring::RingBuffer;
pub use sched::{Scheduler, TaskId, TaskState};
