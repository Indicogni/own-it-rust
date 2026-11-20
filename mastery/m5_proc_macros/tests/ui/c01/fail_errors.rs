//! Code review lesson: what the user of `#[derive(Reset)]` must see for wrong
//! input.
//! This program must NOT compile. The expected compiler output is in
//! `fail_errors.stderr`.

use m5_proc_macros::Reset;

#[derive(Reset)]
pub enum Level {
    Low,
    High,
}

#[derive(Reset)]
struct Counter {
    #[reset(kep)]
    count: u32,
    #[reset(keep, skip)]
    label: String,
}

fn main() {}
