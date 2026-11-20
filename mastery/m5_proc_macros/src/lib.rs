//! Mastery module M5: procedural macros.
//!
//! This crate is the one a *user* of your macros depends on. It holds the
//! traits that the derives implement, the small run-time helpers that the
//! generated code calls, and it re-exports the macros themselves. You do not
//! need to edit this file. The macros are in two other crates of this workspace:
//!
//! - `bare/` (`m5_bare`): lesson 01, written with the compiler's `proc_macro`
//!   crate only;
//! - `macros/` (`m5_macros`): lessons 02 to 08 and the code review lesson,
//!   written with `syn` and `quote`.
//!
//! A procedural macro crate can export nothing but macros, so a trait and its
//! derive can never live in the same crate. Real libraries use the same split:
//! a main crate that users name in `Cargo.toml`, and a `*_derive` or `*_macros`
//! crate behind it. Start with `README.md`.

// Generated code names this crate by the absolute path `::m5_proc_macros`.
// That path does not exist inside the crate itself, so this line gives the
// crate a second name for its own code. Lesson 07 explains why.
extern crate self as m5_proc_macros;

// A derive macro and a trait may share one name: macros and traits are in
// different namespaces. One `use m5_proc_macros::Changes;` imports both.
pub use m5_bare::{FieldNames, TypeName};
pub use m5_macros::{
    Builder, Changes, Label, Named, Redacted, Reset, Setters, attempt, flags, logged,
};

/// Placeholder for a prediction. Replace the whole call with your answer before
/// running the lesson, for example `let p1: usize = 5;`.
#[track_caller]
pub fn guess<T>() -> T {
    panic!("PREDICT: replace `guess()` with your written prediction, then run again")
}

/// Lesson 01. The name of a type, as written in its declaration.
pub trait TypeName {
    const TYPE_NAME: &'static str;
}

/// Lesson 01. The names of a struct's fields, in declaration order.
pub trait FieldNames {
    const FIELD_NAMES: &'static [&'static str];
}

/// Lesson 02. Compares two values of one struct field by field.
pub trait Changes {
    /// The names of the fields whose values differ between `self` and `newer`,
    /// in declaration order.
    fn changes(&self, newer: &Self) -> Vec<&'static str>;
}

/// Lesson 03. A printable form of a value that hides chosen fields.
pub trait Redacted {
    /// For example `Login { user: "ann", password: *** }`.
    fn redacted(&self) -> String;
}

/// Lesson 04. A fixed text for each variant of an enum without fields.
pub trait Label: Sized {
    /// The text of this variant.
    fn label(&self) -> &'static str;
    /// The variant with this text, if there is one.
    fn from_label(text: &str) -> Option<Self>;
}

/// Lesson 07. Like [`TypeName`], with a derive that can be told where this
/// crate is.
pub trait Named {
    const NAME: &'static str;
}

/// Code review lesson. Puts a value back into its starting state.
pub trait Reset {
    /// Sets every field to its `Default` value, except fields marked
    /// `#[reset(keep)]`.
    fn reset(&mut self);
}

/// Lesson 05. A log of function calls, one log per thread.
///
/// The `#[logged]` attribute makes a function call [`enter`](calls::enter) as
/// its first statement.
pub mod calls {
    use std::cell::RefCell;

    thread_local! {
        static LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    fn record(line: String) {
        // `try_with`: a guard may be dropped while the thread shuts down,
        // when the log is already gone. Then there is nothing to record.
        let _ = LOG.try_with(|log| log.borrow_mut().push(line));
    }

    /// Writes `< label` into the log when it is dropped.
    #[must_use = "the exit line is written when this guard is dropped; bind it to a named variable"]
    pub struct Guard {
        label: &'static str,
    }

    /// Writes `> label` into this thread's log. The returned guard writes
    /// `< label` when it is dropped, however the function ends.
    pub fn enter(label: &'static str) -> Guard {
        record(format!("> {label}"));
        Guard { label }
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            record(format!("< {}", self.label));
        }
    }

    /// Returns this thread's log, oldest line first, and empties it.
    pub fn take() -> Vec<String> {
        LOG.with(|log| std::mem::take(&mut *log.borrow_mut()))
    }
}

/// Lesson 08. Why `build()` could not make a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildError {
    type_name: &'static str,
    missing: Vec<&'static str>,
}

impl BuildError {
    /// `missing` lists the required fields that were never set, in
    /// declaration order.
    pub fn new(type_name: &'static str, missing: Vec<&'static str>) -> BuildError {
        BuildError { type_name, missing }
    }

    /// The name of the type that was being built.
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// The required fields that were never set, in declaration order.
    pub fn missing(&self) -> &[&'static str] {
        &self.missing
    }
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cannot build `{}`: missing ", self.type_name)?;
        for (index, name) in self.missing.iter().enumerate() {
            let separator = if index == 0 { "" } else { ", " };
            write!(f, "{separator}`{name}`")?;
        }
        Ok(())
    }
}

impl std::error::Error for BuildError {}

/// The teaching text, compiled as documentation tests: `cargo test --doc`
/// builds every `rust` block of `CONCEPTS.md` that needs only the standard
/// library. The blocks that use `syn` and `quote` are compiled and tested in
/// `macros/src/worked.rs`.
#[cfg(doctest)]
#[doc = include_str!("../CONCEPTS.md")]
pub struct ConceptsDoctests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn call_log_records_entry_and_exit() {
        {
            let _outer = calls::enter("outer");
            let _inner = calls::enter("inner");
        }
        assert_eq!(calls::take(), ["> outer", "> inner", "< inner", "< outer"]);
        assert!(calls::take().is_empty(), "take() must empty the log");
    }

    #[test]
    fn build_error_lists_the_missing_fields() {
        let error = BuildError::new("Job", vec!["id", "name"]);
        assert_eq!(
            error.to_string(),
            "cannot build `Job`: missing `id`, `name`"
        );
        assert_eq!(error.type_name(), "Job");
        assert_eq!(error.missing(), ["id", "name"]);
    }

    #[test]
    #[should_panic(expected = "PREDICT")]
    fn guess_panics_until_replaced() {
        let _: u8 = guess();
    }
}
