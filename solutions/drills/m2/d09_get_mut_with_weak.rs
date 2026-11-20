//! DRILL M2-09 · Mutating through the only Rc
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: panic
//!
//! Run `cargo drill m2`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: who else could reach the String while the `&mut String` is in use?

use std::rc::Rc;

// FIX: `Rc::get_mut` returns `None` while any other pointer to the allocation
// exists, and a `Weak` counts: it could be upgraded to a second `Rc` while the
// `&mut String` is alive. Change the value first, then hand out the observer.
// (If observers must see later changes too, the value needs a `RefCell`.)
fn main() {
    let mut config = Rc::new(String::from("v1"));
    Rc::get_mut(&mut config).unwrap().push_str("-patched");
    let observer = Rc::downgrade(&config);
    assert_eq!(*config, "v1-patched"); // KEEP
    assert!(observer.upgrade().is_some_and(|seen| *seen == "v1-patched")); // KEEP
}
