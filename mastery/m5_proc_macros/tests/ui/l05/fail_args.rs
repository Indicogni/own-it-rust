//! Lesson 05: wrong arguments give one error each, and the functions stay.
//! This program must NOT compile. The expected compiler output is in
//! `fail_args.stderr`: three errors from the macro, and then one from the
//! compiler about `main`. That last error is the point. The compiler can find
//! the mistake in `main` only because `first` still exists. A macro that
//! returns its `compile_error!` without the function hides it until the
//! argument is fixed.

use m5_proc_macros::logged;

#[logged(level = 3)]
fn first() -> u32 {
    1
}

#[logged(label = "a", label = "b")]
fn second() -> u32 {
    2
}

#[logged(label = 7)]
fn third() -> u32 {
    3
}

fn main() {
    let total: u32 = first() + second() + third();
    let _text: String = total;
}
