//! The step interface of tasklane.
//!
//! A *step* is one operation inside a task, such as `write` or `upper`. tasklane ships
//! its built-in steps in `tasklane-core`. To add your own step, implement [`Step`] and
//! register it with the registry of `tasklane-core`. This crate is small on purpose:
//! a plugin depends only on it, not on the parser or the runner.
//!
//! Steps never touch the disk. They work on a [`Workspace`], an in-memory set of text
//! files, so a run is repeatable and easy to test.
//!
//! ```
//! use tasklane_plugin::{Step, StepContext, StepError, TaskFlags, Workspace};
//!
//! /// Reverses the lines of a file.
//! struct Reverse;
//!
//! impl Step for Reverse {
//!     fn name(&self) -> &str {
//!         "reverse"
//!     }
//!     fn usage(&self) -> &str {
//!         "reverse PATH"
//!     }
//!     fn run(
//!         &self,
//!         args: &[String],
//!         ws: &mut Workspace,
//!         _cx: &StepContext<'_>,
//!     ) -> Result<(), StepError> {
//!         let [path] = args else {
//!             return Err(StepError::usage(self));
//!         };
//!         let text = ws
//!             .read(path)
//!             .ok_or_else(|| StepError::new(self.name(), format!("no such file `{path}`")))?;
//!         let reversed: Vec<&str> = text.lines().rev().collect();
//!         ws.write(path, reversed.join("\n"));
//!         Ok(())
//!     }
//! }
//!
//! let mut ws = Workspace::new();
//! ws.write("a.txt", "one\ntwo");
//! let cx = StepContext::new("demo", TaskFlags::empty());
//! Reverse.run(&["a.txt".to_string()], &mut ws, &cx).unwrap();
//! assert_eq!(ws.read("a.txt"), Some("two\none"));
//! ```

#![deny(missing_docs)]

use std::collections::BTreeMap;
use std::fmt;

bitflags::bitflags! {
    /// Per-task switches, set with the `flags` directive in a Lanefile.
    // bitflags 2 no longer implements these traits by itself. All eight were part of
    // the type's public API with bitflags 1, so all eight stay. The one method that
    // bitflags 2 dropped, `from_bits_unchecked`, is restored by hand below.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct TaskFlags: u8 {
        /// The task's steps are not recorded in the workspace log.
        const SILENT = 0b001;
        /// A failure of this task does not fail the run and does not skip its dependents.
        const OPTIONAL = 0b010;
        /// `tasklane list` does not show the task.
        const HIDDEN = 0b100;
    }
}

impl TaskFlags {
    /// The number of flags that are set.
    pub fn count(self) -> u32 {
        self.bits().count_ones()
    }

    /// Converts from the underlying bits without checking them. Bits that belong to
    /// no flag are kept.
    ///
    /// bitflags 1 generated this method for every flags type and bitflags 2 does
    /// not. Users of 0.4.0 may call it, so it stays until 0.5.
    ///
    /// # Safety
    ///
    /// There is nothing left to uphold: this is the safe `from_bits_retain` under
    /// its old name. The function stays `unsafe` so that its signature is the one
    /// of 0.4.0.
    #[deprecated = "use the safe `from_bits_retain` method instead"]
    pub const unsafe fn from_bits_unchecked(bits: u8) -> TaskFlags {
        TaskFlags::from_bits_retain(bits)
    }

    /// Looks up one flag by the name used in a Lanefile: `silent`, `optional` or
    /// `hidden`. Names are case-sensitive.
    pub fn from_keyword(name: &str) -> Option<TaskFlags> {
        match name {
            "silent" => Some(TaskFlags::SILENT),
            "optional" => Some(TaskFlags::OPTIONAL),
            "hidden" => Some(TaskFlags::HIDDEN),
            _ => None,
        }
    }
}

/// The files and the log that steps work on.
///
/// Paths are plain strings with `/` as the separator. There are no directories: a
/// "directory" exists while some file path starts with its name. Files are kept sorted
/// by path, so everything that lists files does so in a stable order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Workspace {
    files: BTreeMap<String, String>,
    log: Vec<String>,
}

impl Workspace {
    /// Creates an empty workspace.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates or replaces the file at `path`.
    pub fn write(&mut self, path: &str, content: impl Into<String>) {
        self.files.insert(path.to_string(), content.into());
    }

    /// The content of the file at `path`, or `None` if there is no such file.
    pub fn read(&self, path: &str) -> Option<&str> {
        self.files.get(path).map(String::as_str)
    }

    /// A part of the file at `path`: `len` bytes starting at byte offset `start`.
    ///
    /// The result borrows from the workspace; nothing is copied. Returns `None` if
    /// there is no such file, if the range does not lie inside the file, or if it
    /// starts or ends in the middle of a character.
    pub fn read_range(&self, path: &str, start: usize, len: usize) -> Option<&str> {
        let text = self.files.get(path)?;
        // `checked_add` rejects a range whose end does not fit in a `usize`. `get`
        // checks both ends against the length and against character boundaries, so
        // no `unsafe` is needed and the result still borrows from the file.
        let end = start.checked_add(len)?;
        text.get(start..end)
    }

    /// Whether a file exists at `path`.
    pub fn contains(&self, path: &str) -> bool {
        self.files.contains_key(path)
    }

    /// Removes the file at `path` and returns its content, or `None` if there was none.
    pub fn remove(&mut self, path: &str) -> Option<String> {
        self.files.remove(path)
    }

    /// All file paths, sorted.
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(String::as_str)
    }

    /// The number of files.
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Whether the workspace has no files.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Appends one line to the log. Steps use the log for output meant for the user.
    pub fn log(&mut self, line: impl Into<String>) {
        self.log.push(line.into());
    }

    /// The log lines, oldest first.
    pub fn log_lines(&self) -> &[String] {
        &self.log
    }
}

/// Why a step failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepError {
    step: String,
    message: String,
}

impl StepError {
    /// Creates an error for the step named `step`.
    pub fn new(step: &str, message: impl Into<String>) -> Self {
        StepError {
            step: step.to_string(),
            message: message.into(),
        }
    }

    /// The error for wrong arguments: its message shows the step's usage line.
    pub fn usage(step: &dyn Step) -> Self {
        StepError::new(step.name(), format!("usage: {}", step.usage()))
    }

    /// The name of the step that failed.
    pub fn step(&self) -> &str {
        &self.step
    }

    /// What went wrong, without the step name.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for StepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.step, self.message)
    }
}

impl std::error::Error for StepError {}

/// What a step may know about the task it runs in.
///
/// More fields may be added in minor releases, so build values with
/// [`StepContext::new`].
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct StepContext<'a> {
    /// The name of the running task.
    pub task: &'a str,
    /// The flags of the running task.
    pub flags: TaskFlags,
}

impl<'a> StepContext<'a> {
    /// Creates the context for one task.
    pub fn new(task: &'a str, flags: TaskFlags) -> Self {
        StepContext { task, flags }
    }
}

/// One kind of step.
///
/// A step is stateless: everything it reads or writes goes through the [`Workspace`].
/// `Send + Sync` is required so that a registry of steps can be shared between threads.
pub trait Step: Send + Sync {
    /// The name used in a Lanefile, for example `upper`.
    fn name(&self) -> &str;

    /// One line that shows the arguments, for example `upper PATH`.
    fn usage(&self) -> &str;

    /// Runs the step. `args` are the arguments after variable expansion.
    fn run(
        &self,
        args: &[String],
        ws: &mut Workspace,
        cx: &StepContext<'_>,
    ) -> Result<(), StepError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_then_read() {
        let mut ws = Workspace::new();
        assert!(ws.is_empty());
        ws.write("b.txt", "bee");
        ws.write("a.txt", String::from("ant"));
        assert_eq!(ws.read("a.txt"), Some("ant"));
        assert_eq!(ws.read("missing"), None);
        assert_eq!(ws.len(), 2);
        assert!(ws.contains("b.txt"));
    }

    #[test]
    fn paths_are_sorted() {
        let mut ws = Workspace::new();
        ws.write("z", "");
        ws.write("a/b", "");
        ws.write("a/a", "");
        assert_eq!(ws.paths().collect::<Vec<_>>(), ["a/a", "a/b", "z"]);
    }

    #[test]
    fn remove_returns_the_content() {
        let mut ws = Workspace::new();
        ws.write("a", "x");
        assert_eq!(ws.remove("a"), Some("x".to_string()));
        assert_eq!(ws.remove("a"), None);
    }

    #[test]
    fn read_range_inside_the_file() {
        let mut ws = Workspace::new();
        ws.write("a", "hello, world");
        assert_eq!(ws.read_range("a", 0, 5), Some("hello"));
        assert_eq!(ws.read_range("a", 7, 5), Some("world"));
        assert_eq!(ws.read_range("a", 12, 0), Some(""));
        assert_eq!(ws.read_range("a", 13, 0), None);
        assert_eq!(ws.read_range("nope", 0, 0), None);
    }

    #[test]
    fn read_range_rejects_ranges_that_leave_the_file() {
        let mut ws = Workspace::new();
        ws.write("a", "hello");
        assert_eq!(ws.read_range("a", 0, 6), None);
        assert_eq!(ws.read_range("a", 5, 1), None);
        assert_eq!(ws.read_range("a", 2, usize::MAX), None);
    }

    #[test]
    fn read_range_rejects_ranges_that_split_a_character() {
        let mut ws = Workspace::new();
        ws.write("a", "h\u{e9}llo");
        assert_eq!(ws.read_range("a", 0, 2), None);
        assert_eq!(ws.read_range("a", 2, 1), None);
        assert_eq!(ws.read_range("a", 1, 2), Some("\u{e9}"));
    }

    #[test]
    fn log_keeps_order() {
        let mut ws = Workspace::new();
        ws.log("first");
        ws.log(String::from("second"));
        assert_eq!(ws.log_lines(), ["first", "second"]);
    }

    #[test]
    fn flag_names() {
        assert_eq!(TaskFlags::from_keyword("silent"), Some(TaskFlags::SILENT));
        assert_eq!(TaskFlags::from_keyword("Silent"), None);
        assert_eq!((TaskFlags::SILENT | TaskFlags::HIDDEN).count(), 2);
        assert_eq!(TaskFlags::empty().count(), 0);
    }

    #[test]
    #[allow(deprecated)]
    fn from_bits_unchecked_is_still_there() {
        // Kept for users of 0.4.0 across the bitflags upgrade (#19).
        // SAFETY: the function has no requirements; see its documentation.
        let flags = unsafe { TaskFlags::from_bits_unchecked(0b101) };
        assert_eq!(flags, TaskFlags::SILENT | TaskFlags::HIDDEN);
    }

    #[test]
    fn step_error_display() {
        let err = StepError::new("upper", "no such file `a`");
        assert_eq!(err.to_string(), "upper: no such file `a`");
        assert_eq!(err.step(), "upper");
        assert_eq!(err.message(), "no such file `a`");
    }
}
