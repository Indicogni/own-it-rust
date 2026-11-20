//! DRILL M1-09 · A destructor that may read a dead borrow
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0597
//!
//! Run `cargo drill m1`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: in which order are `report` and `title` dropped, and what does `Drop for Report` read?

struct Report<'a> {
    title: &'a str,
}

impl Drop for Report<'_> {
    fn drop(&mut self) {
        println!("closing {}", self.title);
    }
}

// FIX: locals are dropped in the reverse order of their declaration, so
// `title` died before `report`, and `Drop for Report` reads the title. The
// drop checker demands that everything a type with a `Drop` impl borrows is
// still alive when the value is dropped. Declare `title` first, so that it
// is dropped last. (Without the `Drop` impl the first order is accepted:
// then nothing can read the dead reference.)
fn main() {
    let title = String::from("quarterly");
    let report;
    report = Report { title: &title }; // KEEP
    assert_eq!(report.title, "quarterly"); // KEEP
}
