//! Tansy: the reference solution of the interpreter capstone.
//!
//! The pipeline, one module per stage:
//!
//! ```text
//! text --lexer--> tokens --parser--> tree --resolve--> tree with addresses --interp--> effects
//! ```
//!
//! `session` ties the stages together and keeps the state that outlives one
//! run (the REPL needs that); `cli` is the command line. `DESIGN.md` next
//! to this crate explains the decisions.

#![forbid(unsafe_code)]

pub mod ast;
pub mod builtins;
pub mod cli;
pub mod interp;
pub mod lexer;
pub mod parser;
pub mod resolve;
pub mod session;
pub mod source;
pub mod value;

/// The stack of the thread that runs the interpreter.
///
/// The evaluator is recursive: a Tansy call that is 1000 levels deep is a
/// Rust call chain several thousand frames deep, and unoptimised frames are
/// large. The main thread's stack is too small for that on some systems
/// (1 MiB on Windows), so `main` starts a thread with this much. The memory
/// is only reserved; pages are used when the stack actually grows.
pub const STACK_SIZE: usize = 256 * 1024 * 1024;

/// Runs `work` on a new thread with [`STACK_SIZE`] bytes of stack and waits
/// for the result. A panic in `work` continues in the caller.
pub fn with_large_stack<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> std::io::Result<T> {
    let worker = std::thread::Builder::new()
        .name("tansy".into())
        .stack_size(STACK_SIZE)
        .spawn(work)?;
    match worker.join() {
        Ok(result) => Ok(result),
        Err(panic) => std::panic::resume_unwind(panic),
    }
}
