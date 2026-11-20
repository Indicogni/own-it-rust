//! DRILL M8-01 · A trait gets a new method
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m8`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: who has to write code when a published trait gains a method without a body?

/// Stands for version 1.1.0 of a library crate that you maintain.
mod host {
    pub trait Plugin {
        fn name(&self) -> String;

        /// New in 1.1.0. Most plugins should run at priority 5.
        fn priority(&self) -> u8;
    }

    pub fn describe(plugin: &dyn Plugin) -> String {
        format!("{} (priority {})", plugin.name(), plugin.priority())
    }
}

/// A user's plugin, written for 1.0.0, when `Plugin` had one method.
struct Spellcheck;

impl host::Plugin for Spellcheck { // KEEP
    fn name(&self) -> String { // KEEP
        String::from("spellcheck") // KEEP
    } // KEEP
} // KEEP

fn main() {
    assert_eq!(host::describe(&Spellcheck), "spellcheck (priority 5)"); // KEEP
}
