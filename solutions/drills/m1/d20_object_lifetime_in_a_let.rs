//! DRILL M1-20 · The lifetime nobody wrote, inside a function
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ok
//!
//! Run `cargo drill m1`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: drill d12 wrote this type in a struct field. Here it is the type of a `let`. Is the rule the same?

trait Greeter {
    fn greet(&self) -> String;
}

struct Hello<'a>(&'a str);

impl Greeter for Hello<'_> {
    fn greet(&self) -> String {
        format!("Hello, {}", self.0)
    }
}

// NOTHING TO FIX: this compiles and runs. The default `'static` for
// `Box<dyn Greeter>` applies where the type is written in an item: a struct
// field (drill d12), a function signature, a type alias. In the type of a
// `let` inside a function body, the missing lifetime is inferred like every
// other lifetime there, so the box may hold a borrow of `name`. Move the same
// type into a field or a return type and the E0597 of drill d12 is back.
fn main() {
    let name = String::from("Ada");
    let greeter: Box<dyn Greeter> = Box::new(Hello(&name)); // KEEP
    let desk: Vec<Box<dyn Greeter>> = vec![greeter]; // KEEP
    assert_eq!(desk[0].greet(), "Hello, Ada"); // KEEP
}
