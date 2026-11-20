//! Lesson 02: what the user sees when a field cannot be compared.
//! This program must NOT compile. The expected compiler output is in
//! `fail_no_partial_eq.stderr`: the error names the field's type and the
//! missing trait. The macro did nothing special for that; the compiler
//! checks the generated code like any other code.

use m5_proc_macros::Changes;

struct Opaque;

#[derive(Changes)]
struct Job {
    id: u32,
    handle: Opaque,
}

fn main() {}
