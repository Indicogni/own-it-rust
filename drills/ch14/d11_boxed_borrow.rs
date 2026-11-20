//! DRILL 14-11 · A boxed closure that borrows a parameter
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: ?
//!
//! Run `cargo drill ch14`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: how long does a `Box<dyn Trait>` promise to stay valid when no lifetime is written?

fn starts_with(prefix: &str) -> Box<dyn Fn(&str) -> bool> {
    Box::new(move |text| text.starts_with(prefix))
}

fn main() {
    let prefix = String::from("api/");
    let test = starts_with(&prefix);
    assert!(test("api/users") && !test("web/users")); // KEEP
}
