//! DRILL M1-09 · A destructor that may read a dead borrow
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
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

fn main() {
    let report;
    let title = String::from("quarterly");
    report = Report { title: &title }; // KEEP
    assert_eq!(report.title, "quarterly"); // KEEP
}
