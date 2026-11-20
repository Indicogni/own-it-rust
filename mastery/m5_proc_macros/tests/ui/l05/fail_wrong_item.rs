//! Lesson 05: `#[logged]` on items that it cannot change.
//! This program must NOT compile. The expected compiler output is in
//! `fail_wrong_item.stderr`: one error per item from the macro, and one from
//! the compiler about `main`, which it can check because both items survive.

use m5_proc_macros::logged;

#[logged]
struct Job {
    id: u32,
}

#[logged]
const fn limit() -> u32 {
    100
}

fn main() {
    let job = Job { id: limit() };
    let _text: String = job.id;
}
