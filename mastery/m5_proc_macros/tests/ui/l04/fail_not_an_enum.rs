//! Lesson 04, task B: `Label` on something that is not an enum.
//! This program must NOT compile. The expected compiler output is in
//! `fail_not_an_enum.stderr`: each error sits under the keyword.

use m5_proc_macros::Label;

#[derive(Label)]
pub struct Job {
    id: u32,
}

#[derive(Label)]
union Bits {
    int: u32,
    float: f32,
}

fn main() {}
