//! DRILL M1-12 · The lifetime nobody wrote
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m1`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: `Box<dyn Greeter>` has no lifetime in it. Which one does the compiler fill in?

trait Greeter {
    fn greet(&self) -> String;
}

struct Hello<'a>(&'a str);

impl Greeter for Hello<'_> {
    fn greet(&self) -> String {
        format!("Hello, {}", self.0)
    }
}

struct Desk {
    greeter: Box<dyn Greeter>,
}

fn main() {
    let name = String::from("Ada");
    let desk = Desk { greeter: Box::new(Hello(&name)) }; // KEEP
    assert_eq!(desk.greeter.greet(), "Hello, Ada"); // KEEP
}
