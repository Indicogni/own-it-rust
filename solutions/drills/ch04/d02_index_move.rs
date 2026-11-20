//! DRILL 04-02 · Moving out of an index
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0507
//!
//! Run `cargo drill ch04`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what would be left in the vector's slot?

// FIX: a vector slot must never be left empty, so `names[0]` cannot be moved
// out. The KEEP lines only read the name and need both elements to stay, so
// borrow the element.
fn main() {
    let names = vec![String::from("ada"), String::from("grace")];
    let first = &names[0];
    assert_eq!(first, "ada"); // KEEP
    assert_eq!(names.len(), 2); // KEEP
}
