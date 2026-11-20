//! DRILL M7-01 · One buffer for every line
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0597
//!
//! Run `cargo drill m7`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what do the `&str` values in `fields` point into, and when is that value destroyed?

// FIX: the buffer holds `&str` borrows, so the text they point into must live
// at least as long as the buffer. `upper` was destroyed at the end of every
// loop round while `fields` still pointed into it (E0597). Make all the owned
// strings first, then borrow from them.
fn main() {
    let lines = ["a,b", "c,d,e"];
    // One buffer for all lines.
    let mut fields: Vec<&str> = Vec::new(); // KEEP
    let mut longest = String::new();
    // The buffer holds borrows, so the text it borrows from must live at
    // least as long as the buffer: make all the owned strings first.
    let upper: Vec<String> = lines.iter().map(|line| line.to_uppercase()).collect();
    for line in &upper {
        fields.clear(); // KEEP
        fields.extend(line.split(','));
        if fields.len() > 2 {
            longest = fields.join("+");
        }
    }
    assert_eq!(longest, "C+D+E"); // KEEP
}
