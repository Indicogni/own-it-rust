//! DRILL M1-17 · A blanket impl and one more
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0119
//!
//! Run `cargo drill m1`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: does `bool` implement `Display`?

trait Describe {
    fn describe(&self) -> String;
}

// FIX: `impl<T: Display> Describe for T` already covers `bool`, because
// `bool` implements `Display`. Two impls for one type are never allowed, and
// stable Rust has no way to say "this one is more specific". A blanket impl
// claims every type that meets its bound, now and in the future, so give it
// up and write the impls that are needed.
impl Describe for i32 {
    fn describe(&self) -> String {
        format!("<{self}>")
    }
}

impl Describe for bool {
    fn describe(&self) -> String {
        if *self { "yes" } else { "no" }.to_string()
    }
}

fn main() {
    assert_eq!(7.describe(), "<7>"); // KEEP
    assert_eq!(true.describe(), "yes"); // KEEP
}
