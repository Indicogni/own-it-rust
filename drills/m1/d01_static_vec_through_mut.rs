//! DRILL M1-01 · A longer-lived element type behind `&mut`
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m1`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which element type does the vector have, and may `&mut` change it?

fn add_note<'a>(notes: &mut Vec<&'a str>, note: &'a str) {
    notes.push(note);
}

fn main() {
    let mut notes: Vec<&'static str> = vec!["built in"];
    let typed = String::from("typed by the user");
    add_note(&mut notes, &typed); // KEEP
    assert_eq!(notes, ["built in", "typed by the user"]); // KEEP
}
