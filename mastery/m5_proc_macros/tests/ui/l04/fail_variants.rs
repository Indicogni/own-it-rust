//! Lesson 04, tasks A and B: every problem of an enum in one compiler run.
//! This program must NOT compile. The expected compiler output is in
//! `fail_variants.stderr`: six errors, each under the token that is wrong.

use m5_proc_macros::Label;

#[derive(Label)]
enum Level {
    Low(u8),
    #[label("same")]
    Middle,
    #[label(7)]
    High,
    #[label("same")]
    Top,
    #[label("a")]
    #[label("b")]
    Peak {
        height: u32,
    },
    Middle2,
    #[label("Middle2")]
    Last,
}

fn main() {}
