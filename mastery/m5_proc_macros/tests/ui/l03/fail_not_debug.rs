//! Lesson 03, task C: a shown field needs `Debug`.
//! This program must NOT compile. The expected compiler output is in
//! `fail_not_debug.stderr`. Look at where the compiler points: the bound in
//! the generated where clause is built from the field's own type tokens.

use m5_proc_macros::Redacted;

struct Opaque;

#[derive(Redacted)]
struct Job {
    id: u32,
    handle: Opaque,
}

fn main() {}
