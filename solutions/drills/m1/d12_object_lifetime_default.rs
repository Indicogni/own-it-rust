//! DRILL M1-12 · The lifetime nobody wrote
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0597
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

// FIX: in a struct field, `Box<dyn Greeter>` means
// `Box<dyn Greeter + 'static>`: the boxed value may hold no borrow of a
// local. Give the desk a lifetime parameter and write it on the trait object.
struct Desk<'a> {
    greeter: Box<dyn Greeter + 'a>,
}

fn main() {
    let name = String::from("Ada");
    let desk = Desk { greeter: Box::new(Hello(&name)) }; // KEEP
    assert_eq!(desk.greeter.greet(), "Hello, Ada"); // KEEP
}
