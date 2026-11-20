//! DRILL 05-13 · Find it, or add it and return the new one
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0499
//!
//! Run `cargo drill ch05`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: `found` is returned on one path only. For how long does the compiler
//! treat `names` as borrowed on the other path?

fn slot_for<'a>(names: &'a mut Vec<String>, name: &str) -> &'a mut String {
    // FIX: search for an INDEX first. A `usize` borrows nothing, so the search
    // borrow is over before either branch borrows `names` again. (For a map,
    // `entry(key).or_insert(..)` does the whole find-or-add in one borrow.)
    match names.iter().position(|n| n == name) {
        Some(index) => &mut names[index],
        None => {
            names.push(name.to_string());
            names.last_mut().unwrap()
        }
    }
}

fn main() {
    let mut names = vec![String::from("ada")];
    slot_for(&mut names, "ada").push('!'); // KEEP
    slot_for(&mut names, "lin").push('?'); // KEEP
    assert_eq!(names, ["ada!", "lin?"]); // KEEP
}
