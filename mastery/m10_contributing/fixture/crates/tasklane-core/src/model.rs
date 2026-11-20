//! The data a Lanefile is parsed into.

use tasklane_plugin::TaskFlags;

/// One `step` line of a task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepCall {
    /// The step name, for example `write`.
    pub name: String,
    /// The arguments as written, before variable expansion.
    pub args: Vec<String>,
    /// The line of the Lanefile, counting from 1.
    pub line: usize,
}

/// One task of a Lanefile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    /// The task name.
    pub name: String,
    /// The text of a `##` comment directly above the task, if there is one.
    pub doc: Option<String>,
    /// The tasks that must run first, in the order they were written.
    pub deps: Vec<String>,
    /// The flags from the task's `flags` lines.
    pub flags: TaskFlags,
    /// The steps, in order.
    pub steps: Vec<StepCall>,
    /// The line of the `task` directive, counting from 1.
    pub line: usize,
}

/// A parsed Lanefile.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lanefile {
    /// The `set` directives in file order. A later entry overrides an earlier one
    /// with the same name.
    pub vars: Vec<(String, String)>,
    /// The tasks in file order.
    pub tasks: Vec<Task>,
}

impl Lanefile {
    /// The task called `name`.
    pub fn task(&self, name: &str) -> Option<&Task> {
        self.tasks.iter().find(|task| task.name == name)
    }

    /// The task names in file order.
    pub fn task_names(&self) -> impl Iterator<Item = &str> {
        self.tasks.iter().map(|task| task.name.as_str())
    }
}

/// The flags as shown to users: lower case, separated by `, `, in the order
/// `silent`, `optional`, `hidden`. No flags gives an empty string.
///
/// ```
/// use tasklane_core::{TaskFlags, describe_flags};
///
/// assert_eq!(describe_flags(TaskFlags::HIDDEN | TaskFlags::SILENT), "silent, hidden");
/// assert_eq!(describe_flags(TaskFlags::empty()), "");
/// ```
pub fn describe_flags(flags: TaskFlags) -> String {
    if flags.is_empty() {
        return String::new();
    }
    format!("{flags:?}").to_lowercase().replace(" | ", ", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_lookup() {
        let file = Lanefile {
            vars: vec![],
            tasks: vec![Task {
                name: "a".into(),
                doc: None,
                deps: vec![],
                flags: TaskFlags::empty(),
                steps: vec![],
                line: 1,
            }],
        };
        assert_eq!(file.task("a").map(|t| t.line), Some(1));
        assert!(file.task("b").is_none());
        assert_eq!(file.task_names().collect::<Vec<_>>(), ["a"]);
    }

    #[test]
    fn one_flag() {
        assert_eq!(describe_flags(TaskFlags::OPTIONAL), "optional");
    }
}
