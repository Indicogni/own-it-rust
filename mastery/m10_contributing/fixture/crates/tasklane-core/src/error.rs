//! The error type shared by the parser, the planner and the runner.

use std::fmt;

/// Everything that can go wrong before or while running a Lanefile.
///
/// A failing step is not an `Error`: it is recorded in the
/// [`RunReport`](crate::RunReport), because the other tasks may still run.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The Lanefile is malformed. `line` counts from 1.
    Parse {
        /// The line that could not be understood.
        line: usize,
        /// What is wrong with it.
        message: String,
    },
    /// The requested task does not exist.
    UnknownTask(String),
    /// Tasks depend on each other in a circle. The names are in dependency order and
    /// the first name is repeated at the end: `["a", "b", "a"]`.
    Cycle(Vec<String>),
    /// A step argument uses `${NAME}` but `NAME` was never set.
    UnknownVariable(String),
    /// A step argument has `${` without a closing `}`. Holds the whole argument.
    UnterminatedVariable(String),
    /// A task uses a step that is not in the registry.
    UnknownStep {
        /// The task that uses the step.
        task: String,
        /// The name that was not found.
        step: String,
    },
    /// A step with this name is already registered.
    DuplicateStep(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Parse { line, message } => write!(f, "line {line}: {message}"),
            Error::UnknownTask(name) => write!(f, "unknown task `{name}`"),
            Error::Cycle(names) => write!(f, "dependency cycle: {}", names.join(" -> ")),
            Error::UnknownVariable(name) => write!(f, "unknown variable `{name}`"),
            Error::UnterminatedVariable(text) => write!(f, "missing `}}` in `{text}`"),
            Error::UnknownStep { task, step } => {
                write!(f, "task `{task}` uses unknown step `{step}`")
            }
            Error::DuplicateStep(name) => {
                write!(f, "a step named `{name}` is already registered")
            }
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages() {
        let parse = Error::Parse {
            line: 3,
            message: "unknown directive `stpe`".into(),
        };
        assert_eq!(parse.to_string(), "line 3: unknown directive `stpe`");
        let cycle = Error::Cycle(vec!["a".into(), "b".into(), "a".into()]);
        assert_eq!(cycle.to_string(), "dependency cycle: a -> b -> a");
        assert_eq!(
            Error::UnterminatedVariable("${X".into()).to_string(),
            "missing `}` in `${X`"
        );
    }
}
