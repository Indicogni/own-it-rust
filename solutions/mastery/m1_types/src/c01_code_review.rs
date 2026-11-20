//! CODE REVIEW 01 — A generic API that asks for too much and shows too much
//! Run (in mastery/m1_types/): cargo test --no-fail-fast c01_
//!
//! This code was written in a hurry. It compiles, and some of its tests pass.
//! REVIEW it first: in NOTES.md, list every problem you can find before you
//! run anything. Look for wrong results, for bounds and lifetimes that demand
//! more from the caller than the code needs, for types in signatures that
//! show how the code is built inside, and for copies that serve no purpose.
//!
//! Then run the tests, fix the code, and run `cargo clippy --all-targets`
//! until it is quiet. You may change every signature. Keep the names
//! `Pipeline`, `new`, `stage`, `run`, `names` and `longest`, and keep the way
//! the tests call them. The programs in `tests/cases/c01/` show three callers
//! that the first version turns away, or tells too much.
//!
//! EXPLAIN afterward: For each bound or lifetime that you removed, say what a
//! caller can do now that was rejected before. For `names`, say what you may
//! change inside `Pipeline` now without breaking a caller.

/// One stage: a function from text to text that may borrow for `'f`.
type StageFn<'f> = Box<dyn Fn(&str) -> String + 'f>;

/// A list of named text stages, run in the order in which they were added.
///
/// `'f` is how long the stage closures may borrow. A caller who only uses
/// closures without borrows never has to write it.
pub struct Pipeline<'f> {
    stages: Vec<(String, StageFn<'f>)>,
}

impl<'f> Pipeline<'f> {
    pub fn new() -> Self {
        Pipeline { stages: Vec::new() }
    }

    /// Adds a stage after the stages that are already there.
    // No `'static`, `Send` or `Sync`: the pipeline stores the closure and
    // calls it on the caller's thread, so it needs none of them. The name is
    // copied, so any `&str` will do.
    pub fn stage<F>(mut self, name: &str, f: F) -> Self
    where
        F: Fn(&str) -> String + 'f,
    {
        self.stages.push((name.to_string(), Box::new(f)));
        self
    }

    /// Runs every stage, each one on the output of the stage before it.
    pub fn run(&self, input: &str) -> String {
        let mut stages = self.stages.iter();
        let Some((_, first)) = stages.next() else {
            return input.to_string();
        };
        // Each stage borrows the text of the stage before; nothing is copied.
        stages.fold(first(input), |text, (_, stage)| stage(&text))
    }

    /// The stage names, in running order.
    // `impl Iterator` hides how the names are stored and how they are walked.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.stages.iter().map(|(name, _)| name.as_str())
    }
}

impl Default for Pipeline<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// The longest item, or `None` for an empty slice. When several items have
/// the greatest length, the first of them wins.
pub fn longest<T: AsRef<str>>(items: &[T]) -> Option<&str> {
    // `max_by_key` returns the LAST maximum, so walk the items backwards.
    items
        .iter()
        .map(AsRef::as_ref)
        .rev()
        .max_by_key(|item| item.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::alloc::count;

    fn names_of(pipeline: &Pipeline) -> Vec<String> {
        pipeline.names().map(|name| name.to_string()).collect()
    }

    #[test]
    fn stages_run_in_the_order_added() {
        let pipeline = Pipeline::new()
            .stage("add a", |text| format!("{text}a"))
            .stage("add b", |text| format!("{text}b"));
        let input = String::from("x");
        assert_eq!(
            pipeline.run(&input),
            "xab",
            "the stage that was added first must run first"
        );
    }

    #[test]
    fn names_are_in_running_order() {
        let pipeline = Pipeline::new()
            .stage("trim", |text| text.trim().to_string())
            .stage("upper", |text| text.to_uppercase());
        assert_eq!(names_of(&pipeline), ["trim", "upper"]);
        let input = String::from("  quiet ");
        assert_eq!(pipeline.run(&input), "QUIET");
    }

    #[test]
    fn empty_pipeline_returns_the_input() {
        let pipeline = Pipeline::new();
        let input = String::from("unchanged");
        assert_eq!(pipeline.run(&input), "unchanged");
        assert!(names_of(&pipeline).is_empty());
    }

    #[test]
    fn run_copies_the_text_at_most_once() {
        // Each stage returns one new String, which is one allocation. Every
        // other allocation that is counted here was made by `run` itself.
        let pipeline = Pipeline::new()
            .stage("one", |text| text.to_owned())
            .stage("two", |text| text.to_owned())
            .stage("three", |text| text.to_owned());
        let input = String::from("some text that is long enough to need a heap buffer");
        let (output, allocations) = count(|| pipeline.run(&input));
        assert_eq!(output, input);
        assert!(
            allocations <= 4,
            "run made {} heap allocation(s) of its own for 3 stages (the stages made 3 more); \
             each stage can borrow the output of the one before, so run needs at most one \
             copy of the input",
            allocations.saturating_sub(3)
        );
    }

    #[test]
    fn longest_prefers_the_first_of_equals() {
        assert_eq!(
            longest(&["pear", "fig", "plum", "kiwi"]),
            Some("pear"),
            "three items have 4 bytes; the first of them must win"
        );
        assert_eq!(longest(&["a", "ccc", "bb"]), Some("ccc"));
        assert_eq!(longest(&["only"]), Some("only"));
    }

    #[test]
    fn longest_of_nothing_is_none() {
        let nothing: &[&str] = &[];
        assert_eq!(longest(nothing), None);
    }
}
