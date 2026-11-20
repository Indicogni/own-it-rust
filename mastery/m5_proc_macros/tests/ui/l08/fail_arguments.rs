//! Lesson 08: wrong `#[builder(...)]` arguments, all reported in one run.
//! This program must NOT compile. The expected compiler output is in
//! `fail_arguments.stderr`.

use m5_proc_macros::Builder;

#[derive(Builder)]
struct Job {
    #[builder(defualt, into)]
    id: u32,
    #[builder(default = 1, each = "x", default)]
    #[builder(into)]
    #[builder(into)]
    retries: u8,
}

fn main() {}
