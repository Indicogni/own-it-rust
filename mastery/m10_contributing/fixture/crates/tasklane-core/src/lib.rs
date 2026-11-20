//! tasklane runs named tasks in dependency order.
//!
//! A *Lanefile* declares variables and tasks. A task has dependencies and a list of
//! steps. Running a task first runs everything it depends on, each task once.
//!
//! The crate is organised as a pipeline. Each stage is one module:
//!
//! | Stage | Module | Entry point |
//! | --- | --- | --- |
//! | text to data | [`mod@parse`] (with [`text`] for words and quotes) | [`parse()`] |
//! | which tasks, in which order | [`graph`] | [`plan`] |
//! | `${NAME}` in step arguments | [`vars`] | [`expand`] |
//! | step names to code | [`registry`], [`builtin`] | [`Registry`] |
//! | execution | [`run`] | [`Runner`] |
//! | results | [`report`] | [`RunReport`] |
//!
//! ```
//! use tasklane_core::{Registry, Runner, Vars, Workspace, parse};
//!
//! let file = parse(
//!     "set NAME = world\n\
//!      task hello\n\
//!      \x20 step write out.txt hello, ${NAME}\n\
//!      \x20 step upper out.txt\n",
//! )?;
//! let registry = Registry::with_builtins();
//! let mut ws = Workspace::new();
//! let report = Runner::new(&registry).run(&file, "hello", &Vars::from_lanefile(&file), &mut ws)?;
//! assert!(report.success());
//! assert_eq!(ws.read("out.txt"), Some("HELLO, WORLD"));
//! # Ok::<(), tasklane_core::Error>(())
//! ```

#![deny(missing_docs)]

pub mod builtin;
pub mod error;
pub mod glob;
pub mod graph;
pub mod model;
pub mod parse;
pub mod registry;
pub mod report;
pub mod run;
pub mod text;
pub mod vars;

pub use error::Error;
pub use graph::plan;
pub use model::{Lanefile, StepCall, Task, describe_flags};
pub use parse::parse;
pub use registry::Registry;
pub use report::{RunReport, TaskResult, TaskStatus};
pub use run::{RunOptions, Runner};
pub use tasklane_plugin::{Step, StepContext, StepError, TaskFlags, Workspace};
pub use vars::{Vars, expand};

/// The version of this crate, as in `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
