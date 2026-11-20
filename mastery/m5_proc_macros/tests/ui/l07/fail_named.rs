//! Lesson 07, task B: the errors of `#[derive(Named)]`.
//! This program must NOT compile. The expected compiler output is in
//! `fail_named.stderr`.

use m5_proc_macros::Named;

#[derive(Named)]
#[named(krate = ::kit, crate = ::a)]
#[named(crate = ::b)]
struct Job;

fn main() {}
