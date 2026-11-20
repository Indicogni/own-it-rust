//! DRILL M2-09 · Mutating through the only Rc
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m2`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: who else could reach the String while the `&mut String` is in use?

use std::rc::Rc;

fn main() {
    let mut config = Rc::new(String::from("v1"));
    let observer = Rc::downgrade(&config);
    Rc::get_mut(&mut config).unwrap().push_str("-patched");
    assert_eq!(*config, "v1-patched"); // KEEP
    assert!(observer.upgrade().is_some_and(|seen| *seen == "v1-patched")); // KEEP
}
