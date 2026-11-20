//! DRILL M7-01 · One buffer for every line
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m7`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what do the `&str` values in `fields` point into, and when is that value destroyed?

fn main() {
    let lines = ["a,b", "c,d,e"];
    // One buffer for all lines.
    let mut fields: Vec<&str> = Vec::new(); // KEEP
    let mut longest = String::new();
    for line in lines {
        let upper = line.to_uppercase();
        fields.clear(); // KEEP
        fields.extend(upper.split(','));
        if fields.len() > 2 {
            longest = fields.join("+");
        }
    }
    assert_eq!(longest, "C+D+E"); // KEEP
}
