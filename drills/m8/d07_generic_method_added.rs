//! DRILL M8-07 · A convenient method for a trait that is used as `dyn`
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m8`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which callers can use a method with a type parameter: those with a concrete type, those with `dyn Sink`, or both?

/// Stands for version 1.1.0 of a library crate that you maintain.
mod log {
    pub trait Sink {
        fn write(&mut self, line: &str);

        /// New in 1.1.0. It has a body, so no implementation had to change.
        fn write_all<I: IntoIterator<Item = String>>(&mut self, lines: I) {
            for line in lines {
                self.write(&line);
            }
        }
    }

    pub struct Memory(pub Vec<String>);

    impl Sink for Memory {
        fn write(&mut self, line: &str) {
            self.0.push(line.to_string());
        }
    }
}

use log::{Memory, Sink};

fn main() {
    // A user's code, written for 1.0.0: the sink is chosen at run time.
    let mut sink: Box<dyn Sink> = Box::new(Memory(Vec::new())); // KEEP
    sink.write("started"); // KEEP

    // A user of the new method.
    let mut memory = Memory(Vec::new()); // KEEP
    memory.write_all(vec![String::from("a"), String::from("b")]); // KEEP
    assert_eq!(memory.0, ["a", "b"]); // KEEP
}
